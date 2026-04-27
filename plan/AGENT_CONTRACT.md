# Agent Contract (Unified Interface)
## Phase 0 Deliverable
**Date**: 2026-04-27  
**Status**: APPROVED  
**Owner**: Mathi

---

## 1. CONTRACT OVERVIEW

All agents (ACP CLI and API-native) must implement the `AgentContract` trait. This ensures:
- **Swappability**: Adapters are interchangeable; orchestrator has no provider-specific code
- **Reliability**: Standard error handling, cancellation, streaming semantics
- **Performance**: Bounded resource use; timeouts enforced uniformly
- **Testability**: Mock adapters can be substituted; contract is easy to unit test

---

## 2. CORE TRAIT (Rust Definition)

```rust
use tokio::sync::mpsc;
use std::collections::HashMap;

/// Result type for agent operations
pub type AgentResult<T> = Result<T, AgentError>;

#[derive(Debug, Clone)]
pub enum AgentError {
    InitFailed(String),
    ExecutionFailed(String),
    Timeout,
    Cancelled,
    InvalidPayload(String),
    AdapterUnavailable(String),
    RateLimited { retry_after_ms: u64 },
    Unknown(String),
}

/// Streaming event from agent to orchestrator
#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// Agent is ready; return capabilities and metadata
    Ready(AgentCapabilities),
    
    /// Streaming chunk of output (code, text, diagnostics)
    StreamChunk(StreamChunk),
    
    /// Agent finished; return final summary
    Finished(AgentResult),
    
    /// Progress update (e.g., "analyzed 50 files")
    Progress { current: u32, total: u32, message: String },
    
    /// Non-fatal diagnostic (warning, timing, cache hit)
    Diagnostic(DiagnosticEvent),
    
    /// Agent explicitly cancelling (e.g., user interrupt)
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct AgentCapabilities {
    pub adapter_id: String,           // "gemini-cli" | "opencode" | "claude-api"
    pub version: String,              // e.g., "1.2.3"
    pub supported_tasks: Vec<String>, // ["analyze", "refactor", "test-gen"]
    pub model_info: ModelInfo,
}

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub name: String,
    pub context_window: usize,
    pub supports_streaming: bool,
    pub supports_tool_use: bool,
}

#[derive(Debug, Clone)]
pub struct StreamChunk {
    pub sequence: u64,
    pub content: String,
    pub is_final: bool,
    pub metadata: Option<ChunkMetadata>,
}

#[derive(Debug, Clone)]
pub struct ChunkMetadata {
    pub mime_type: Option<String>,
    pub language: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DiagnosticEvent {
    pub level: DiagnosticLevel,
    pub message: String,
    pub timestamp_ms: i64,
}

#[derive(Debug, Clone)]
pub enum DiagnosticLevel {
    Debug,
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct AgentResult {
    pub output: String,
    pub status: String,              // "success" | "partial" | "error"
    pub duration_ms: u64,
    pub tokens_used: Option<TokenUsage>,
}

#[derive(Debug, Clone)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}

/// Task request from orchestrator to agent
#[derive(Debug, Clone)]
pub struct TaskRequest {
    pub id: u64,
    pub task_type: String,        // "code_analyze", "refactor", "test_gen", etc.
    pub context: HashMap<String, serde_json::Value>,
    pub payload: serde_json::Value,
    pub deadline_ms: Option<i64>,
}

/// Agent trait: All adapters implement this
#[async_trait::async_trait]
pub trait Agent: Send + Sync {
    /// Initialize agent; return capabilities and metadata
    /// 
    /// Called once at worker startup. Must complete within 5s.
    /// Returns: AgentCapabilities (name, version, model info)
    /// Errors: InitFailed, AdapterUnavailable, Timeout
    async fn initialize(&self) -> AgentResult<AgentCapabilities>;
    
    /// Execute a task; stream results via tx channel
    /// 
    /// Called for each task. Must respect deadline_ms.
    /// Sends events to tx; closes channel when done.
    /// Errors: ExecutionFailed, Timeout, RateLimited, InvalidPayload
    /// 
    /// Contract:
    ///   - Send Ready event first (or error)
    ///   - Stream 1+ chunks (StreamChunk events)
    ///   - Send Finished event last (always)
    ///   - Respect Cancelled flag; exit cleanly
    ///   - Total execution <= deadline_ms (or send Timeout error)
    async fn execute(
        &self,
        request: TaskRequest,
        cancel_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
        tx: mpsc::UnboundedSender<AgentEvent>,
    ) -> AgentResult<()>;
    
    /// Health check; verify adapter is responsive
    /// 
    /// Called periodically (every 30s) by orchestrator.
    /// Must complete within 5s.
    /// Returns: true if healthy, false if degraded.
    async fn health_check(&self) -> bool;
    
    /// Shutdown agent; clean up resources
    /// 
    /// Called on worker exit or session cancel.
    /// Must complete within 2s.
    /// Safe to call multiple times.
    async fn shutdown(&self) -> AgentResult<()>;
}

/// Minimal mock for testing
#[cfg(test)]
pub struct MockAgent;

#[cfg(test)]
#[async_trait::async_trait]
impl Agent for MockAgent {
    async fn initialize(&self) -> AgentResult<AgentCapabilities> {
        Ok(AgentCapabilities {
            adapter_id: "mock".to_string(),
            version: "1.0.0".to_string(),
            supported_tasks: vec!["analyze".to_string()],
            model_info: ModelInfo {
                name: "MockModel".to_string(),
                context_window: 4096,
                supports_streaming: true,
                supports_tool_use: false,
            },
        })
    }
    
    async fn execute(
        &self,
        _request: TaskRequest,
        _cancel_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
        tx: mpsc::UnboundedSender<AgentEvent>,
    ) -> AgentResult<()> {
        let _ = tx.send(AgentEvent::StreamChunk(StreamChunk {
            sequence: 0,
            content: "Mock result".to_string(),
            is_final: true,
            metadata: None,
        }));
        Ok(())
    }
    
    async fn health_check(&self) -> bool {
        true
    }
    
    async fn shutdown(&self) -> AgentResult<()> {
        Ok(())
    }
}
```

