use crate::policy::{default_decision, ActionClass};
use crate::types::{AgentEvent, AgentRequest, RuntimeError, WorkerCommand};
use tokio::sync::{mpsc, oneshot};
use tokio::time::{sleep, Duration};
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
    let _decision = default_decision(ActionClass::Read);
    let started = std::time::Instant::now();
    let total = 3;
    for current in 1..=total {
        sleep(Duration::from_millis(10)).await;
        let _ = event_tx
            .send(AgentEvent::Progress {
                worker_id,
                current,
                total,
            })
            .await;
    }
    let output = format!("task {} handled: {}", request.id, request.task_type);
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
