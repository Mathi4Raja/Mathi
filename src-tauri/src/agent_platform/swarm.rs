use crate::agent_platform::contract::{AgentExecution, AgentPlatformError, ProviderId};
use crate::agent_platform::scheduler::{ExecutionPlan, Scheduler};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwarmTemplate {
    CodeFix,
    Refactor,
    Research,
}

impl SwarmTemplate {
    pub fn team_size_range(&self) -> (usize, usize) {
        match self {
            SwarmTemplate::CodeFix => (2, 4),
            SwarmTemplate::Refactor => (3, 6),
            SwarmTemplate::Research => (2, 3),
        }
    }

    pub fn task_sequence(&self) -> &'static [&'static str] {
        match self {
            SwarmTemplate::CodeFix => &["diagnose", "patch", "verify"],
            SwarmTemplate::Refactor => &["analyze", "refactor", "validate", "summarize"],
            SwarmTemplate::Research => &["collect", "compare", "synthesize"],
        }
    }
}

#[derive(Debug, Clone)]
pub struct SwarmExecutionSummary {
    pub template: SwarmTemplate,
    pub executions: Vec<AgentExecution>,
}

impl SwarmExecutionSummary {
    pub fn total_duration_ms(&self) -> u64 {
        self.executions.iter().map(|exec| exec.duration_ms).sum()
    }
}

pub struct SwarmCoordinator {
    scheduler: Scheduler,
}

impl SwarmCoordinator {
    pub fn new(scheduler: Scheduler) -> Self {
        Self { scheduler }
    }

    pub async fn execute_template(
        &self,
        template: SwarmTemplate,
        payload: serde_json::Value,
        provider_priority: Vec<ProviderId>,
    ) -> Result<SwarmExecutionSummary, AgentPlatformError> {
        let mut executions = Vec::new();

        for task in template.task_sequence() {
            let mut task_payload = payload.clone();
            if let Some(object) = task_payload.as_object_mut() {
                object.insert("swarm_task".to_string(), serde_json::Value::String((*task).to_string()));
                object.insert("template".to_string(), serde_json::Value::String(format!("{:?}", template)));
            }
            let plan = ExecutionPlan::new(*task, task_payload, provider_priority.clone());
            let execution = self.scheduler.execute_with_fallback(&plan).await?;
            executions.push(execution);
        }

        Ok(SwarmExecutionSummary {
            template,
            executions,
        })
    }
}