---

## 3. ADAPTER IMPLEMENTATION CHECKLIST

Each adapter (Gemini CLI, OpenCode, Claude API, etc.) must:

- [ ] **Implement `AgentContract` trait** with all 4 methods (initialize, execute, health_check, shutdown)
- [ ] **Streaming**: Send `StreamChunk` events as output arrives (not wait for full response)
- [ ] **Cancellation**: Poll `cancel_flag.load(Ordering::Relaxed)` every 100ms; exit immediately if true
- [ ] **Timeout**: Compare `SystemTime::now()` against `deadline_ms`; send `Timeout` error if exceeded
- [ ] **Error Handling**: Map adapter-specific errors to `AgentError` enum (don't leak provider details)
- [ ] **Token Tracking**: Populate `TokenUsage` if available; else leave `None`
- [ ] **Rate Limiting**: Return `RateLimited { retry_after_ms }` if quota exceeded; orchestrator retries
- [ ] **Health Check**: Quick sanity check (e.g., `adapter --version`); return true/false
- [ ] **Graceful Shutdown**: Clean up resources (close sockets, kill subprocesses); complete within 2s

---

## 4. ORCHESTRATOR RESPONSIBILITIES (How it uses the trait)

```rust
pub struct AgentWorker {
    agent: Box<dyn Agent>,
    id: u64,
    cancel_flag: Arc<AtomicBool>,
}

impl AgentWorker {
    /// Lifecycle: init → execute* → shutdown
    pub async fn run(&self) -> WorkerResult {
        // 1. Initialize (with timeout)
        let caps = tokio::time::timeout(
            Duration::from_secs(5),
            self.agent.initialize()
        ).await??;
        
        // 2. Execute tasks until cancel
        while let Some(request) = self.rx.recv().await {
            let (tx, mut rx) = mpsc::unbounded_channel();
            
            // Spawn execute with deadline
            let execute_fut = self.agent.execute(request, self.cancel_flag.clone(), tx);
            let timeout_dur = Duration::from_millis(request.deadline_ms.unwrap_or(60000));
            
            match tokio::time::timeout(timeout_dur, execute_fut).await {
                Ok(Ok(())) => {
                    // Collect events from rx
                    while let Some(event) = rx.recv().await {
                        self.report_event(event).await;
                    }
                }
                Ok(Err(e)) => self.report_error(e).await,
                Err(_) => self.report_error(AgentError::Timeout).await,
            }
        }
        
        // 3. Cleanup
        self.agent.shutdown().await?;
        Ok(())
    }
}
```

---

## 5. EXPECTED BEHAVIOR PATTERNS

### 5.1 Happy Path (Successful Execution)
```
Orchestrator sends TaskRequest
    ↓
Agent: initialize (if first task)
    ↓
Agent: execute
    → Send Ready event
    → Stream N chunks
    → Send Finished(Success)
    ↓
Orchestrator: collect results, send to UI
```

### 5.2 Cancellation
```
Orchestrator: set cancel_flag = true (user cancels)
    ↓
Agent: polls cancel_flag, sees true
    → Stop processing (backtrack partial state)
    → Send Cancelled event
    ↓
Orchestrator: clean up task, report user cancellation
```

### 5.3 Rate Limiting
```
Agent: rate limit triggered (API quota exhausted)
    → Send RateLimited { retry_after_ms: 5000 }
    ↓
Orchestrator: re-queue task with delay; pick next adapter in fallback list
```

### 5.4 Timeout
```
Agent: takes >deadline_ms to respond
    ↓
Orchestrator: cancel_flag set; timeout fires
    → Agent receives cancel signal, cleans up
    → Send Timeout error
    ↓
Orchestrator: retry with next adapter (if available)
```

---

## 6. ERROR RECOVERY STRATEGY

| Error | Orchestrator Action |
|-------|---------------------|
| `InitFailed` | Log warning; try next adapter in priority list; if all fail, alert user. |
| `ExecutionFailed` | Log error; send to UI; optionally retry with next adapter. |
| `Timeout` | Log warning; cancel task; retry with next adapter (up to 3x). |
| `Cancelled` | Expected; clean up; no retry (user action). |
| `RateLimited` | Re-queue task with exponential backoff + jitter; cap retries at 5. |
| `AdapterUnavailable` | Log error; remove from priority list; alert user if no adapters left. |

---

## 7. TESTING STRATEGY

### 7.1 Unit Tests (Per Adapter)
```rust
#[tokio::test]
async fn test_initialize() {
    let agent = GeminiAdapter::new(api_key);
    let caps = agent.initialize().await.expect("init");
    assert_eq!(caps.adapter_id, "gemini-cli");
    assert!(caps.model_info.supports_streaming);
}

#[tokio::test]
async fn test_execute_streaming() {
    let agent = GeminiAdapter::new(api_key);
    let request = TaskRequest { /* ... */ };
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let (tx, mut rx) = mpsc::unbounded_channel();
    
    agent.execute(request, cancel_flag, tx).await.unwrap();
    
    let mut chunks_received = 0;
    while let Some(AgentEvent::StreamChunk(_)) = rx.recv().await {
        chunks_received += 1;
    }
    assert!(chunks_received > 0);
}

#[tokio::test]
async fn test_cancellation() {
    let agent = GeminiAdapter::new(api_key);
    let request = TaskRequest { /* ... */ };
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::unbounded_channel();
    
    // Spawn execute in background
    let handle = tokio::spawn(agent.execute(request, cancel_flag.clone(), tx));
    
    // After 100ms, set cancel
    tokio::time::sleep(Duration::from_millis(100)).await;
    cancel_flag.store(true, Ordering::Relaxed);
    
    // Should finish soon
    let result = tokio::time::timeout(Duration::from_secs(2), handle).await;
    assert!(result.is_ok(), "cancellation should complete quickly");
}

#[tokio::test]
async fn test_timeout() {
    let agent = GeminiAdapter::new(api_key);
    let mut request = TaskRequest { /* ... */ };
    request.deadline_ms = Some(100); // Very short deadline
    
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::unbounded_channel();
    
    agent.execute(request, cancel_flag, tx).await.expect_err("should timeout");
}
```

### 7.2 Integration Tests (Orchestrator + Adapter)
- Test fallback routing: primary adapter fails → secondary succeeds
- Test approval gate: shell command requires approval before adapter gets request
- Test memory cleanup: adapter shutdown doesn't leak resources

---

## 8. VERSION & MIGRATION

| Version | Date | Changes |
|---------|------|---------|
| v1.0 | 2026-04-27 | Initial agent contract; 4 methods (init, execute, health_check, shutdown). |
| v1.1 | TBD | Add tool_use capability flag; streaming metadata enrichment. |
| v2.0 | TBD | Multi-turn context; persistent agent memory. |

---

## Sign-Off

**Approved By**: Mathi  
**Date**: 2026-04-27  
**Status**: Locked for Phase 1 adapter implementation

**Next Steps**:
- P1: Scaffold adapter base class in Rust; test harness
- P3: Implement adapters (Gemini, OpenCode, Claude, etc.)
- P3: Integration tests with orchestrator
