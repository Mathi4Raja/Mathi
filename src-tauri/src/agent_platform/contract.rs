use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ProviderId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdapterKind {
    AcpCli,
    ApiNative,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExecution {
    pub provider: ProviderId,
    pub kind: AdapterKind,
    pub task_type: String,
    pub output: String,
    pub duration_ms: u64,
    pub metadata: serde_json::Value,
}

#[derive(thiserror::Error, Debug)]
pub enum AgentPlatformError {
    #[error("provider unavailable: {0}")]
    ProviderUnavailable(String),
    #[error("adapter execution failed: {0}")]
    AdapterExecutionFailed(String),
    #[error("all providers failed for task: {0}")]
    AllProvidersFailed(String),
    #[error("invalid template configuration: {0}")]
    InvalidTemplate(String),
}

#[async_trait]
pub trait UnifiedAgentAdapter: Send + Sync {
    fn provider_id(&self) -> ProviderId;
    fn kind(&self) -> AdapterKind;

    async fn initialize(&self) -> Result<(), AgentPlatformError>;

    async fn execute(
        &self,
        task_type: &str,
        payload: &serde_json::Value,
    ) -> Result<AgentExecution, AgentPlatformError>;

    async fn health_check(&self) -> bool;

    async fn shutdown(&self) -> Result<(), AgentPlatformError>;
}
