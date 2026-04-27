use std::sync::Arc;

use mathi_runtime::agent_platform::{
    AcpCliAdapter, ApiNativeAdapter, ExecutionPlan, MockFailAdapter, ProviderId, Scheduler,
    SwarmCoordinator, SwarmTemplate,
};

#[tokio::test]
async fn scheduler_uses_fallback_when_primary_fails() {
    let mut scheduler = Scheduler::new();
    scheduler.register(Arc::new(MockFailAdapter::new("primary-fail")));
    scheduler.register(Arc::new(ApiNativeAdapter::new("secondary-api")));

    let plan = ExecutionPlan::new(
        "code_fix",
        serde_json::json!({"file": "src/main.rs"}),
        vec![
            ProviderId("primary-fail".to_string()),
            ProviderId("secondary-api".to_string()),
        ],
    );

    let result = scheduler.execute_with_fallback(&plan).await.expect("fallback success");
    assert_eq!(result.provider.0, "secondary-api");
}

#[tokio::test]
async fn scheduler_supports_acp_adapter() {
    let mut scheduler = Scheduler::new();
    scheduler.register(Arc::new(AcpCliAdapter::new("gemini-cli")));

    let plan = ExecutionPlan::new(
        "research",
        serde_json::json!({"topic": "ipc"}),
        vec![ProviderId("gemini-cli".to_string())],
    );

    let result = scheduler.execute_with_fallback(&plan).await.expect("acp success");
    assert_eq!(result.provider.0, "gemini-cli");
    assert!(result.output.contains("acp"));
}

#[tokio::test]
async fn swarm_templates_execute_task_sequences() {
    let mut scheduler = Scheduler::new();
    scheduler.register(Arc::new(ApiNativeAdapter::new("mistral-api")));

    let coordinator = SwarmCoordinator::new(scheduler);
    let summary = coordinator
        .execute_template(
            SwarmTemplate::CodeFix,
            serde_json::json!({"file": "src/lib.rs"}),
            vec![ProviderId("mistral-api".to_string())],
        )
        .await
        .expect("swarm execution");

    assert_eq!(summary.executions.len(), SwarmTemplate::CodeFix.task_sequence().len());
    assert!(summary.total_duration_ms() <= 2000);
}

#[test]
fn template_team_size_ranges_are_locked() {
    assert_eq!(SwarmTemplate::CodeFix.team_size_range(), (2, 4));
    assert_eq!(SwarmTemplate::Refactor.team_size_range(), (3, 6));
    assert_eq!(SwarmTemplate::Research.team_size_range(), (2, 3));
}
