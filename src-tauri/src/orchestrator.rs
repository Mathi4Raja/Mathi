use crate::db::RuntimeDatabase;
use crate::ipc::IpcBridge;
use crate::policy::{ActionClass, PolicyEngine, PolicyOutcome};
use crate::runtime_context::CapabilityContext;
use crate::telemetry::measure;
use crate::types::{AgentEvent, AgentRequest, RuntimeError, WorkerCommand};
use crate::worker::{spawn_worker, WorkerHandle};
use tokio::sync::mpsc;
use tracing::info;
use uuid::Uuid;
use std::sync::Mutex;

#[derive(Debug)]
pub struct Orchestrator {
    context: CapabilityContext,
    database: RuntimeDatabase,
    worker_count: usize,
    channel_buffer: usize,
    workers: Mutex<Vec<WorkerHandle>>,
    worker_command_txs: Mutex<Vec<mpsc::Sender<WorkerCommand>>>,
    policy_engine: PolicyEngine,
}

impl Orchestrator {
    pub fn new(worker_count: usize, channel_buffer: usize) -> Self {
        Self {
            context: CapabilityContext::new(".", worker_count, channel_buffer).with_balanced_guardrails(),
            database: RuntimeDatabase::new_in_memory().expect("runtime database initialization"),
            worker_count,
            channel_buffer,
            workers: Mutex::new(Vec::new()),
            worker_command_txs: Mutex::new(Vec::new()),
            policy_engine: PolicyEngine::default(),
        }
    }

    pub async fn bootstrap(&self) -> Result<(), RuntimeError> {
        let (_, sample) = measure("orchestrator_bootstrap", || ());
        info!(elapsed_ms = sample.duration.as_millis(), "bootstrapping orchestrator");
        self.database.record_sample("orchestrator_bootstrap", sample.duration.as_millis() as u64)?;
        self.database.save_session_state("workspace_root", &self.context.workspace_root.display().to_string())?;
        self.database.save_session_state("worker_count", &self.context.worker_count.to_string())?;
        self.database.save_session_state("channel_buffer", &self.context.channel_buffer.to_string())?;
        self.policy_engine.approvals().grant_session(ActionClass::Tool, "default-agent");

        let (bridge, command_rx, event_tx) = IpcBridge::new(self.channel_buffer);
        let (command_tx, event_rx) = bridge.into_parts();
        let _ = command_tx;

        let mut workers: Vec<WorkerHandle> = Vec::with_capacity(self.worker_count);
        let mut worker_command_txs: Vec<mpsc::Sender<WorkerCommand>> = Vec::with_capacity(self.worker_count);
        let mut worker_event_rxs: Vec<mpsc::Receiver<AgentEvent>> = Vec::with_capacity(self.worker_count);

        for _ in 0..self.worker_count {
            let worker_id = Uuid::new_v4();
            let (bridge, worker_command_rx, worker_event_tx) = IpcBridge::new(self.channel_buffer);
            let (worker_command_tx, worker_event_rx) = bridge.into_parts();
            let handle = spawn_worker(worker_id, worker_command_rx, worker_event_tx);
            workers.push(handle);
            worker_command_txs.push(worker_command_tx);
            worker_event_rxs.push(worker_event_rx);
        }

        let _ = (command_rx, event_tx, event_rx, worker_event_rxs);
        let worker_total = workers.len();
        *self.workers.lock().expect("workers mutex") = workers;
        *self.worker_command_txs.lock().expect("command tx mutex") = worker_command_txs;
        let telemetry_count = self.database.telemetry_count()?;
        info!(telemetry_count, "telemetry sample persisted");
        info!(workers = worker_total, "orchestrator ready");
        Ok(())
    }

    pub fn worker_count(&self) -> usize {
        self.workers.lock().expect("workers mutex").len()
    }

    pub async fn dispatch(&self, request: AgentRequest) -> Result<Uuid, RuntimeError> {
        match self.policy_engine.evaluate_request(&request, &self.context) {
            PolicyOutcome::Allow => {}
            PolicyOutcome::RequiresApproval(reason) => {
                return Err(RuntimeError::ApprovalRequired(reason));
            }
            PolicyOutcome::Deny(reason) => {
                return Err(RuntimeError::PolicyDenied(reason));
            }
        }

        let sender = {
            let worker_command_txs = self.worker_command_txs.lock().expect("command tx mutex");
            worker_command_txs.first().cloned()
        };

        match sender {
            Some(sender) => {
                let command = WorkerCommand { id: Uuid::new_v4(), request };
                sender.send(command.clone()).await.map_err(|_| RuntimeError::ChannelClosed)?;
                Ok(command.id)
            }
            None => {
                let (bridge, _, _) = IpcBridge::new(self.channel_buffer);
                bridge.dispatch(request).await
            }
        }
    }
}
