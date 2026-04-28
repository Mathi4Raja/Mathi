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
| P3 | Agent platform (ACP + API + swarm) | DONE | 100 | Mathi | 2026-04-27 | 2026-05-18 | Production contract/scheduler/swarm; ACP and API adapters now execute real process/HTTP paths; fallback helper is test-only; passing smoke tests. |
| P4 | Guardrails and sandbox | DONE | 100 | Mathi | 2026-04-27 | 2026-05-25 | Policy engine, approval revocation, audit trail persistence, network allowlist gating, and dispatch timeout/path scope enforcement completed. |
| P5 | Auth, secrets, and memory | DONE | 100 | Mathi | 2026-04-27 | 2026-06-01 | Local encrypted vault + memory layers are fully integrated into orchestrator dispatch with credential scope enforcement, memory boundary gating, retention/audit hooks, and passing smoke tests. |
| P6 | Observability, verification, packaging | DONE | 100 | Mathi | 2026-04-27 | 2026-06-15 | Verification, release validation probes, packaging, checksum publication, and auto-generated release notes are in place for v1 (Windows/macOS). |

## Current Sprint Tasks (P0 COMPLETE → Ready for P1)
| ID | Task | Status | Progress % | Dependencies | Notes |
| --- | --- | --- | --- | --- | --- |
| S1 | Create architecture decision record (ADR) | DONE | 100 | P0 | ✅ ADR_P0_Architecture.md finalized. 10 sections locked (stack, topology, decisions, gates). |
| S2 | Define SLO test harness | DONE | 100 | P0 | ✅ SLO_CONTRACT_P0.md: 7 SLOs quantified, measurement methods, acceptance gates. |
| S3 | Scaffold orchestrator and worker IPC | DONE | 100 | P1 | ✅ Rust runtime scaffold created in src-tauri; bounded IPC, worker lifecycle, SQLite state, and smoke tests compile cleanly. |
| S4 | Implement agent platform core | DONE | 100 | P3 | ✅ Added unified adapter contract, production ACP/API adapters, fallback scheduler, and swarm coordinator templates; mock fail adapter moved to tests. |
| S5 | Implement guardrails core policy engine | DONE | 100 | P4 | ✅ Added session approval cache, policy outcomes, dispatch enforcement, and sandbox command filtering with smoke tests. |
| S6 | Complete guardrails enforcement and auditability | DONE | 100 | P4 | ✅ Added approval revocation, policy audit persistence, network allowlist checks, and max-timeout/path-scope constraints. |
| S7 | Implement P5 core vault and memory layers | DONE | 100 | P5 | ✅ Added encrypted local vault, redaction, scoped memory service, and TTL retention cleanup with dedicated smoke tests. |
| S8 | Integrate P5 into orchestration paths | DONE | 100 | P5 | ✅ Dispatch now enforces credential provider scope, memory scope boundaries, and retention cleanup/audit behavior with new runtime smoke coverage. |
| S9 | Establish CI verification quality gates | DONE | 100 | P6 | ✅ Added CI workflow for tests + coverage floor enforcement using cargo-llvm-cov. |
| S10 | Add local coverage gate scripts | DONE | 100 | P6 | ✅ Added PowerShell and bash scripts to enforce 94% total coverage locally. |
| S11 | Instrument SLO measurement harness | DONE | 100 | P6 | ✅ Added runnable probes for startup, handoff, TTFT, and idle-memory with threshold-based exit codes. |
| S12 | Packaging pipeline scaffolding | DONE | 100 | P6 | ✅ Added release workflow, local packaging script, and enabled Tauri bundles for installer generation. |
| S13 | Artifact publication workflow | DONE | 100 | P6 | ✅ Added tagged release publication with auto-generated notes and SHA256 checksums in CI. |
| S14 | Pre-release smoke validation gate | DONE | 100 | P6 | ✅ Added consolidated pre-release validation script and release validation probe in CI. |

## v1.1 Backlog (Linux Rollout - Deferred from v1)
| ID | Task | Status | Progress % | Dependencies | Notes |
| --- | --- | --- | --- | --- | --- |
| V11-L1 | Add Linux CI build matrix | NOT_STARTED | 0 | P6 | Add ubuntu runner jobs for test + package workflow while keeping v1 release scope unchanged. |
| V11-L2 | Add Linux packaging target | NOT_STARTED | 0 | V11-L1 | Produce Linux artifacts (AppImage and/or .deb) via Tauri bundle targets. |
| V11-L3 | Linux runtime compatibility pass | NOT_STARTED | 0 | V11-L1 | Validate process model, sandbox policy defaults, and worker lifecycle behavior on Linux. |
| V11-L4 | Linux smoke tests and SLO probes | NOT_STARTED | 0 | V11-L3 | Run startup/TTFT/handoff/idle-memory probes on Linux and capture baseline thresholds. |
| V11-L5 | Linux release publication gates | NOT_STARTED | 0 | V11-L2, V11-L4 | Publish Linux artifacts to GitHub Releases with checksums and release-note sections. |

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
- 2026-04-27: **P4 COMPLETE**. Finished remaining sandbox/approval controls and policy audit trail.
  - `src-tauri/src/runtime_context.rs`: network allowlist + max timeout settings
  - `src-tauri/src/db.rs`: `policy_audit` persistence and counters
  - `src-tauri/src/policy.rs`: approval revocation, deadline ceiling enforcement, context-driven network allowlist checks
  - `src-tauri/src/orchestrator.rs`: policy audit logging for allow/approval-needed/deny outcomes and approval grant/revoke APIs
  - `src-tauri/tests/runtime_smoke.rs`: orchestrator audit + revocation coverage
  - `src-tauri/tests/policy_engine_smoke.rs`: revocation, allowlist, and timeout policy tests
  - Validation: `cargo check`, `cargo test --test runtime_smoke` (6 pass), `cargo test --test policy_engine_smoke` (7 pass), `cargo test --test agent_platform_smoke` (4 pass)
