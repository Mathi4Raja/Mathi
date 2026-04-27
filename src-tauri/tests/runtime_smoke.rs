use mathi_runtime::{
    db::RuntimeDatabase,
    policy::{default_decision, ActionClass, ApprovalDecision},
    runtime_context::CapabilityContext,
    Orchestrator,
};

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
