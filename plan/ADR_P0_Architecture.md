# Architecture Decision Record (ADR)
## Phase 0: Foundation & Constraints Lock
**Date**: 2026-04-27  
**Status**: APPROVED  
**Owner**: Mathi  
**Scope**: v1 MVP (4–8 week delivery)

---

## 1. SYSTEM OVERVIEW

**Project Name**: Agent-Native IDE (codename: Mathi)

**Vision**: Standalone, high-performance IDE with multi-agent swarm orchestration, supporting ACP CLI and API-native agents with balanced guardrails and local-first privacy.

**Non-Goals**: VS Code fork, collaborative editing, peer mesh, custom plugin SDK, cloud sync, full LSP in v1.

---

## 2. STACK DECISIONS

### 2.1 Runtime Stack
| Layer | Decision | Rationale | Trade-off |
|-------|----------|-----------|-----------|
| **Frontend UI** | Tauri (Rust → Webview) | Startup <1s, idle <150MB vs Electron (~200MB). Single-binary distribution. | Build complexity higher than TS/Electron. Learning curve for Rust async. |
| **Backend Core** | Rust + tokio (async/await) | Performance (no GC pauses), memory efficiency, type safety, concurrency model aligns with multi-agent orchestration. | Longer dev cycles than Python; stdlib is minimal. |
| **IPC Mechanism** | Bounded async channels (tokio::sync::mpsc) + JSON-RPC-like wire format | Battle-tested Rust pattern; supports backpressure, cancellation, error propagation. | Not a formal RPC framework; custom serialization required. |
| **Persistence** | SQLite + optional vector DB (Qdrant/Milvus, deferred to v1.1) | Zero-setup deployment, local by default, schema versioning via migrations. | Single-threaded writes (mitigate: write-ahead logging, batch inserts). |
| **Auth/Secrets** | Local vault (encrypted file + OS keychain integration) | No cloud dependency, offline-first, user controls export/import. | Key rotation requires manual workflow; no built-in expiry. |
| **Platforms** | Windows, macOS (v1); Linux deferred to v1.1 | Tauri targets both; macOS notarization chain complete; Windows installer via MSI. | Linux Wayland support (deferrable to v1.1). |

### 2.2 Agent Orchestration Stack
| Component | Decision | Rationale |
|-----------|----------|-----------|
| **Swarm Model** | Leader-worker (no peer mesh) | Simpler state machine; no consensus algorithm overhead (Raft/Paxos overkill for v1). Single leader is bottleneck but mitigated by health monitoring + quick failover. |
| **Task Scheduling** | Dependency-aware DAG (Directed Acyclic Graph) | Precedence modeling (e.g., "analyze before refactor"), prevents starving high-priority tasks, reuses CLIonGUI `TaskManager` pattern. |
| **Agent Adapters** | Unified `AgentContract` (init, stream, cancel, error, cleanup) | ACP CLI and API-native agents present same interface; no orchestrator has adapter-specific code. Swappable fallback routing. |
| **Worker Lifecycle** | Supervised with exponential backoff restart | Single worker crash doesn't kill swarm; max 5 restarts per 60s window, then alert user. Prevents cascading failures. |
| **Approval Gates** | Session-level cache (not per-action modal) | One "grant shell for this session" → all agents inherit it. Reduces UX spam but requires revocable scope + clear audit log. |

---

## 3. SCOPE DECISIONS (Locked for v1)

