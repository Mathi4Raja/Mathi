use mathi_runtime::{
    ipc::IpcBridge,
    db::RuntimeDatabase,
    memory::MemoryScope,
    policy::{default_decision, ActionClass, ApprovalDecision},
    types::{AgentEvent, AgentRequest, WorkerCommand},
    runtime_context::CapabilityContext,
    worker::spawn_worker,
    Orchestrator,
};
use std::collections::BTreeMap;
use uuid::Uuid;

#[tokio::test]
async fn orchestrator_bootstraps() {
    let orchestrator = Orchestrator::new(2, 32);
    orchestrator.bootstrap().await.expect("bootstrap");
}

#[test]
fn balanced_policy_defaults_match_expectations() {
    assert_eq!(default_decision(ActionClass::Read), ApprovalDecision::Allow);
    assert_eq!(default_decision(ActionClass::Write), ApprovalDecision::Allow);
    assert_eq!(default_decision(ActionClass::Shell), ApprovalDecision::Block);
    assert_eq!(default_decision(ActionClass::Network), ApprovalDecision::Block);
}

#[test]
fn sqlite_database_initializes_and_persists_samples() {
    let database = RuntimeDatabase::new_in_memory().expect("database");
    database.record_sample("startup", 12).expect("record sample");
    database.save_session_state("workspace_root", ".").expect("save state");
    assert_eq!(database.telemetry_count().expect("telemetry count"), 1);
}

#[test]
fn capability_context_defaults_are_balanced() {
    let context = CapabilityContext::new(".", 4, 32).with_balanced_guardrails();
    assert!(!context.allow_shell);
    assert!(!context.allow_network);
    assert_eq!(context.worker_count, 4);
    assert_eq!(context.channel_buffer, 32);
}

#[tokio::test]
async fn orchestrator_retains_workers_after_bootstrap() {
    let orchestrator = Orchestrator::new(2, 32);
    orchestrator.bootstrap().await.expect("bootstrap");
    assert!(orchestrator.worker_count() >= 2);
}

#[tokio::test]
async fn orchestrator_records_policy_audit_and_honors_revocation() {
    let orchestrator = Orchestrator::new(2, 32);
    orchestrator.bootstrap().await.expect("bootstrap");

    let mut request = AgentRequest {
        id: Uuid::new_v4(),
        task_type: "shell-check".to_string(),
        payload: serde_json::json!({"cmd": "cargo check"}),
        deadline_ms: Some(2_000),
        context: BTreeMap::new(),
    };
    request
        .context
        .insert("action_class".to_string(), serde_json::Value::String("shell".to_string()));
    request
        .context
        .insert("agent_id".to_string(), serde_json::Value::String("agent-shell".to_string()));
    request
        .context
        .insert("command".to_string(), serde_json::Value::String("cargo check".to_string()));

    orchestrator.grant_approval(ActionClass::Shell, "agent-shell");
    orchestrator.dispatch(request.clone()).await.expect("dispatch with approval");

    orchestrator.revoke_approval(ActionClass::Shell, "agent-shell");
    let denied = orchestrator.dispatch(request).await;
    assert!(denied.is_err());

    let audit_count = orchestrator.policy_audit_count().expect("policy audit count");
    assert!(audit_count >= 2);
}

#[tokio::test]
async fn orchestrator_enforces_credential_scope() {
    let orchestrator = Orchestrator::new(2, 32);
    orchestrator.bootstrap().await.expect("bootstrap");
    orchestrator
        .store_provider_secret("provider/openrouter", "sk-live-999")
        .expect("store provider secret");

    let mut request = AgentRequest {
        id: Uuid::new_v4(),
        task_type: "credentialed-task".to_string(),
        payload: serde_json::json!({}),
        deadline_ms: Some(2_000),
        context: BTreeMap::new(),
    };
    request
        .context
        .insert("action_class".to_string(), serde_json::Value::String("read".to_string()));
    request
        .context
        .insert("agent_id".to_string(), serde_json::Value::String("agent-cred".to_string()));
    request.context.insert(
        "provider_key".to_string(),
        serde_json::Value::String("provider/openrouter".to_string()),
    );
    request.context.insert(
        "agent_provider_scope".to_string(),
        serde_json::Value::String("provider/other".to_string()),
    );

    let denied = orchestrator.dispatch(request).await;
    assert!(denied.is_err());
}

#[tokio::test]
async fn orchestrator_memory_scope_boundary_and_retention_cleanup() {
    let orchestrator = Orchestrator::new(2, 32);
    orchestrator.bootstrap().await.expect("bootstrap");

    orchestrator
        .put_memory(
            MemoryScope::Persistent,
            "release-notes",
            "Bearer topsecret-token",
            Some(0),
        )
        .expect("put memory");

    let mut denied_request = AgentRequest {
        id: Uuid::new_v4(),
        task_type: "memory-task".to_string(),
        payload: serde_json::json!({}),
        deadline_ms: Some(2_000),
        context: BTreeMap::new(),
    };
    denied_request
        .context
        .insert("action_class".to_string(), serde_json::Value::String("read".to_string()));
    denied_request
        .context
        .insert("agent_id".to_string(), serde_json::Value::String("agent-memory".to_string()));
    denied_request
        .context
        .insert("memory_scope".to_string(), serde_json::Value::String("persistent".to_string()));
    denied_request
        .context
        .insert("memory_key".to_string(), serde_json::Value::String("release-notes".to_string()));

    let denied = orchestrator.dispatch(denied_request).await;
    assert!(denied.is_err());

    let _purged = orchestrator.cleanup_memory_retention().expect("cleanup");
    let after_cleanup = orchestrator
        .get_memory(MemoryScope::Persistent, "release-notes")
        .expect("get memory after cleanup");
    assert!(after_cleanup.is_none());
}

#[tokio::test]
async fn worker_executes_real_shell_commands() {
    let worker_id = Uuid::new_v4();
    let (bridge, command_rx, event_tx) = IpcBridge::new(8);
    let (command_tx, mut event_rx) = bridge.into_parts();
    let handle = spawn_worker(worker_id, command_rx, event_tx);

    loop {
        match event_rx.recv().await {
            Some(AgentEvent::Ready { worker_id: id }) if id == worker_id => break,
            Some(_) => continue,
            None => panic!("worker did not become ready"),
        }
    }

    let mut context = BTreeMap::new();
    context.insert("action_class".to_string(), serde_json::Value::String("shell".to_string()));
    context.insert("agent_id".to_string(), serde_json::Value::String("worker-shell".to_string()));
    context.insert("command".to_string(), serde_json::Value::String("cargo --version".to_string()));

    let request = AgentRequest {
        id: Uuid::new_v4(),
        task_type: "shell-version".to_string(),
        payload: serde_json::json!({}),
        deadline_ms: Some(5_000),
        context,
    };

    command_tx
        .send(WorkerCommand {
            id: Uuid::new_v4(),
            request,
        })
        .await
        .expect("send shell command");

    let mut finished_output = None;
    while let Some(event) = event_rx.recv().await {
        if let AgentEvent::Finished { output, .. } = event {
            finished_output = Some(output);
            break;
        }
    }

    handle.cancel().expect("cancel worker");
    let output = finished_output.expect("finished output");
    assert!(output.to_lowercase().contains("cargo"));
}