- 2026-04-27: **P5 STARTED**. Implemented auth/memory core primitives.
  - `src-tauri/src/auth.rs`: local encrypted vault (AES-GCM) for provider secrets with revoke support
  - `src-tauri/src/memory.rs`: session/persistent/workspace memory service with TTL retention and cleanup
  - `src-tauri/src/redaction.rs`: redaction engine for bearer tokens, secret-like assignments, and emails
  - `src-tauri/src/db.rs`: `secrets_vault` and `memory_entries` schema + CRUD helpers
  - `src-tauri/tests/p5_memory_auth_smoke.rs`: vault, redaction, scope isolation, and TTL cleanup tests
  - Validation: `cargo check`, `cargo test --test runtime_smoke` (6 pass), `cargo test --test policy_engine_smoke` (7 pass), `cargo test --test agent_platform_smoke` (4 pass), `cargo test --test p5_memory_auth_smoke` (4 pass)
- 2026-04-27: **P5 COMPLETE**. Wired auth and memory into orchestrator runtime dispatch and validated end-to-end smoke coverage.
  - `src-tauri/src/orchestrator.rs`: added local vault + memory service integration, provider-scope credential checks, memory-scope gating (`session` default with opt-in `persistent/workspace`), retention cleanup hook, and policy audit logging for credential/memory decisions
  - `src-tauri/tests/runtime_smoke.rs`: added orchestrator credential scope denial and memory boundary/retention verification tests
  - Validation: `cargo check -p mathi-runtime`, `cargo test -p mathi-runtime --test runtime_smoke` (8 pass), `cargo test -p mathi-runtime --test policy_engine_smoke` (7 pass), `cargo test -p mathi-runtime --test p5_memory_auth_smoke` (4 pass), `cargo test -p mathi-runtime --test agent_platform_smoke` (4 pass)
- 2026-04-27: **P6 STARTED**. Added first verification gates for test and coverage enforcement.
  - `.github/workflows/quality-gates.yml`: CI job runs full runtime tests and fails when line coverage drops below 94%
  - `scripts/coverage_gate.ps1`: local PowerShell coverage gate with threshold enforcement and auto-install of cargo-llvm-cov
  - `scripts/coverage_gate.sh`: local bash coverage gate with threshold enforcement
  - `README.md`: quality gate usage instructions for local and CI verification
- 2026-04-27: **P6 UPDATE**. Added initial SLO probe execution path.
  - `src-tauri/src/bin/slo_probe.rs`: startup (<1s) and dispatch handoff p95 (<50ms) measurement probe with pass/fail exit code
  - `README.md`: added command to run the SLO probe locally
- 2026-04-27: **P6 UPDATE**. Added TTFT and idle-memory probes and validated them locally.
  - `src-tauri/src/bin/ttft_probe.rs`: TTFT p95 measurement from first stream chunk with 100ms SLO gate
  - `src-tauri/src/bin/idle_memory_probe.rs`: RSS memory check after idle period with 150MB SLO gate
  - Validation: `cargo run -p mathi-runtime --bin ttft_probe` (p95=54ms, PASS), `cargo run -p mathi-runtime --bin idle_memory_probe -- 3` (rss=23.63MB, PASS)
- 2026-04-27: **P6 UPDATE**. Added packaging workflow scaffolding for release artifacts.
  - `src-tauri/tauri.conf.json`: bundle activation enabled for installer generation
  - `.github/workflows/package.yml`: tag-driven Windows/macOS package workflow with runtime tests and Tauri build step
  - `scripts/package_release.ps1`: local packaging helper for debug/release bundle creation
- 2026-04-27: **P6 UPDATE**. Documented local packaging commands and relaxed workflow installation pinning.
  - `README.md`: added packaging command examples and CI release bundle note
  - `.github/workflows/package.yml`: installs current locked Tauri CLI instead of a brittle exact version pin
- 2026-04-27: **P6 COMPLETE**. Implemented tagged release publication with generated notes, checksums, and validation gate.
  - `.github/workflows/package.yml`: split into pre-release validation, package matrix (Windows/macOS), and publish-release jobs with GitHub Release creation
  - `scripts/pre_release_validation.sh`: runs smoke/integration suites and SLO probes before packaging
  - `scripts/release_notes.sh`: generates grouped release notes from conventional commits between tags
  - `src-tauri/src/bin/release_validation_probe.rs`: validates bootstrap/dispatch behavior and shell-approval guard behavior prior to release
  - `README.md`: documented release notes and tagged publish flow
- 2026-04-27: **v1.1 STAGED**. Added deferred Linux rollout backlog as a dedicated post-v1 track.
  - `others/TRACK_TASKS.md`: added V11-L1..V11-L5 tasks covering Linux CI, packaging, compatibility, SLO verification, and release publication gates

## Update Rule
When a task changes, update:
1. Status
2. Progress %
3. Notes
4. Last Updated date at top
