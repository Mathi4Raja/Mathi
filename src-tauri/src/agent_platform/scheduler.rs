use std::collections::BTreeMap;
use std::sync::Arc;

use crate::agent_platform::contract::{
    AgentExecution, AgentPlatformError, ProviderId, UnifiedAgentAdapter,
};

#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    pub task_type: String,
    pub payload: serde_json::Value,
    pub provider_priority: Vec<ProviderId>,
}

impl ExecutionPlan {
    pub fn new(task_type: impl Into<String>, payload: serde_json::Value, provider_priority: Vec<ProviderId>) -> Self {
        Self {
            task_type: task_type.into(),
            payload,
            provider_priority,
        }
    }
}

pub struct Scheduler {
    adapters: BTreeMap<ProviderId, Arc<dyn UnifiedAgentAdapter>>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            adapters: BTreeMap::new(),
        }
    }

    pub fn register(&mut self, adapter: Arc<dyn UnifiedAgentAdapter>) {
        self.adapters.insert(adapter.provider_id(), adapter);
    }

    pub fn registered_count(&self) -> usize {
        self.adapters.len()
    }

    pub async fn execute_with_fallback(
        &self,
        plan: &ExecutionPlan,
    ) -> Result<AgentExecution, AgentPlatformError> {
        let mut last_error: Option<AgentPlatformError> = None;

        for provider in &plan.provider_priority {
            let Some(adapter) = self.adapters.get(provider) else {
                last_error = Some(AgentPlatformError::ProviderUnavailable(provider.0.clone()));
                continue;
            };

            if adapter.initialize().await.is_err() {
                last_error = Some(AgentPlatformError::ProviderUnavailable(provider.0.clone()));
                continue;
            }

            match adapter.execute(&plan.task_type, &plan.payload).await {
                Ok(result) => {
                    let _ = adapter.shutdown().await;
                    return Ok(result);
                }
                Err(error) => {
                    last_error = Some(error);
                    let _ = adapter.shutdown().await;
                }
            }
        }

        Err(last_error.unwrap_or_else(|| AgentPlatformError::AllProvidersFailed(plan.task_type.clone())))
    }
}
