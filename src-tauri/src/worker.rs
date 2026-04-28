use std::path::PathBuf;
use std::process::Command;

use crate::policy::{parse_action_class, ActionClass};
use crate::types::{AgentEvent, AgentRequest, RuntimeError, WorkerCommand};
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, info};
use uuid::Uuid;

#[derive(Debug)]
pub struct WorkerHandle {
    pub id: Uuid,
    cancel_tx: oneshot::Sender<()>,
}

impl WorkerHandle {
    pub fn cancel(self) -> Result<(), RuntimeError> {
        self.cancel_tx.send(()).map_err(|_| RuntimeError::Cancelled)
    }
}

pub fn spawn_worker(
    worker_id: Uuid,
    mut command_rx: mpsc::Receiver<WorkerCommand>,
    event_tx: mpsc::Sender<AgentEvent>,
) -> WorkerHandle {
    let (cancel_tx, mut cancel_rx) = oneshot::channel();

    tokio::spawn(async move {
        info!(%worker_id, "worker started");
        let _ = event_tx.send(AgentEvent::Ready { worker_id }).await;
        loop {
            tokio::select! {
                _ = &mut cancel_rx => {
                    let _ = event_tx.send(AgentEvent::Cancelled { worker_id }).await;
                    break;
                }
                maybe_command = command_rx.recv() => {
                    let Some(command) = maybe_command else {
                        break;
                    };
                    if let Err(error) = run_command(worker_id, command.request, event_tx.clone()).await {
                        debug!(%worker_id, ?error, "worker command failed");
                    }
                }
            }
        }
        info!(%worker_id, "worker stopped");
    });

    WorkerHandle { id: worker_id, cancel_tx }
}

async fn run_command(
    worker_id: Uuid,
    request: AgentRequest,
    event_tx: mpsc::Sender<AgentEvent>,
) -> Result<(), RuntimeError> {
    let started = std::time::Instant::now();
    let total = 2;
    let _ = event_tx
        .send(AgentEvent::Progress {
            worker_id,
            current: 1,
            total,
        })
        .await;

    let output = execute_request(&request).await?;

    let _ = event_tx
        .send(AgentEvent::Progress {
            worker_id,
            current: 2,
            total,
        })
        .await;
    let _ = event_tx
        .send(AgentEvent::StreamChunk {
            worker_id,
            sequence: 0,
            content: output.clone(),
            is_final: true,
        })
        .await;
    let _ = event_tx
        .send(AgentEvent::Finished {
            worker_id,
            output,
            duration_ms: started.elapsed().as_millis() as u64,
        })
        .await;
    Ok(())
}

    async fn execute_request(request: &AgentRequest) -> Result<String, RuntimeError> {
        let action = request
            .context
            .get("action_class")
            .and_then(|value| value.as_str())
            .and_then(parse_action_class)
            .unwrap_or(ActionClass::Read);

        match action {
            ActionClass::Shell => execute_shell_request(request).await,
            _ => Ok(format!("task {} handled: {}", request.id, request.task_type)),
        }
    }

    async fn execute_shell_request(request: &AgentRequest) -> Result<String, RuntimeError> {
        let command = request
            .context
            .get("command")
            .and_then(|value| value.as_str())
            .ok_or_else(|| RuntimeError::PolicyDenied("shell command required".to_string()))?
            .to_string();
        let command_for_fallback = command.clone();

        let workspace_path = request
            .context
            .get("workspace_path")
            .and_then(|value| value.as_str())
            .map(PathBuf::from);

        let output = tokio::task::spawn_blocking(move || {
            let parsed = shell_words::split(&command)
                .map_err(|error| RuntimeError::PolicyDenied(format!("invalid shell command: {error}")))?;
            let Some((executable, args)) = parsed.split_first() else {
                return Err(RuntimeError::PolicyDenied("shell command required".to_string()));
            };

            let mut child = Command::new(executable);
            child.args(args);
            if let Some(path) = workspace_path {
                if !path.exists() {
                    return Err(RuntimeError::PolicyDenied(format!(
                        "workspace path does not exist: {}",
                        path.display()
                    )));
                }
                child.current_dir(path);
            }

            child
                .output()
                .map_err(|error| RuntimeError::BootstrapFailed(error.to_string()))
        })
        .await
        .map_err(|error| RuntimeError::BootstrapFailed(error.to_string()))??;

        let mut rendered = String::new();
        if !output.stdout.is_empty() {
            rendered.push_str(&String::from_utf8_lossy(&output.stdout));
        }
        if !output.stderr.is_empty() {
            if !rendered.is_empty() {
                rendered.push('\n');
            }
            rendered.push_str(&String::from_utf8_lossy(&output.stderr));
        }

        if !output.status.success() {
            return Err(RuntimeError::BootstrapFailed(format!(
                "shell command exited with {}",
                output.status
            )));
        }

        let rendered = rendered.trim();
        if rendered.is_empty() {
            Ok(format!("shell command completed: {command_for_fallback}"))
        } else {
            Ok(rendered.to_string())
        }
    }
