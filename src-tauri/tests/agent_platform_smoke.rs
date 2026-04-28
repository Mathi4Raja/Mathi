use async_trait::async_trait;
use std::sync::Arc;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use mathi_runtime::agent_platform::{
    AcpCliAdapter, AdapterKind, AgentExecution, AgentPlatformError, ApiNativeAdapter,
    ExecutionPlan, ProviderId, Scheduler, SwarmCoordinator, SwarmTemplate, UnifiedAgentAdapter,
};

#[derive(Debug, Clone)]
struct FailingAdapter {
    provider: ProviderId,
}

impl FailingAdapter {
    fn new(provider: impl Into<String>) -> Self {
        Self {
            provider: ProviderId(provider.into()),
        }
    }
}

#[async_trait]
impl UnifiedAgentAdapter for FailingAdapter {
    fn provider_id(&self) -> ProviderId {
        self.provider.clone()
    }

    fn kind(&self) -> AdapterKind {
        AdapterKind::AcpCli
    }

    async fn initialize(&self) -> Result<(), AgentPlatformError> {
        Err(AgentPlatformError::ProviderUnavailable(self.provider.0.clone()))
    }

    async fn execute(
        &self,
        task_type: &str,
        _payload: &serde_json::Value,
    ) -> Result<AgentExecution, AgentPlatformError> {
        Err(AgentPlatformError::AdapterExecutionFailed(format!(
            "{} failed {}",
            self.provider.0, task_type
        )))
    }

    async fn health_check(&self) -> bool {
        false
    }

    async fn shutdown(&self) -> Result<(), AgentPlatformError> {
        Ok(())
    }
}

fn spawn_http_server(response_body: &'static str, connections: usize) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
    let address = listener.local_addr().expect("server addr");

    thread::spawn(move || {
        for _ in 0..connections {
            let (mut stream, _) = listener.accept().expect("accept request");
            let _request = read_http_request(&mut stream);
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response_body.len(),
                response_body
            );
            stream.write_all(response.as_bytes()).expect("write response");
        }
    });

    format!("http://127.0.0.1:{}/execute", address.port())
}

fn read_http_request(stream: &mut std::net::TcpStream) -> String {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 1024];

    loop {
        let read = stream.read(&mut chunk).expect("read request chunk");
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);

        if let Some(header_end) = find_subsequence(&buffer, b"\r\n\r\n") {
            let header_text = String::from_utf8_lossy(&buffer[..header_end]);
            let content_length = header_text
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    if name.eq_ignore_ascii_case("content-length") {
                        value.trim().parse::<usize>().ok()
                    } else {
                        None
                    }
                })
                .unwrap_or(0);
            let body_start = header_end + 4;
            while buffer.len() < body_start + content_length {
                let more = stream.read(&mut chunk).expect("read request body");
                if more == 0 {
                    break;
                }
                buffer.extend_from_slice(&chunk[..more]);
            }
            break;
        }
    }

    String::from_utf8_lossy(&buffer).to_string()
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|window| window == needle)
}

fn acp_echo_command() -> (String, Vec<String>) {
    #[cfg(windows)]
    {
        ("cmd".to_string(), vec!["/C".to_string(), "more".to_string()])
    }

    #[cfg(not(windows))]
    {
        ("sh".to_string(), vec!["-lc".to_string(), "cat".to_string()])
    }
}

#[tokio::test]
async fn scheduler_uses_fallback_when_primary_fails() {
    let endpoint = spawn_http_server("secondary-ok", 1);
    let mut scheduler = Scheduler::new();
    scheduler.register(Arc::new(FailingAdapter::new("primary-fail")));
    scheduler.register(Arc::new(ApiNativeAdapter::with_endpoint("secondary-api", endpoint)));

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
async fn acp_cli_adapter_executes_against_real_process() {
    let (command, args) = acp_echo_command();
    let adapter = AcpCliAdapter::with_command("gemini-cli", command, args);
    let result = adapter
        .execute("research", &serde_json::json!({"topic": "ipc"}))
        .await
        .expect("acp success");

    assert_eq!(result.provider.0, "gemini-cli");
    assert!(result.output.contains("research"));
    assert!(result.output.contains("gemini-cli"));
}

#[tokio::test]
async fn api_native_adapter_posts_to_real_http_service() {
    let endpoint = spawn_http_server("mistral-ok", SwarmTemplate::CodeFix.task_sequence().len());
    let adapter = ApiNativeAdapter::with_endpoint("mistral-api", endpoint);
    let result = adapter
        .execute("generate_patch", &serde_json::json!({"file": "src/lib.rs"}))
        .await
        .expect("api success");

    assert_eq!(result.provider.0, "mistral-api");
    assert_eq!(result.kind, mathi_runtime::agent_platform::AdapterKind::ApiNative);
    assert!(result.output.contains("mistral-ok"));
}

#[tokio::test]
async fn swarm_templates_execute_task_sequences() {
    let endpoint = spawn_http_server("mistral-ok", SwarmTemplate::CodeFix.task_sequence().len());
    let mut scheduler = Scheduler::new();
    scheduler.register(Arc::new(ApiNativeAdapter::with_endpoint("mistral-api", endpoint)));

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
