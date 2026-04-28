use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct CapabilityContext {
    pub workspace_root: PathBuf,
    pub worker_count: usize,
    pub channel_buffer: usize,
    pub allow_shell: bool,
    pub allow_network: bool,
    pub allowed_network_hosts: HashSet<String>,
    pub max_task_timeout_ms: u64,
}

impl CapabilityContext {
    pub fn new(workspace_root: impl Into<PathBuf>, worker_count: usize, channel_buffer: usize) -> Self {
        Self {
            workspace_root: workspace_root.into(),
            worker_count,
            channel_buffer,
            allow_shell: false,
            allow_network: false,
            allowed_network_hosts: HashSet::new(),
            max_task_timeout_ms: 60_000,
        }
    }

    pub fn with_balanced_guardrails(mut self) -> Self {
        self.allow_shell = false;
        self.allow_network = false;
        self
    }

    pub fn with_network_allowlist<I, S>(mut self, hosts: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.allowed_network_hosts = hosts.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_max_timeout(mut self, timeout_ms: u64) -> Self {
        self.max_task_timeout_ms = timeout_ms;
        self
    }
}
