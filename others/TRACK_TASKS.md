# TRACK_TASKS

Last Updated: 2026-04-27
Owner: Mathi

## Status Legend
- NOT_STARTED
- IN_PROGRESS
- BLOCKED
- DONE

## Master Progress
| ID | Task | Status | Progress % | Owner | Start Date | Target Date | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| P0 | Foundation and constraints lock | DONE | 100 | Mathi | 2026-04-26 | 2026-04-27 | ✅ ADR, SLO contract, agent contract locked. Ready for P1. |
| P1 | Core runtime and process topology | DONE | 100 | Mathi | 2026-04-27 | 2026-05-04 | Rust runtime scaffold complete; orchestrator, IPC, worker, policy, telemetry, SQLite persistence, and capability context are in place. |
| P2 | Editor core MVP | DONE | 100 | Mathi | 2026-04-27 | 2026-05-11 | Tauri editor shell delivered with tree, tabs, split panes, search/replace, terminal panel, git basics panel, and diagnostics. |
| P3 | Agent platform (ACP + API + swarm) | DONE | 100 | Mathi | 2026-04-27 | 2026-05-18 | Unified contract, ACP/API adapters, fallback scheduler, and 3 swarm templates implemented with passing smoke tests. |
| P4 | Guardrails and sandbox | IN_PROGRESS | 45 | Mathi | 2026-04-27 | 2026-05-25 | Core policy engine and approval cache implemented; dispatch now enforces approvals and sandbox command checks. |
| P5 | Auth, secrets, and memory | NOT_STARTED | 0 | Mathi | - | 2026-06-01 | Local vault, memory layers, redaction, TTL retention. |
| P6 | Observability, verification, packaging | NOT_STARTED | 0 | Mathi | - | 2026-06-15 | Telemetry, SLO checks, E2E acceptance, installers. |

## Current Sprint Tasks (P0 COMPLETE → Ready for P1)
| ID | Task | Status | Progress % | Dependencies | Notes |
| --- | --- | --- | --- | --- | --- |
| S1 | Create architecture decision record (ADR) | DONE | 100 | P0 | ✅ ADR_P0_Architecture.md finalized. 10 sections locked (stack, topology, decisions, gates). |
| S2 | Define SLO test harness | DONE | 100 | P0 | ✅ SLO_CONTRACT_P0.md: 7 SLOs quantified, measurement methods, acceptance gates. |
| S3 | Scaffold orchestrator and worker IPC | DONE | 100 | P1 | ✅ Rust runtime scaffold created in src-tauri; bounded IPC, worker lifecycle, SQLite state, and smoke tests compile cleanly. |
| S4 | Implement agent platform core | DONE | 100 | P3 | ✅ Added unified adapter contract, ACP/API adapters, fallback scheduler, and swarm coordinator templates. |
| S5 | Implement guardrails core policy engine | DONE | 100 | P4 | ✅ Added session approval cache, policy outcomes, dispatch enforcement, and sandbox command filtering with smoke tests. |

## Change Log
- 2026-04-26: Tracker created with initial tasks and baseline progress.
- 2026-04-26: Phase 0 started; ADR work is in progress and foundation tasks are now active.
- 2026-04-27: **P0 COMPLETE**. ADR, SLO contract, agent contract all finalized and locked.
  - ADR_P0_Architecture.md: 10-section decision record (stack, topology, guardrails, risks, gates)
  - SLO_CONTRACT_P0.md: 7 hard SLOs with measurement plans (startup, typing, memory, TTFT, handoff, leaks, degradation)
  - AGENT_CONTRACT.md: Rust trait definition with 4 methods (init, execute, health_check, shutdown), error handling, testing strategy
  - .gitignore: Rust/Tauri project template
  - Rust 1.95.0 verified; Tauri/SQLite available
- 2026-04-27: **P1 STARTED**. Created buildable Rust runtime scaffold and validated it with `cargo check` + `cargo test`.
  - `src-tauri/src/orchestrator.rs`: host orchestrator scaffold
  - `src-tauri/src/worker.rs`: worker lifecycle, cancellation, progress events
  - `src-tauri/src/ipc.rs`: bounded command/event bridge
  - `src-tauri/src/policy.rs`: balanced approval defaults
  - `src-tauri/src/telemetry.rs`: measurement hook helper
- 2026-04-27: **P1 COMPLETE**. Added SQLite-backed runtime state and capability context; smoke tests now cover persistence and guardrail defaults.
  - `src-tauri/src/db.rs`: bundled SQLite database wrapper
  - `src-tauri/src/runtime_context.rs`: capability context and guardrail defaults
  - `src-tauri/tests/runtime_smoke.rs`: 4 passing smoke tests (policy, DB, context, bootstrap)
- 2026-04-27: **P2 COMPLETE**. Added visible Tauri editor shell and validated launch path.
  - `src-tauri/tauri.conf.json`: app window and dev URL config
  - `src-tauri/src/main.rs`: Tauri setup + local shell server + orchestrator bootstrap spawn
  - `src-tauri/dist/index.html`: interactive editor shell (file tree, tabs, split panes, search/replace, terminal panel, git basics panel, diagnostics)
  - Validation: `cargo check`, `cargo test --test runtime_smoke`, and `cargo run` all succeeded without panic
- 2026-04-27: **P3 COMPLETE**. Implemented agent platform core with ACP/API unification and swarm templates.
  - `src-tauri/src/agent_platform/contract.rs`: unified adapter trait + provider/kind/exec models
  - `src-tauri/src/agent_platform/adapters.rs`: ACP CLI adapter, API-native adapter, failover test adapter
  - `src-tauri/src/agent_platform/scheduler.rs`: provider-priority execution with fallback routing
  - `src-tauri/src/agent_platform/swarm.rs`: CodeFix/Refactor/Research template coordination
  - `src-tauri/tests/agent_platform_smoke.rs`: fallback + adapter + template coverage
  - Validation: `cargo check`, `cargo test --test runtime_smoke`, `cargo test --test agent_platform_smoke`
- 2026-04-27: **P4 STARTED**. Implemented guardrails core policy engine and sandbox checks.
  - `src-tauri/src/policy.rs`: PolicyEngine, ApprovalStore, PolicyOutcome, sandbox command/network gating
  - `src-tauri/src/orchestrator.rs`: dispatch now enforces approval-required/denied outcomes
  - `src-tauri/src/types.rs`: RuntimeError variants for approval/policy denials
  - `src-tauri/tests/policy_engine_smoke.rs`: default posture, approval flow, unsafe command denial, workspace write scope checks
  - Validation: `cargo check`, `cargo test --test runtime_smoke`, `cargo test --test agent_platform_smoke`, `cargo test --test policy_engine_smoke`

## Update Rule
When a task changes, update:
1. Status
2. Progress %
3. Notes
4. Last Updated date at top
