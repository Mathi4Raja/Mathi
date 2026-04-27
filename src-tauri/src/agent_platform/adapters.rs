use std::time::Instant;

use async_trait::async_trait;

use crate::agent_platform::contract::{
    AdapterKind, AgentExecution, AgentPlatformError, ProviderId, UnifiedAgentAdapter,
};

#[derive(Debug, Clone)]
pub struct AcpCliAdapter {
    provider: ProviderId,
}

impl AcpCliAdapter {
    pub fn new(provider: impl Into<String>) -> Self {
        Self {
            provider: ProviderId(provider.into()),
        }
    }
}

#[async_trait]
impl UnifiedAgentAdapter for AcpCliAdapter {
    fn provider_id(&self) -> ProviderId {
        self.provider.clone()
    }

    fn kind(&self) -> AdapterKind {
        AdapterKind::AcpCli
    }

    async fn initialize(&self) -> Result<(), AgentPlatformError> {
        Ok(())
    }

    async fn execute(
        &self,
        task_type: &str,
        payload: &serde_json::Value,
    ) -> Result<AgentExecution, AgentPlatformError> {
        let started = Instant::now();
        let output = format!("acp:{} handled {}", self.provider.0, task_type);
        Ok(AgentExecution {
            provider: self.provider_id(),
            kind: self.kind(),
            task_type: task_type.to_string(),
            output,
            duration_ms: started.elapsed().as_millis() as u64,
            metadata: serde_json::json!({
                "route": "acp-cli",
                "payload_keys": payload.as_object().map(|o| o.len()).unwrap_or(0),
            }),
        })
    }

    async fn health_check(&self) -> bool {
        true
    }

    async fn shutdown(&self) -> Result<(), AgentPlatformError> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ApiNativeAdapter {
    provider: ProviderId,
}

impl ApiNativeAdapter {
    pub fn new(provider: impl Into<String>) -> Self {
        Self {
            provider: ProviderId(provider.into()),
        }
    }
}

#[async_trait]
impl UnifiedAgentAdapter for ApiNativeAdapter {
    fn provider_id(&self) -> ProviderId {
        self.provider.clone()
    }

    fn kind(&self) -> AdapterKind {
        AdapterKind::ApiNative
    }

    async fn initialize(&self) -> Result<(), AgentPlatformError> {
        Ok(())
    }

    async fn execute(
        &self,
        task_type: &str,
        payload: &serde_json::Value,
    ) -> Result<AgentExecution, AgentPlatformError> {
        let started = Instant::now();
        let output = format!("api:{} handled {}", self.provider.0, task_type);
        Ok(AgentExecution {
            provider: self.provider_id(),
            kind: self.kind(),
            task_type: task_type.to_string(),
            output,
            duration_ms: started.elapsed().as_millis() as u64,
            metadata: serde_json::json!({
                "route": "api-native",
                "payload_size": payload.to_string().len(),
            }),
        })
    }

    async fn health_check(&self) -> bool {
        true
    }

    async fn shutdown(&self) -> Result<(), AgentPlatformError> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct MockFailAdapter {
    provider: ProviderId,
}

impl MockFailAdapter {
    pub fn new(provider: impl Into<String>) -> Self {
        Self {
            provider: ProviderId(provider.into()),
        }
    }
}

#[async_trait]
impl UnifiedAgentAdapter for MockFailAdapter {
    fn provider_id(&self) -> ProviderId {
        self.provider.clone()
    }

    fn kind(&self) -> AdapterKind {
        AdapterKind::AcpCli
    }

    async fn initialize(&self) -> Result<(), AgentPlatformError> {
        Err(AgentPlatformError::ProviderUnavailable(self.provider.0.clone()))
    }

    async fn execute(
        &self,
        task_type: &str,
        _payload: &serde_json::Value,
    ) -> Result<AgentExecution, AgentPlatformError> {
        Err(AgentPlatformError::AdapterExecutionFailed(format!(
            "{} failed {}",
            self.provider.0, task_type
        )))
    }

    async fn health_check(&self) -> bool {
        false
    }

    async fn shutdown(&self) -> Result<(), AgentPlatformError> {
        Ok(())
    }
}