| Decision | Rationale | v1.1+ Plan |
|----------|-----------|-----------|
| **Process isolation only** (no containers in v1) | Deliver agent platform in 4–8 weeks. Containers add 2–3 weeks. Process isolation covers 90% of v1 guardrails use case. | v1.1: Full container isolation with seccomp, cgroup limits, network namespaces. |
| **3 swarm templates only** (no dynamic composition) | Fixed team shapes → predictable perf + easy testing. Dynamic = unbounded complexity. User selects template at session start. | v2: Dynamic team spawning, custom role composition, runtime rebalancing. |
| **No full LSP in v1** | LSP is massive surface area. Defer to v1.1 or v2. Support via adapter hooks (call external lsp-server process, cache results). | v1.1: Full LSP server mode + client caching. |
| **Local-first, no cloud sync** | User controls data; no privacy concerns. All state encrypted at rest. Git handles code collab. | v2: Optional encrypted cloud backup (user opt-in, no telemetry). |
| **Balanced guardrails by default** (not strict) | Read + workspace write OK; shell/network/creds require approval. Improves UX velocity vs strict sandbox. | v1.1: Optional "strict mode" for sensitive projects. |

---

## 4. CRITICAL ARCHITECTURE BOUNDARIES

### 4.1 Process Topology (v1)
```
┌─────────────────────────────────────────────┐
│         Tauri Frontend (Webview)            │
│  - File UI, terminal, editor canvas, agent  │
│    status dashboard                         │
└────────────────────┬────────────────────────┘
                     │ (JSON-RPC over ipc)
┌────────────────────▼────────────────────────┐
│   Rust Host Orchestrator (Main Process)     │
│  - Session lifecycle, approval store        │
│  - Task DAG scheduler, worker health        │
│  - IPC command dispatcher, backpressure     │
└──┬───┬───┬────────────────────┬─────────────┘
   │   │   │                    │
┌──▼─┐│   │                    │ (child processes)
│W1  ││   │                    │
└────┘│   │                    │
   ┌──▼─┐│   │                 │
   │W2  ││   │                 │
   └────┘│   │                 │
   ┌───────▼─┐│                 │
   │   W3    ││                 │
   └─────────┘│ (ACP/API adapters)
      ┌────────▼──┐
      │   W4      │
      │ (Executor)│
      └───────────┘
```
- **Host**: Session state, approval store, task DAG, leader logic
- **Workers**: Stateless; pull tasks from host; report via IPC; exit on cancel/error
- **v1 Isolation**: OS process boundary, workspace path constraints, command allowlist, ulimit/rlimit

### 4.2 IPC Contract
```rust
// Command: Host → Worker
struct WorkerCommand {
    id: u64,  // task ID
    task_type: String,  // "agent_init", "agent_stream", etc.
    payload: serde_json::Value,  // task-specific args
    deadline: Option<i64>,  // Unix millis; worker cancels if exceeded
}

// Event: Worker → Host
struct WorkerEvent {
    task_id: u64,
    event_type: String,  // "result", "error", "progress", "cancelled"
    data: serde_json::Value,
    ts: i64,  // emission timestamp
}

// Backpressure: Host sends "pause" if queue > threshold (100 tasks)
// Workers respect deadline; host retries on timeout.
```

