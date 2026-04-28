use std::collections::BTreeMap;
use std::path::PathBuf;

use mathi_runtime::policy::{ActionClass, PolicyCheck, PolicyEngine, PolicyOutcome};
use mathi_runtime::runtime_context::CapabilityContext;
use mathi_runtime::types::AgentRequest;
use uuid::Uuid;

#[test]
fn read_action_allowed_by_default() {
    let engine = PolicyEngine::default();
    let context = CapabilityContext::new(".", 2, 32).with_balanced_guardrails();
    let check = PolicyCheck {
        action: ActionClass::Read,
        agent_id: "agent-1".to_string(),
        command: None,
        network_target: None,
        workspace_path: None,
    };

    assert_eq!(engine.evaluate(&check, &context), PolicyOutcome::Allow);
}

#[test]
fn shell_requires_approval_by_default() {
    let engine = PolicyEngine::default();
    let context = CapabilityContext::new(".", 2, 32).with_balanced_guardrails();
    let check = PolicyCheck {
        action: ActionClass::Shell,
        agent_id: "agent-shell".to_string(),
        command: Some("cargo check".to_string()),
        network_target: None,
        workspace_path: None,
    };

    match engine.evaluate(&check, &context) {
        PolicyOutcome::RequiresApproval(_) => {}
        other => panic!("expected approval required, got {other:?}"),
    }
}

#[test]
fn approved_shell_allows_safe_command_and_blocks_unsafe_one() {
    let engine = PolicyEngine::default();
    let context = CapabilityContext::new(".", 2, 32).with_balanced_guardrails();
    engine
        .approvals()
        .grant_session(ActionClass::Shell, "agent-shell");

    let safe = PolicyCheck {
        action: ActionClass::Shell,
        agent_id: "agent-shell".to_string(),
        command: Some("cargo test".to_string()),
        network_target: None,
        workspace_path: None,
    };
    assert_eq!(engine.evaluate(&safe, &context), PolicyOutcome::Allow);

    let unsafe_check = PolicyCheck {
        action: ActionClass::Shell,
        agent_id: "agent-shell".to_string(),
        command: Some("cmd /c del /s *".to_string()),
        network_target: None,
        workspace_path: None,
    };

    match engine.evaluate(&unsafe_check, &context) {
        PolicyOutcome::Deny(reason) => assert!(reason.contains("sandbox")),
        other => panic!("expected deny for unsafe command, got {other:?}"),
    }
}

#[test]
fn write_denied_outside_workspace_scope() {
    let engine = PolicyEngine::default();
    let workspace = std::env::current_dir().expect("cwd");
    let context = CapabilityContext::new(&workspace, 2, 32).with_balanced_guardrails();

    let outside = if cfg!(windows) {
        PathBuf::from("C:\\Windows\\System32\\drivers\\etc\\hosts")
    } else {
        PathBuf::from("/etc/hosts")
    };

    let check = PolicyCheck {
        action: ActionClass::Write,
        agent_id: "agent-write".to_string(),
        command: None,
        network_target: None,
        workspace_path: Some(outside),
    };

    match engine.evaluate(&check, &context) {
        PolicyOutcome::Deny(reason) => assert!(reason.contains("outside workspace")),
        other => panic!("expected deny for outside workspace write, got {other:?}"),
    }
}

#[test]
fn revoked_shell_approval_requires_reapproval() {
    let engine = PolicyEngine::default();
    let context = CapabilityContext::new(".", 2, 32).with_balanced_guardrails();
    engine.approvals().grant_session(ActionClass::Shell, "agent-shell");
    engine.approvals().revoke_session(ActionClass::Shell, "agent-shell");

    let check = PolicyCheck {
        action: ActionClass::Shell,
        agent_id: "agent-shell".to_string(),
        command: Some("cargo check".to_string()),
        network_target: None,
        workspace_path: None,
    };

    match engine.evaluate(&check, &context) {
        PolicyOutcome::RequiresApproval(_) => {}
        other => panic!("expected re-approval after revoke, got {other:?}"),
    }
}

#[test]
fn network_target_requires_allowlist_even_with_approval() {
    let engine = PolicyEngine::default();
    let context = CapabilityContext::new(".", 2, 32)
        .with_balanced_guardrails()
        .with_network_allowlist(["api.openrouter.ai", "api.mistral.ai"]);

    engine
        .approvals()
        .grant_session(ActionClass::Network, "agent-network");

    let allowed = PolicyCheck {
        action: ActionClass::Network,
        agent_id: "agent-network".to_string(),
        command: None,
        network_target: Some("api.mistral.ai".to_string()),
        workspace_path: None,
    };
    assert_eq!(engine.evaluate(&allowed, &context), PolicyOutcome::Allow);

    let denied = PolicyCheck {
        action: ActionClass::Network,
        agent_id: "agent-network".to_string(),
        command: None,
        network_target: Some("evil.example".to_string()),
        workspace_path: None,
    };

    match engine.evaluate(&denied, &context) {
        PolicyOutcome::Deny(reason) => assert!(reason.contains("network target")),
        other => panic!("expected deny for non-allowlisted host, got {other:?}"),
    }
}

#[test]
fn deadline_over_max_is_denied() {
    let engine = PolicyEngine::default();
    let context = CapabilityContext::new(".", 2, 32)
        .with_balanced_guardrails()
        .with_max_timeout(5_000);

    let mut request = AgentRequest {
        id: Uuid::new_v4(),
        task_type: "shell_task".to_string(),
        payload: serde_json::json!({"cmd": "cargo check"}),
        deadline_ms: Some(10_000),
        context: BTreeMap::new(),
    };
    request
        .context
        .insert("action_class".to_string(), serde_json::Value::String("shell".to_string()));
    request
        .context
        .insert("agent_id".to_string(), serde_json::Value::String("agent-shell".to_string()));

    match engine.evaluate_request(&request, &context) {
        PolicyOutcome::Deny(reason) => assert!(reason.contains("exceeds max allowed")),
        other => panic!("expected timeout denial, got {other:?}"),
    }
}
