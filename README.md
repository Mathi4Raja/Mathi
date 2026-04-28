# Mathi Runtime Scaffold

Phase 0 is locked. This workspace now contains a minimal Rust/Tauri runtime scaffold for Phase 1:
- `src-tauri/src/orchestrator.rs`
- `src-tauri/src/worker.rs`
- `src-tauri/src/ipc.rs`
- `src-tauri/src/policy.rs`
- `src-tauri/src/telemetry.rs`
- `src-tauri/src/types.rs`

Build and validation will be expanded as P1 continues.

## Quality Gates (P6)

Phase 6 introduces explicit verification gates for tests and coverage.

Local validation:

- Run test suite: `cargo test -p mathi-runtime --all-targets`
- Run coverage gate (Windows PowerShell): `./scripts/coverage_gate.ps1`
- Run coverage gate (bash): `./scripts/coverage_gate.sh 94.0`

CI validation:

- GitHub Actions workflow: `.github/workflows/quality-gates.yml`
- Enforces line coverage floor of 94% via `cargo llvm-cov --fail-under-lines 94`

SLO probe (startup + handoff latency):

- Run probe: `cargo run -p mathi-runtime --bin slo_probe`
- Probe exits non-zero if startup is >= 1000ms or handoff p95 is >= 50ms.

TTFT probe:

- Run probe: `cargo run -p mathi-runtime --bin ttft_probe`
- Probe exits non-zero if TTFT p95 is >= 100ms.

Idle memory probe:

- Run probe: `cargo run -p mathi-runtime --bin idle_memory_probe -- 10`
- First argument is idle duration in seconds (for release validation, use 3600).
- Probe exits non-zero if RSS is >= 150MB.

Packaging:

- Local release bundle: `powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\package_release.ps1`
- Local debug bundle: `powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\package_release.ps1 -Profile debug`
- CI release bundles are produced by `.github/workflows/package.yml` on tag pushes.

Release publication and notes:

- Pre-release validation suite: `./scripts/pre_release_validation.sh`
- Release notes generation: `./scripts/release_notes.sh v0.1.0`
- Publish flow: push a tag such as `v0.1.0`; workflow builds Windows/macOS bundles, generates SHA256 checksums, generates release notes from conventional commits, and publishes a GitHub Release.
