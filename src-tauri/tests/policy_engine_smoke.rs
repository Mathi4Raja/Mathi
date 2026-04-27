use std::path::PathBuf;

use mathi_runtime::policy::{ActionClass, PolicyCheck, PolicyEngine, PolicyOutcome};
use mathi_runtime::runtime_context::CapabilityContext;

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
