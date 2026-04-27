## Plan: Mathi v1 Agent-Native IDE

Build a standalone, non-Electron IDE using Rust core + Tauri UI with a host orchestrator and sandboxed workers, optimized for performance, bounded multi-agent execution, and local-first privacy. Reuse proven orchestration ideas from existing references (team session, approval cache, ACP/API bridge patterns), but implement a clean-room architecture focused on v1 scope and 4-8 week delivery.

**Steps**
1. Phase 0 - Foundation and constraints lock (blocks all other phases)
1. Finalize v1 architecture decision record: standalone app, Rust + Tauri, Windows/macOS target, hybrid LSP strategy, host orchestrator + worker isolation, balanced guardrails, no plugin marketplace, no cloud sync.
1. Define measurable SLO contract for v1 (startup, typing latency, idle RAM, scheduling latency, cancellation latency) with pass/fail thresholds and profiling instrumentation requirements.
1. Freeze v1 out-of-scope list and success gate to prevent scope creep.

2. Phase 1 - Core runtime and process topology
1. Implement process model with strict separation: UI shell, orchestrator service, worker pool (agent workers, tool workers, terminal worker), language services, and telemetry/profiler channel.
1. Add non-blocking IPC bus with bounded queues, backpressure, and cancellation propagation from UI to orchestrator to worker.
1. Implement workspace-scoped capability context passed to all workers (filesystem scope, network policy, auth scope, memory budget, time budget).
1. Add crash containment and supervised restart strategy so worker failures do not degrade editor responsiveness.

3. Phase 2 - Editor core (minimal but production-usable)
1. Build editor UX baseline: tabs, split panes, file tree, global search/replace, integrated terminal, Git basics (status/diff/commit).
1. Integrate LSP via reuse-first strategy (JS/TS or Python first), ensure async LSP calls and debounce policies to keep typing path under budget.
1. Add file indexing/watch pipeline with incremental updates and background scheduling only; enforce zero UI-thread blocking.
1. Introduce startup/lazy-load policy: cold path minimal load, all heavy systems deferred (agent runtime, provider metadata, deep indexing).

4. Phase 3 - Agent platform (single agent to swarm)
1. Define unified agent contract for both ACP CLI agents and API-native agents: initialize, send, stream, cancel, timeout, usage metrics, structured errors.
1. Implement ACP adapter layer (priority order: Gemini CLI, OpenCode, Copilot CLI, Claude Code, Codex CLI) with health checks, warm-pool strategy, and strict idle reclaim.
1. Implement API adapter layer (priority order: NVIDIA NIM, Cerebras, OpenCode, Mistral, OpenRouter, local Ollama, Anthropic) with retries, provider-specific rate-limit policies, and fallback routing.
1. Add orchestrator scheduler with bounded concurrency, queue priorities, per-task token/memory/time budgets, and hard cancellation semantics.
1. Implement 3 swarm templates for v1: Code Fix, Refactor, Research; use leader-worker only (no peer mesh), immutable worker templates at spawn-time.

5. Phase 4 - Guardrails and sandbox (v1 + hardening path)
1. Implement policy engine with action classes (read, write, shell, network, credential, tool), allow/deny rules, and approval hooks.
1. Apply balanced default mode: workspace writes allowed, shell/network/sensitive operations require explicit approval; destructive operations always require confirmation.
1. Enforce worker isolation in v1 via process boundaries, workspace path confinement, command filtering, timeout/kill controls, and per-agent credential scoping.
1. Add v1.1 hardening plan for full container enforcement (Docker/Podman) while preserving same policy interface.

6. Phase 5 - Auth, secrets, and memory system
1. Implement local credential vault integration (OS keychain/encrypted local store) for API keys and OAuth tokens with per-agent grants.
1. Build memory layers: session ephemeral memory, persistent user memory, workspace memory (opt-in sharing), external vector backend optional adapter.
1. Enforce memory discipline: structured summaries only, retrieval-gated injection, top-K limits, relevance thresholding, TTL deletion, and confidence filtering.
1. Implement privacy controls: local-only default, PII/secret redaction before log persistence, retention policy with hard-delete scheduler.

7. Phase 6 - Observability, verification, and packaging
1. Add observability dashboard for per-agent CPU/RAM, queue depth, slow operation alarms (>100ms), and startup timeline breakdown.
1. Run performance stabilization loops against SLOs; tune lazy loading, process pooling, and scheduler fairness.
1. Execute end-to-end acceptance suite for v1 success gate (open project -> edit -> terminal run -> agent action -> git commit) under 30-60 minute stability run.
1. Package reproducible installers for Windows/macOS with first-run onboarding (<15 minutes to productive state).

