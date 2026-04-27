use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct CapabilityContext {
    pub workspace_root: PathBuf,
    pub worker_count: usize,
    pub channel_buffer: usize,
    pub allow_shell: bool,
    pub allow_network: bool,
}

impl CapabilityContext {
    pub fn new(workspace_root: impl Into<PathBuf>, worker_count: usize, channel_buffer: usize) -> Self {
        Self {
            workspace_root: workspace_root.into(),
            worker_count,
            channel_buffer,
            allow_shell: false,
            allow_network: false,
        }
    }

    pub fn with_balanced_guardrails(mut self) -> Self {
        self.allow_shell = false;
        self.allow_network = false;
        self
    }
}
