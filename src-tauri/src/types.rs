use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRequest {
    pub id: Uuid,
    pub task_type: String,
    pub payload: serde_json::Value,
    pub deadline_ms: Option<u64>,
    #[serde(default)]
    pub context: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentEvent {
    Ready { worker_id: Uuid },
    Progress { worker_id: Uuid, current: u32, total: u32 },
    StreamChunk { worker_id: Uuid, sequence: u64, content: String, is_final: bool },
    Finished { worker_id: Uuid, output: String, duration_ms: u64 },
    Cancelled { worker_id: Uuid },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    pub worker_id: Uuid,
    pub output: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerCommand {
    pub id: Uuid,
    pub request: AgentRequest,
}

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("channel closed")]
    ChannelClosed,
    #[error("worker queue full")]
    QueueFull,
    #[error("worker cancelled")]
    Cancelled,
    #[error("bootstrap failed: {0}")]
    BootstrapFailed(String),
    #[error("approval required: {0}")]
    ApprovalRequired(String),
    #[error("policy denied: {0}")]
    PolicyDenied(String),
    #[error("crypto failure: {0}")]
    CryptoFailure(String),
    #[error("record not found: {0}")]
    NotFound(String),
}
