use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::runtime_context::CapabilityContext;
use crate::types::AgentRequest;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionClass {
    Read,
    Write,
    Shell,
    Network,
    Credentials,
    Tool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalDecision {
    Allow,
    Block,
    Scoped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyOutcome {
    Allow,
    RequiresApproval(String),
    Deny(String),
}

#[derive(Debug, Clone)]
pub struct PolicyCheck {
    pub action: ActionClass,
    pub agent_id: String,
    pub command: Option<String>,
    pub network_target: Option<String>,
    pub workspace_path: Option<PathBuf>,
}

#[derive(Debug, Default)]
pub struct ApprovalStore {
    approvals: Mutex<HashSet<String>>,
}

impl ApprovalStore {
    pub fn grant_session(&self, action: ActionClass, agent_id: &str) {
        let mut approvals = self.approvals.lock().expect("approval store mutex poisoned");
        approvals.insert(Self::key(action, agent_id));
    }

    pub fn is_granted(&self, action: ActionClass, agent_id: &str) -> bool {
        let approvals = self.approvals.lock().expect("approval store mutex poisoned");
        approvals.contains(&Self::key(action, agent_id))
    }

    pub fn revoke_session(&self, action: ActionClass, agent_id: &str) {
        let mut approvals = self.approvals.lock().expect("approval store mutex poisoned");
        approvals.remove(&Self::key(action, agent_id));
    }

    pub fn revoke_all_for_agent(&self, agent_id: &str) {
        let mut approvals = self.approvals.lock().expect("approval store mutex poisoned");
        approvals.retain(|entry| !entry.ends_with(&format!(":{agent_id}")));
    }

    fn key(action: ActionClass, agent_id: &str) -> String {
        format!("{:?}:{agent_id}", action)
    }
}

#[derive(Debug)]
pub struct SandboxConfig {
    allowed_commands: HashSet<String>,
    denied_tokens: HashSet<String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        let allowed_commands = ["git", "cargo", "rustc", "node", "npm", "python", "pip", "tauri"]
            .into_iter()
            .map(ToString::to_string)
            .collect();
        let denied_tokens = ["&&", "|", ";", "rm -rf", "del /s", "format", "powershell -c", "cmd /c"]
            .into_iter()
            .map(ToString::to_string)
            .collect();
        Self {
            allowed_commands,
            denied_tokens,
        }
    }
}

impl SandboxConfig {
    pub fn is_safe_command(&self, command: &str) -> bool {
        let normalized = command.to_lowercase();
        if self
            .denied_tokens
            .iter()
            .any(|token| normalized.contains(token.as_str()))
        {
            return false;
        }

        let executable = normalized.split_whitespace().next().unwrap_or_default();
        self.allowed_commands.contains(executable)
    }

    pub fn is_allowed_network_target(&self, target: &str, context: &CapabilityContext) -> bool {
        if context.allowed_network_hosts.is_empty() {
            return false;
        }

        context.allowed_network_hosts.contains(target)
    }
}

#[derive(Debug, Default)]
pub struct PolicyEngine {
    approvals: ApprovalStore,
    sandbox: SandboxConfig,
}

impl PolicyEngine {
    pub fn approvals(&self) -> &ApprovalStore {
        &self.approvals
    }

    pub fn evaluate(&self, check: &PolicyCheck, context: &CapabilityContext) -> PolicyOutcome {
        match default_decision(check.action) {
            ApprovalDecision::Allow => self.evaluate_allow_action(check, context),
            ApprovalDecision::Scoped => PolicyOutcome::Allow,
            ApprovalDecision::Block => self.evaluate_blocked_action(check, context),
        }
    }

    pub fn evaluate_request(&self, request: &AgentRequest, context: &CapabilityContext) -> PolicyOutcome {
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

        let command = request
            .context
            .get("command")
            .and_then(|value| value.as_str())
            .map(ToString::to_string);
        let network_target = request
            .context
            .get("network_target")
            .and_then(|value| value.as_str())
            .map(ToString::to_string);
        let workspace_path = request
            .context
            .get("workspace_path")
            .and_then(|value| value.as_str())
            .map(PathBuf::from);

        let check = PolicyCheck {
            action,
            agent_id,
            command,
            network_target,
            workspace_path,
        };

        if let Some(deadline_ms) = request.deadline_ms {
            if deadline_ms > context.max_task_timeout_ms {
                return PolicyOutcome::Deny(format!(
                    "deadline {}ms exceeds max allowed {}ms",
                    deadline_ms, context.max_task_timeout_ms
                ));
            }
        }

        self.evaluate(&check, context)
    }

    fn evaluate_allow_action(&self, check: &PolicyCheck, context: &CapabilityContext) -> PolicyOutcome {
        if check.action == ActionClass::Write || check.action == ActionClass::Shell {
            if let Some(path) = &check.workspace_path {
                if !is_in_workspace(path, &context.workspace_root) {
                    return PolicyOutcome::Deny("path is outside workspace scope".to_string());
                }
            }
        }
        PolicyOutcome::Allow
    }

    fn evaluate_blocked_action(&self, check: &PolicyCheck, context: &CapabilityContext) -> PolicyOutcome {
        if self.approvals.is_granted(check.action, &check.agent_id) {
            match check.action {
                ActionClass::Shell => {
                    if let Some(command) = &check.command {
                        if !self.sandbox.is_safe_command(command) {
                            return PolicyOutcome::Deny("shell command rejected by sandbox".to_string());
                        }
                    }
                    PolicyOutcome::Allow
                }
                ActionClass::Network => {
                    if let Some(target) = &check.network_target {
                        if !self.sandbox.is_allowed_network_target(target, context) {
                            return PolicyOutcome::Deny("network target rejected by sandbox".to_string());
                        }
                    }
                    PolicyOutcome::Allow
                }
                _ => PolicyOutcome::Allow,
            }
        } else {
            PolicyOutcome::RequiresApproval(format!("{:?} requires approval for {}", check.action, check.agent_id))
        }
    }
}

fn is_in_workspace(path: &Path, workspace_root: &Path) -> bool {
    let normalized = path.canonicalize().ok();
    let normalized_root = workspace_root.canonicalize().ok();
    match (normalized, normalized_root) {
        (Some(path), Some(root)) => path.starts_with(root),
        _ => path.starts_with(workspace_root),
    }
}

pub fn parse_action_class(input: &str) -> Option<ActionClass> {
    match input.to_lowercase().as_str() {
        "read" => Some(ActionClass::Read),
        "write" => Some(ActionClass::Write),
        "shell" => Some(ActionClass::Shell),
        "network" => Some(ActionClass::Network),
        "credentials" => Some(ActionClass::Credentials),
        "tool" => Some(ActionClass::Tool),
        _ => None,
    }
}

pub fn default_decision(action: ActionClass) -> ApprovalDecision {
    match action {
        ActionClass::Read | ActionClass::Write | ActionClass::Credentials | ActionClass::Tool => ApprovalDecision::Allow,
        ActionClass::Shell | ActionClass::Network => ApprovalDecision::Block,
    }
}
