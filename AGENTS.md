## IDENTITY & CONTEXT

- You are an expert Software Architect and Systems Engineer.
- Goal: Zero-defect, root-cause-oriented engineering for bugs; test-driven engineering for new features. Think carefully; no need to rush.
- Code: Write the simplest code possible. Keep the codebase minimal and modular.
- **Project**: Standalone agent-native IDE (Rust + Tauri). Leader-worker swarm. 4–8 week delivery to MVP. See [plan-mathiV1AgentNativeIde.prompt.md](../plan/plan-mathiV1AgentNativeIde.prompt.md).

## PROJECT SCOPE & CONSTRAINTS

**What We're Building:**
- Multi-agent IDE with Rust core + Tauri UI (non-Electron, single-process-free design)
- Leader-worker swarm model; 3 templates (Code Architect, Refactor, Research) for v1
- Unified ACP CLI + API-native agent adapters with priority fallback routing
- Balanced guardrails by default (read OK, write to workspace OK, shell/network/credentials require approval)
- Process isolation v1 (containers deferred to v1.1)
- Local-first privacy: no cloud sync, redaction, TTL deletion

**Hard Performance SLOs (gate for acceptance):**
- Startup: <1s
- Editor typing latency: <5ms (p95)
- Idle memory: <150MB (no active swarms, measured after 1h idle)
- Agent response streaming: <100ms time-to-first-token
- Swarm orchestration overhead: <50ms per task handoff

**Scope Decisions (Locked for v1):**
- **NOT a VS Code fork** – standalone app with independent architecture
- **Standalone, no cloud sync** – all data local, encrypted at rest
- **Process isolation only in v1** – containers deferred to v1.1
- **No peer mesh** – leader-worker swarm only; no agent-to-agent direct messaging
- **Windows + macOS only in v1** – Linux deferred to v1.1
- **Out of scope for v1:** Full LSP, collaborative editing, multi-workspace, custom plugin SDK

## MULTI-AGENT PRINCIPLES

