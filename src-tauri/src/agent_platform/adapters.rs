use std::env;
use std::io::Write;
use std::net::TcpStream;
use std::process::{Command, Stdio};
use std::time::Instant;

use async_trait::async_trait;

use crate::agent_platform::contract::{
    AdapterKind, AgentExecution, AgentPlatformError, ProviderId, UnifiedAgentAdapter,
};

#[derive(Debug, Clone)]
pub struct AcpCliAdapter {
    provider: ProviderId,
    command: String,
    args: Vec<String>,
}

impl AcpCliAdapter {
    pub fn new(provider: impl Into<String>) -> Self {
        let provider = provider.into();
        let command = env::var("MATHI_ACP_CLI_COMMAND")
            .or_else(|_| env::var("ACP_CLI_COMMAND"))
            .unwrap_or_else(|_| "acp".to_string());
        let args = env::var("MATHI_ACP_CLI_ARGS")
            .or_else(|_| env::var("ACP_CLI_ARGS"))
            .ok()
            .and_then(|raw| shell_words::split(&raw).ok())
            .unwrap_or_default();
        Self::with_command(provider, command, args)
    }

    pub fn with_command(
        provider: impl Into<String>,
        command: impl Into<String>,
        args: Vec<String>,
    ) -> Self {
        Self {
            provider: ProviderId(provider.into()),
            command: command.into(),
            args,
        }
    }
}

#[async_trait]
impl UnifiedAgentAdapter for AcpCliAdapter {
    fn provider_id(&self) -> ProviderId {
        self.provider.clone()
    }

    fn kind(&self) -> AdapterKind {
        AdapterKind::AcpCli
    }

    async fn initialize(&self) -> Result<(), AgentPlatformError> {
        if self.command.is_empty() {
            return Err(AgentPlatformError::ProviderUnavailable(self.provider.0.clone()));
        }
        Ok(())
    }

    async fn execute(
        &self,
        task_type: &str,
        payload: &serde_json::Value,
    ) -> Result<AgentExecution, AgentPlatformError> {
        let started = Instant::now();
        let payload_bytes = serde_json::to_vec(&serde_json::json!({
            "provider": self.provider.0.clone(),
            "task_type": task_type,
            "payload": payload,
        }))
        .map_err(|error| AgentPlatformError::AdapterExecutionFailed(error.to_string()))?;

        let mut child = Command::new(&self.command)
            .args(&self.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| AgentPlatformError::ProviderUnavailable(error.to_string()))?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(&payload_bytes)
                .map_err(|error| AgentPlatformError::AdapterExecutionFailed(error.to_string()))?;
        }

        let output = child
            .wait_with_output()
            .map_err(|error| AgentPlatformError::AdapterExecutionFailed(error.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(AgentPlatformError::AdapterExecutionFailed(if stderr.is_empty() {
                format!("{} exited with {}", self.command, output.status)
            } else {
                stderr
            }));
        }

        let output_text = String::from_utf8(output.stdout)
            .map_err(|error| AgentPlatformError::AdapterExecutionFailed(error.to_string()))?;
        let output_text = output_text.trim().to_string();
        Ok(AgentExecution {
            provider: self.provider_id(),
            kind: self.kind(),
            task_type: task_type.to_string(),
            output: output_text,
            duration_ms: started.elapsed().as_millis() as u64,
            metadata: serde_json::json!({
                "route": "acp-cli",
                "payload_keys": payload.as_object().map(|o| o.len()).unwrap_or(0),
                "command": self.command,
                "args": self.args,
            }),
        })
    }

    async fn health_check(&self) -> bool {
        !self.command.is_empty()
    }

    async fn shutdown(&self) -> Result<(), AgentPlatformError> {
        Ok(())
    }
}