### 4.3 Agent Contract
See `AGENT_CONTRACT.md` (P0 deliverable #3).

---

## 5. GUARDRAILS & POLICY ENGINE (v1)

**Default Posture**: Balanced (read + workspace write OK; shell/network/credentials require approval)

| Action Class | Default | Gate | Scope |
|--------------|---------|------|-------|
| **read** | ALLOW | None | All files except secrets (redacted before injection). Env vars scanned for secrets, redacted. |
| **write** | ALLOW | None | Workspace root + .gitignore'd subdirs. System dirs blocked. |
| **shell** | BLOCK | Per-session approval | Command filter: only whitelisted tools (git, cargo, node, python, etc.). No eval/exec. |
| **network** | BLOCK | Per-session approval | Separate from shell. Only HTTP/SSH. DNS filtering (no C2 domains). Optional Burp proxy. |
| **credentials** | SCOPED | None | Agent reads only creds for its assigned provider (no cross-agent leaks). |
| **tool** (LSP, debugger) | ALLOW (LSP only) | None | LSP server invocations cached + rate-limited. Custom tools require per-tool approval. |

**Approval Store**: Session-level cache; session ID → approvals map. User can revoke per-session.

---

## 6. PERFORMANCE REQUIREMENTS (Hard Gates)

| Metric | Target | Measurement | Gate |
|--------|--------|-------------|------|
| **Startup Time** | <1s | Cold start (no cache); warm (with DB cached) | Block release if violated. |
| **Typing Latency** | <5ms p95 | UI frame time measurement; agents actively running. | P6 validation; block if >5ms. |
| **Idle Memory** | <150MB | Measure after 1h idle, no active swarms. Exclude OS overhead. | Block if >200MB (20% margin). |
| **Agent Time-to-First-Token** | <100ms | Measure across 3 primary adapters (Gemini, OpenCode, Claude Code). | Block if >150ms. |
| **Task Handoff Overhead** | <50ms | Leader dispatch → worker ack. Exclude adapter execution. | Block if >100ms. |
| **Graceful Degradation** | 10+ concurrent tasks | Memory stays <250MB, no missed cancellations, all results collected. | E2E test in P6. |

---

## 7. v1 EXCLUSIONS (Out of Scope)

- Collaborative editing (mutex per file, single editor per workspace)
- Full LSP (adapter-based delegation only)
- Plugin SDK (no extension API in v1)
- Custom project templates
- Cloud backup / team sync
- GPU agent dispatch
- Workflow DSL / DAG UI
- Live debugging (attach only, no breakpoint UI)

---

## 8. DEPENDENCIES & KNOWN RISKS

### 8.1 External Dependencies
- **Tauri 2.x** (UI framework) — May introduce breaking changes. Mitigate: pin version, test on each upgrade.
- **ACP CLI tools** (Gemini, OpenCode, etc.) — Subject to provider EOL. Mitigate: fallback routing, version pinning.
- **SQLite** — Single-writer limit. Mitigate: write-ahead logging, batch inserts, read replicas (v1.1).

### 8.2 Technical Risks
| Risk | Mitigation | Owner |
|------|-----------|-------|
| Rust learning curve delays core runtime | Pair programming with Rust expert; prioritize patterns from CLIonGUI. | Mathi |
| IPC serialization bottleneck | Use bincode (faster than JSON) for hot paths; profile in P2. | P1 owner |
| Leader crash loses session state | Persist approvals + task DAG to SQLite; replay on restart. | P1 owner |
| Agent adapters flake under load | Implement retry + jitter at task level; monitor per-adapter success rate. | P3 owner |
| Guardrails bypass via shell redirection | Allowlist only commands, not eval. Sandbox shell (jq, not bash). | P4 owner |

---

## 9. VERIFICATION GATES (P0 → P1 Handoff)

Before Phase 1 begins, sign off on:
- [ ] **ADR Review**: Tech lead reviews and approves all 9 sections.
- [ ] **SLO Contract**: Measurement harness drafted; gates confirmed measurable.
- [ ] **Agent Contract**: Rust trait outline matches spec; all 5 methods (init, stream, cancel, error, cleanup) defined.
- [ ] **Dependency Check**: Rust 1.70+, Tauri 2.0+, SQLite 3.40+ installed and working.
- [ ] **Risk Register**: Known risks logged in ticket system; mitigation plans linked.

---

## 10. REFERENCES

- **Plan**: `plan-mathiV1AgentNativeIde.prompt.md` (7-phase execution plan)
- **Agent Principles**: `AGENTS.md` (Architecture principles, multi-agent patterns)
- **SLO Contract**: `SLO_CONTRACT_P0.md` (Quantified performance gates)
- **Agent Contract**: `AGENT_CONTRACT.md` (Rust trait definition, adapter shape)
- **Pattern Reuse**: CLIonGUI `src/process/team/`, `src/process/agent/acp/`

---

## Sign-Off

**Approved By**: Mathi  
**Date**: 2026-04-27  
**Version**: v1.0  

**Locked Decisions**: All 8 sections are locked for v1 execution. Changes require Mathi approval + changelog update.