**Parallelism and dependencies**
1. Phase 1 blocks all phases.
1. Phase 2 can run in parallel with early Phase 3 adapter scaffolding after IPC contract is stable.
1. Phase 4 can begin once Phase 3 unified contract exists; policy checks become mandatory middleware before feature-complete status.
1. Phase 5 auth/secrets can run in parallel with Phase 3 provider adapters once capability model is defined.
1. Phase 6 starts once minimum slices of Phases 2-5 are integrated.

**Relevant files (reference architecture to reuse patterns, not direct fork targets)**
- CLIonGUI/src/process/team/TeamSession.ts — session coordinator boundaries, MCP-backed team coordination, wake-after-delivery semantics.
- CLIonGUI/src/process/team/TeammateManager.ts — wake lifecycle, timeout guards, status machine, team event plumbing.
- CLIonGUI/src/process/team/TaskManager.ts — dependency-aware task graph and unblock logic.
- CLIonGUI/src/process/team/TeamSessionService.ts — session lifecycle, model/provider resolution, per-team locking.
- CLIonGUI/src/process/agent/acp/index.ts — ACP lifecycle and approval integration points.
- CLIonGUI/src/process/agent/acp/ApprovalStore.ts — session-level approval cache model.
- CLIonGUI/src/common/config/storage.ts — ACP config, idle timeout, sandbox mode, MCP config schema references.
- CLIonGUI/src/common/adapter/ipcBridge.ts — command/event bridge and service surface patterns.
- CLIonGUI/src/process/bridge/acpConversationBridge.ts — backend-agnostic ACP bridge concepts.
- CLIonGUI/src/process/bridge/authBridge.ts — auth bridge touchpoints from prior repo notes.
- CLIonGUI/src/process/agent/acp/AcpDetector.ts — CLI backend detection concepts.
- CLIonGUI/src/renderer/hooks/agent/useModelProviderList.ts — model/provider presentation shape.
- CLIonGUI/src/renderer/pages/settings/AgentSettings/LocalAgents.tsx — local agent settings UX reference.
- officecli/README.md — high-performance CLI interaction and resident-mode UX ideas.
- officecli/SKILL.md — command help, capability layering, and operational guardrail patterns.

**Verification**
1. Performance validation:
1. Cold startup <1s target, warm startup <300ms, typing latency p95 <5ms, autocomplete p95 <50ms, indexed project search <100ms.
1. Idle RAM <150MB without agents, <300MB with agent runtime loaded; per-agent RAM budget alarms at 100MB.
1. Resilience validation:
1. Kill/timeout/cancel flows complete within 100ms response to user cancellation.
1. Single worker crash does not freeze or crash editor shell.
1. Security/privacy validation:
1. Shell/network operations always pass policy and approval middleware in balanced mode.
1. Memory/log redaction test corpus confirms no raw secrets/PII persistence.
1. Data retention jobs enforce TTL hard delete.
1. Functional acceptance:
1. Fresh install to productive first-run <15 minutes.
1. End-to-end user workflow passes on Windows and macOS: open project, edit, run terminal command, run agent task, create git commit.

**Decisions**
- Included scope:
  - Standalone Rust + Tauri app, Windows/macOS, editor core + terminal + Git basics + LSP integration.
  - Agent-first runtime with ACP and API providers, streaming, cancellation, bounded swarm (leader-worker), 2-3 templates.
  - Local-first privacy, no cloud sync by default, memory controls, redaction, TTL deletion.
- Excluded scope (v1):
  - Plugin marketplace/ecosystem, real-time collaboration, advanced training/fine-tuning infra, enterprise SSO, self-hosted gateway, full container hardening on day 1.
- Deferred to v1.1:
  - Strong container-first sandbox on both OSes, expanded provider matrix, deeper language coverage.

**Further considerations**
1. LSP language priority recommendation: start with TypeScript/JavaScript first for fastest validation of UX + agent workflow, add Python second after stability.
1. Provider recommendation: launch with 2 highly reliable defaults plus fallback chain; keep long-tail providers behind experimental flag until telemetry proves reliability.
1. Security hardening recommendation: keep policy interface stable now so container backend can be swapped in later without UX or API breakage.