fn post_json(endpoint: &str, body: &str) -> Result<String, AgentPlatformError> {
    let stripped = endpoint
        .strip_prefix("http://")
        .ok_or_else(|| AgentPlatformError::ProviderUnavailable(endpoint.to_string()))?;

    let (host_port, path) = match stripped.split_once('/') {
        Some((host_port, path)) => (host_port, format!("/{path}")),
        None => (stripped, "/".to_string()),
    };

    let (host, port) = match host_port.split_once(':') {
        Some((host, port)) => {
            let parsed_port = port.parse::<u16>().map_err(|error| {
                AgentPlatformError::ProviderUnavailable(format!("invalid port in {endpoint}: {error}"))
            })?;
            (host.to_string(), parsed_port)
        }
        None => (host_port.to_string(), 80),
    };

    let mut stream = TcpStream::connect((host.as_str(), port))
        .map_err(|error| AgentPlatformError::ProviderUnavailable(error.to_string()))?;
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: {host}:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|error| AgentPlatformError::AdapterExecutionFailed(error.to_string()))?;

    let mut response = String::new();
    use std::io::Read;
    stream
        .read_to_string(&mut response)
        .map_err(|error| AgentPlatformError::AdapterExecutionFailed(error.to_string()))?;

    let (status_line, response_body) = response
        .split_once("\r\n")
        .ok_or_else(|| AgentPlatformError::AdapterExecutionFailed("malformed HTTP response".to_string()))?;

    if !status_line.contains("200") && !status_line.contains("201") {
        return Err(AgentPlatformError::AdapterExecutionFailed(status_line.to_string()));
    }

    let body = response_body
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .unwrap_or(response_body)
        .to_string();

    Ok(body)
}

#[derive(Debug, Clone)]
pub struct ApiNativeAdapter {
    provider: ProviderId,
    endpoint: String,
}

impl ApiNativeAdapter {
    pub fn new(provider: impl Into<String>) -> Self {
        let provider = provider.into();
        let endpoint = env::var("MATHI_API_NATIVE_ENDPOINT")
            .or_else(|_| env::var("API_NATIVE_ENDPOINT"))
            .unwrap_or_default();
        Self::with_endpoint(provider, endpoint)
    }

    pub fn with_endpoint(provider: impl Into<String>, endpoint: impl Into<String>) -> Self {
        Self {
            provider: ProviderId(provider.into()),
            endpoint: endpoint.into(),
        }
    }
}

#[async_trait]
impl UnifiedAgentAdapter for ApiNativeAdapter {
    fn provider_id(&self) -> ProviderId {
        self.provider.clone()
    }

    fn kind(&self) -> AdapterKind {
        AdapterKind::ApiNative
    }

    async fn initialize(&self) -> Result<(), AgentPlatformError> {
        if !self.endpoint.starts_with("http://") {
            return Err(AgentPlatformError::ProviderUnavailable(self.endpoint.clone()));
        }
        Ok(())
    }

    async fn execute(
        &self,
        task_type: &str,
        payload: &serde_json::Value,
    ) -> Result<AgentExecution, AgentPlatformError> {
        let started = Instant::now();
        let request_body = serde_json::to_string(&serde_json::json!({
            "provider": self.provider.0.clone(),
            "task_type": task_type,
            "payload": payload,
        }))
        .map_err(|error| AgentPlatformError::AdapterExecutionFailed(error.to_string()))?;

        let response_body = post_json(&self.endpoint, &request_body)?;
        Ok(AgentExecution {
            provider: self.provider_id(),
            kind: self.kind(),
            task_type: task_type.to_string(),
            output: response_body.trim().to_string(),
            duration_ms: started.elapsed().as_millis() as u64,
            metadata: serde_json::json!({
                "route": "api-native",
                "payload_size": payload.to_string().len(),
                "endpoint": self.endpoint,
            }),
        })
    }

    async fn health_check(&self) -> bool {
        self.endpoint.starts_with("http://")
    }

    async fn shutdown(&self) -> Result<(), AgentPlatformError> {
        Ok(())
    }
}