- **Leader-Worker Asymmetry**: Leader schedules tasks, coordinates state, breaks ties. Workers execute and report; no direct worker-to-worker communication.
- **Swarm Templates**: Pre-configured team shapes (Code Fix: 2–4 agents; Refactor: 3–6; Research: 2–3). User selects; no dynamic team composition in v1.
- **Approval Gates**: Wrap shell, network, credential actions in approval layer. Session-level approval cache to avoid UX spam (see `ApprovalStore` pattern).
- **Task Coordination**: Dependency-aware task graph. Workers unblock on prereqs. Timeouts per task (60s default, configurable per agent).
- **Crash Containment**: Single worker crash doesn't kill swarm. Supervised restart with exponential backoff. Leader monitors health.
- **Backpressure**: Bounded async queues on IPC. If queue fills, slow down task dispatch (don't drop tasks).
- **Cancellation**: User cancels → leader sends cancel signal to all workers → workers finish in-flight, clean up, exit gracefully.

## PHASE CONTEXT

**Current Phase:** P0 (Foundation & Constraints Lock)
- **Status:** IN_PROGRESS; S1 (ADR finalization) active
- **Track progress:** See [TRACK_TASKS.md](../others/TRACK_TASKS.md)

**Phase Progression:**
- **P0** → Architecture Decision Record, SLO contract, agent contract
- **P1** → Process topology, IPC, worker pool, crash containment
- **P2** → Editor core (tabs, split panes, file tree, terminal, Git)
- **P3** → Agent platform (adapters, scheduler, swarm templates)
- **P4** → Guardrails, sandbox, approval gates
- **P5** → Auth, credential vault, memory system (ephemeral + persistent + vector)
- **P6** → Observability, SLO instrumentation, packaging, performance validation

**Critical Path Items (no parallelization gains):**
- P0 → P1 (need architecture locked before building runtime)
- P1 → P3 (need IPC before agents can run)
- P3 → P4 (need agents before guardrails make sense)

## ARCHITECTURE PRINCIPLES (see PLAN.md)

- **Shared utilities**: Put shared Anthropic protocol logic in neutral `core/anthropic/` modules. Do not have one provider import from another provider's utils.
- **DRY**: Extract shared base classes to eliminate duplication. Prefer composition over copy-paste.
- **Encapsulation**: Use accessor methods for internal state (e.g. `set_current_task()`), not direct `_attribute` assignment from outside.
- **Provider-specific config**: Keep provider-specific fields (e.g. `nim_settings`) in provider constructors, not in the base `ProviderConfig`.
- **Dead code**: Remove unused code, legacy systems, and hardcoded values. Use settings/config instead of literals (e.g. `settings.provider_type` not `"nvidia_nim"`).
- **Performance**: Use list accumulation for strings (not `+=` in loops), cache env vars at init, prefer iterative over recursive when stack depth matters.
- **Platform-agnostic naming**: Use generic names (e.g. `PLATFORM_EDIT`) not platform-specific ones (e.g. `TELEGRAM_EDIT`) in shared code.
- **No type ignores**: Do not add `# type: ignore` or `# ty: ignore`. Fix the underlying type issue.
- **Complete migrations**: When moving modules, update imports to the new owner and remove old compatibility shims in the same change unless preserving a published interface is explicitly required.
- **Maximum Test Coverage**: There should be maximum test coverage for everything, preferably live smoke test coverage to catch bugs early

**Multi-Agent Specific:**
- **Adapter Isolation**: Each adapter (ACP/API provider) runs in its own worker process. Never call adapters from the main thread.
- **Protocol Unification**: ACP CLI and API-native adapters present the same `AgentContract` (init, stream, cancel, error, cleanup). No adapter-specific quirks in orchestrator logic.
- **Fallback Routing**: If primary adapter fails, orchestrator tries next in priority list. No retry logic in adapters; orchestrator retries at task level.
- **Worker State Machine**: Workers are IDLE → SCHEDULED → RUNNING → DONE/FAILED/CANCELLED. Transitions are strict; no skipping states.
- **Session Isolation**: Each swarm session has isolated approval store, credential scope, memory context. No cross-session leaks.
- **No Provider Lock-In**: Adapters are swappable via `AgentContract`; fallback routing prevents single-vendor dependency.

## COGNITIVE WORKFLOW

1. **ANALYZE**: Read relevant files. Do not guess.
2. **PLAN**: Map out the logic. Identify root cause or required changes. Order changes by dependency.
3. **EXECUTE**: Fix the cause, not the symptom. Execute incrementally with clear commits.
4. **VERIFY**: Run ci checks and relevant smoke tests. Confirm the fix via logs or output.
5. **SPECIFICITY**: Do exactly as much as asked; nothing more, nothing less.
6. **PROPAGATION**: Changes impact multiple files; propagate updates correctly.

## SUMMARY STANDARDS

- Summaries must be technical and granular.
- Include: [Files Changed], [Logic Altered], [Verification Method], [Residual Risks] (if no residual risks then say none).

## TOOLS

- Prefer built-in tools (grep, read_file, etc.) over manual workflows. Check tool availability before use.

## REFERENCE PATTERNS (Pattern Reuse from Existing Repos)

**Do not fork these files; study and adapt their patterns to Rust context:**

- **`CLIonGUI/src/process/team/TeamSession.ts`** — Session lifecycle, coordination boundaries, wake-after-delivery semantics. Study for swarm session model.
- **`CLIonGUI/src/process/team/TeammateManager.ts`** — Agent state machine (IDLE, ACTIVE, WAITING), wake cycle, timeout guards, event bus. Adapt state transitions to Rust async.
- **`CLIonGUI/src/process/team/TaskManager.ts`** — Dependency-aware task graph, unblock logic, task execution order. Critical for orchestrator task scheduler.
- **`CLIonGUI/src/process/agent/acp/ApprovalStore.ts`** — Session-level approval cache model. Prevents UX spam; cache key by action type + agent ID.
- **`CLIonGUI/src/common/adapter/ipcBridge.ts`** — Command/event bridge patterns for IPC. Study request/response correlation, timeouts, error propagation.
- **`CLIonGUI/src/common/config/storage.ts`** — Config schema for ACP provider, idle timeouts, sandbox mode booleans. Reference for Rust settings struct design.
- **`officecli/SKILL.md`** — Operational guardrails, CLI interface patterns, action classification (read/write/shell/network/credential). Adapt to swarm policy engine.

**Why Study These?**
- Proven patterns under production load
- Avoid reinventing multi-agent coordination logic
- Adapt, don't copy-paste; Rust async/await differs from TS promises

## CRITICAL DECISIONS (Locked; Don't Revisit Without Sign-Off)

| Decision | Rationale | Trade-off |
|----------|-----------|-----------|
| **Rust + Tauri, not Electron** | Performance SLOs (startup <1s, idle <150MB). Electron baseline ~200MB. | No JS/TS runtime; extra build complexity. |
| **Leader-worker, no peer mesh** | Simpler coordination, no split-brain. Peer mesh requires consensus (Raft/Paxos), overkill for v1. | Single leader is bottleneck; mitigate with health monitoring + quick failover. |
| **Balanced guardrails (not strict)** | Read + workspace write OK by default. Improves UX velocity. | Risk: user accidentally grants agent shell access. Mitigate: explicit confirmation, audit log. |
| **Process isolation v1, containers v1.1** | Deliver agent platform sooner. Containers add 2–3 weeks. | Process isolation weaker than containers but sufficient for v1 guardrails. |
| **Approval store (not per-action modal)** | Cache approval decisions per session. One "grant shell for this session" → all agents get it. | Reduces UX friction from spam approvals. Risk: approval scope too broad. Mitigate: revokable per-session, clear UI. |
| **3 swarm templates (not dynamic)** | Fixed shapes = predictable performance + easy testing. Dynamic composition = unbounded complexity. | Less flexible; users can't compose custom teams. v2 feature if demand arises. |

## GUARDRAILS DEFAULTS (Approval Policy)

**Default Posture: Balanced (not Strict, not Permissive)**

| Action Class | Default | Requires Approval? | Notes |
|--------------|---------|-------------------|-------|
| **read** (files, env) | ALLOW | No | Assume agent needs context. Redact secrets before injection. |
| **write** (workspace files) | ALLOW | No | Agents can edit files. Undo via Git. |
| **shell** (exec commands) | BLOCK | Yes | Per-session approval. Once granted, all agents can shell. |
| **network** (HTTP/SSH) | BLOCK | Yes | Separate approval. Risk: data exfil. |
| **credentials** (read from vault) | ALLOW (scoped) | No | Agent gets creds for its assigned provider only. |
| **tool** (invoke external tool) | ALLOW (LSP, debugger) | No | LSP/debugger invocations pre-approved. Custom tools require per-tool approval. |

## PERFORMANCE VALIDATION CHECKLIST

Before shipping:
- [ ] Startup time <1s (measure cold start, warm cache)
- [ ] Typing latency <5ms p95 (measure UI frame time with agents active)
- [ ] Idle memory <150MB (measure after 1h idle, no active swarms)
- [ ] Agent response <100ms time-to-first-token (measure across all 3 primary adapters)
- [ ] Task handoff <50ms overhead (measure leader->worker dispatch + ack cycle)
- [ ] No memory leaks in long-running swarms (measure memory over 24h test)
- [ ] Graceful degradation under load (measure with 10+ concurrent tasks)