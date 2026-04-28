#!/usr/bin/env bash
set -euo pipefail

echo "Running pre-release validation suite..."

cargo test -p mathi-runtime --test runtime_smoke
cargo test -p mathi-runtime --test policy_engine_smoke
cargo test -p mathi-runtime --test agent_platform_smoke
cargo test -p mathi-runtime --test p5_memory_auth_smoke

cargo run -p mathi-runtime --bin slo_probe
cargo run -p mathi-runtime --bin ttft_probe
cargo run -p mathi-runtime --bin idle_memory_probe -- 3
cargo run -p mathi-runtime --bin release_validation_probe

echo "Pre-release validation completed successfully."
