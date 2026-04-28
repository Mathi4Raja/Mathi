#!/usr/bin/env bash
set -euo pipefail

MINIMUM_COVERAGE="${1:-94.0}"

if ! command -v cargo >/dev/null 2>&1; then
  echo "Missing required command: cargo" >&2
  exit 1
fi

if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
  echo "Installing cargo-llvm-cov..."
  cargo install cargo-llvm-cov --locked
fi

echo "Running coverage collection..."
echo "Cleaning previous llvm-cov artifacts..."
cargo llvm-cov clean --workspace

OUTPUT="$(cargo llvm-cov --workspace --all-targets --summary-only)"

echo "$OUTPUT"
TOTAL_LINE="$(echo "$OUTPUT" | grep -E '^TOTAL[[:space:]]+[0-9]+[[:space:]]+[0-9]+[[:space:]]+[0-9]+\.[0-9]+%$' || true)"
if [[ -z "$TOTAL_LINE" ]]; then
  echo "Could not parse TOTAL coverage from output" >&2
  exit 1
fi

COVERAGE="$(echo "$TOTAL_LINE" | grep -Eo '[0-9]+\.[0-9]+%' | tr -d '%')"
if [[ -z "$COVERAGE" ]]; then
  echo "Could not parse coverage percentage" >&2
  exit 1
fi

awk -v cov="$COVERAGE" -v min="$MINIMUM_COVERAGE" 'BEGIN {
  printf("Coverage total: %.2f%%\n", cov);
  if (cov + 0 < min + 0) {
    printf("Coverage gate failed: %.2f%% < %.2f%%\n", cov, min);
    exit 1;
  }
  printf("Coverage gate passed: %.2f%% >= %.2f%%\n", cov, min);
}'
