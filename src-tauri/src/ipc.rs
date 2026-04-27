use crate::types::{AgentEvent, AgentRequest, RuntimeError, WorkerCommand};
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Debug)]
pub struct IpcBridge {
    command_tx: mpsc::Sender<WorkerCommand>,
    event_rx: mpsc::Receiver<AgentEvent>,
}

impl IpcBridge {
    pub fn new(buffer: usize) -> (Self, mpsc::Receiver<WorkerCommand>, mpsc::Sender<AgentEvent>) {
        let (command_tx, command_rx) = mpsc::channel(buffer);
        let (event_tx, event_rx) = mpsc::channel(buffer);
        (Self { command_tx, event_rx }, command_rx, event_tx)
    }

    pub async fn dispatch(&self, request: AgentRequest) -> Result<Uuid, RuntimeError> {
        let command = WorkerCommand { id: Uuid::new_v4(), request };
        self.command_tx.send(command.clone()).await.map_err(|_| RuntimeError::ChannelClosed)?;
        Ok(command.id)
    }

    pub fn into_parts(self) -> (mpsc::Sender<WorkerCommand>, mpsc::Receiver<AgentEvent>) {
        (self.command_tx, self.event_rx)
    }
}
