use crate::auth::LocalVault;
use crate::db::RuntimeDatabase;
use crate::ipc::IpcBridge;
use crate::memory::{MemoryEntry, MemoryScope, MemoryService};
use crate::policy::{parse_action_class, ActionClass, PolicyEngine, PolicyOutcome};
use crate::runtime_context::CapabilityContext;
use crate::telemetry::measure;
use crate::types::{AgentEvent, AgentRequest, RuntimeError, WorkerCommand};
use crate::worker::{spawn_worker, WorkerHandle};
use serde_json::Value;
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
    vault: LocalVault,
    memory: MemoryService,
}

impl Orchestrator {
    pub fn new(worker_count: usize, channel_buffer: usize) -> Self {
        let database = RuntimeDatabase::new_in_memory().expect("runtime database initialization");
        Self {
            context: CapabilityContext::new(".", worker_count, channel_buffer).with_balanced_guardrails(),
            database: database.clone(),
            worker_count,
            channel_buffer,
            workers: Mutex::new(Vec::new()),
            worker_command_txs: Mutex::new(Vec::new()),
            policy_engine: PolicyEngine::default(),
            vault: LocalVault::with_database(database.clone(), "mathi-local-vault"),
            memory: MemoryService::with_database(database),
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

    pub fn grant_approval(&self, action: ActionClass, agent_id: &str) {
        self.policy_engine.approvals().grant_session(action, agent_id);
    }

    pub fn revoke_approval(&self, action: ActionClass, agent_id: &str) {
        self.policy_engine.approvals().revoke_session(action, agent_id);
    }

    pub fn policy_audit_count(&self) -> Result<u64, RuntimeError> {
        self.database.policy_audit_count()
    }

    pub fn store_provider_secret(&self, provider_key: &str, secret: &str) -> Result<(), RuntimeError> {
        self.vault.store_secret(provider_key, secret)?;
        self.database.record_policy_audit(
            "Credentials",
            "vault",
            "ALLOW",
            &format!("stored secret for {provider_key}"),
        )?;
        Ok(())
    }

    pub fn revoke_provider_secret(&self, provider_key: &str) -> Result<(), RuntimeError> {
        self.vault.revoke_secret(provider_key)?;
        self.database.record_policy_audit(
            "Credentials",
            "vault",
            "ALLOW",
            &format!("revoked secret for {provider_key}"),
        )?;
        Ok(())
    }

    pub fn get_provider_secret_for_agent(
        &self,
        provider_key: &str,
        agent_provider_scope: &str,
    ) -> Result<String, RuntimeError> {
        if provider_key != agent_provider_scope {
            return Err(RuntimeError::PolicyDenied(
                "credential access denied outside agent provider scope".to_string(),
            ));
        }
        self.vault.load_secret(provider_key)
    }

    pub fn put_memory(
        &self,
        scope: MemoryScope,
        memory_key: &str,
        value: &str,
        ttl_seconds: Option<u64>,
    ) -> Result<(), RuntimeError> {
        self.memory.put(scope, memory_key, value, ttl_seconds)?;
        self.database.record_policy_audit(
            "Memory",
            "memory-service",
            "ALLOW",
            &format!("stored {scope:?}/{memory_key}"),
        )?;
        Ok(())
    }

    pub fn get_memory(&self, scope: MemoryScope, memory_key: &str) -> Result<Option<MemoryEntry>, RuntimeError> {
        self.memory.get(scope, memory_key)
    }

    pub fn cleanup_memory_retention(&self) -> Result<u64, RuntimeError> {
        let purged = self.memory.cleanup_expired()?;
        if purged > 0 {
            self.database.record_policy_audit(
                "Memory",
                "retention",
                "ALLOW",
                &format!("purged {purged} expired entries"),
            )?;
        }
        Ok(purged)
    }

    pub async fn dispatch(&self, mut request: AgentRequest) -> Result<Uuid, RuntimeError> {
        let _ = self.cleanup_memory_retention();

        let action = request
            .context
            .get("action_class")
            .and_then(|value| value.as_str())
            .and_then(parse_action_class)
            .unwrap_or(ActionClass::Read);
        let agent_id = request
            .context
            .get("agent_id")
            .and_then(|value| value.as_str())
            .unwrap_or("default-agent")
            .to_string();

        match self.policy_engine.evaluate_request(&request, &self.context) {
            PolicyOutcome::Allow => {
                let _ = self.database.record_policy_audit(
                    &format!("{:?}", action),
                    &agent_id,
                    "ALLOW",
                    "request accepted by policy",
                );
            }
            PolicyOutcome::RequiresApproval(reason) => {
                let _ = self.database.record_policy_audit(
                    &format!("{:?}", action),
                    &agent_id,
                    "REQUIRES_APPROVAL",
                    &reason,
                );
                return Err(RuntimeError::ApprovalRequired(reason));
            }
            PolicyOutcome::Deny(reason) => {
                let _ = self.database.record_policy_audit(
                    &format!("{:?}", action),
                    &agent_id,
                    "DENY",
                    &reason,
                );
                return Err(RuntimeError::PolicyDenied(reason));
            }
        }

        if let Some(provider_key) = request
            .context
            .get("provider_key")
            .and_then(|value| value.as_str())
            .map(ToString::to_string)
        {
            let agent_scope = request
                .context
                .get("agent_provider_scope")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string();

            match self.get_provider_secret_for_agent(&provider_key, &agent_scope) {
                Ok(secret) => {
                    request.context.insert(
                        "provider_secret".to_string(),
                        Value::String(secret),
                    );
                    let _ = self.database.record_policy_audit(
                        "Credentials",
                        &agent_id,
                        "ALLOW",
                        &format!("credential access granted for {provider_key}"),
                    );
                }
                Err(error) => {
                    let reason = error.to_string();
                    let _ = self.database.record_policy_audit(
                        "Credentials",
                        &agent_id,
                        "DENY",
                        &reason,
                    );
                    return Err(error);
                }
            }
        }

        if let Some(memory_scope) = request
            .context
            .get("memory_scope")
            .and_then(|value| value.as_str())
            .and_then(parse_memory_scope)
        {
            let memory_key = request
                .context
                .get("memory_key")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string();

            if memory_key.is_empty() {
                return Err(RuntimeError::PolicyDenied("memory key required for memory scope retrieval".to_string()));
            }

            if !is_memory_scope_allowed(memory_scope, &request) {
                let _ = self.database.record_policy_audit(
                    "Memory",
                    &agent_id,
                    "DENY",
                    &format!("memory scope {:?} not permitted for request", memory_scope),
                );
                return Err(RuntimeError::PolicyDenied(format!(
                    "memory scope {:?} not permitted for request",
                    memory_scope
                )));
            }

            if let Some(entry) = self.memory.get(memory_scope, &memory_key)? {
                request.context.insert(
                    "memory_context".to_string(),
                    Value::String(entry.redacted_value),
                );
                let _ = self.database.record_policy_audit(
                    "Memory",
                    &agent_id,
                    "ALLOW",
                    &format!("memory context injected for {memory_scope:?}/{memory_key}"),
                );
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

fn parse_memory_scope(input: &str) -> Option<MemoryScope> {
    match input.to_lowercase().as_str() {
        "session" => Some(MemoryScope::Session),
        "persistent" => Some(MemoryScope::Persistent),
        "workspace" => Some(MemoryScope::Workspace),
        _ => None,
    }
}

fn is_memory_scope_allowed(scope: MemoryScope, request: &AgentRequest) -> bool {
    match scope {
        MemoryScope::Session => true,
        MemoryScope::Persistent => request
            .context
            .get("allow_persistent_memory")
            .and_then(|value| value.as_bool())
            .unwrap_or(false),
        MemoryScope::Workspace => request
            .context
            .get("allow_workspace_memory")
            .and_then(|value| value.as_bool())
            .unwrap_or(false),
    }
}
