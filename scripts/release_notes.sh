#!/usr/bin/env bash
set -euo pipefail

CURRENT_TAG="${1:-${GITHUB_REF_NAME:-}}"
if [[ -z "${CURRENT_TAG}" ]]; then
  echo "Usage: scripts/release_notes.sh <current-tag>" >&2
  exit 1
fi

PREVIOUS_TAG="$(git describe --tags --abbrev=0 "${CURRENT_TAG}^" 2>/dev/null || true)"
if [[ -n "${PREVIOUS_TAG}" ]]; then
  RANGE="${PREVIOUS_TAG}..${CURRENT_TAG}"
else
  RANGE="${CURRENT_TAG}"
fi

mapfile -t COMMITS < <(git log --pretty=format:'%h%x1f%s' "${RANGE}")

declare -a FEATURES=()
declare -a FIXES=()
declare -a PERFORMANCE=()
declare -a REFACTORS=()
declare -a BREAKING=()

for row in "${COMMITS[@]}"; do
  short_hash="${row%%$'\x1f'*}"
  subject="${row#*$'\x1f'}"
  lowered="$(echo "${subject}" | tr '[:upper:]' '[:lower:]')"
  entry="- ${subject} (${short_hash})"

  if [[ "${subject}" == *"!:"* ]] || [[ "${subject}" == *"!"*"("*"):"* ]] || [[ "${lowered}" == *"breaking change"* ]]; then
    BREAKING+=("${entry}")
    continue
  fi

  if [[ "${lowered}" =~ ^feat(\(.+\))?: ]]; then
    FEATURES+=("${entry}")
  elif [[ "${lowered}" =~ ^fix(\(.+\))?: ]]; then
    FIXES+=("${entry}")
  elif [[ "${lowered}" =~ ^perf(\(.+\))?: ]]; then
    PERFORMANCE+=("${entry}")
  elif [[ "${lowered}" =~ ^refactor(\(.+\))?: ]]; then
    REFACTORS+=("${entry}")
  elif [[ "${lowered}" =~ ^(chore|docs|style|test|ci|build)(\(.+\))?: ]]; then
    continue
  fi
done

print_section() {
  local title="$1"
  shift
  local values=("$@")
  if [[ ${#values[@]} -eq 0 ]]; then
    return
  fi

  echo
  echo "## ${title}"
  for line in "${values[@]}"; do
    echo "${line}"
  done
}

echo "# Release ${CURRENT_TAG}"
if [[ -n "${PREVIOUS_TAG}" ]]; then
  echo
  echo "Changes since ${PREVIOUS_TAG}."
fi

print_section "Breaking Changes" "${BREAKING[@]}"
print_section "Features" "${FEATURES[@]}"
print_section "Fixes" "${FIXES[@]}"
print_section "Performance" "${PERFORMANCE[@]}"
print_section "Refactors" "${REFACTORS[@]}"

if [[ ${#BREAKING[@]} -eq 0 && ${#FEATURES[@]} -eq 0 && ${#FIXES[@]} -eq 0 && ${#PERFORMANCE[@]} -eq 0 && ${#REFACTORS[@]} -eq 0 ]]; then
  echo
  echo "## Notes"
  echo "- No user-facing conventional commit entries found in this range."
fi
