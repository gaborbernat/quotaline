#!/usr/bin/env bash
set -euo pipefail

if [[ "${GITHUB_ACTIONS:-}" != true || "${RUNNER_ENVIRONMENT:-}" != github-hosted ]]; then
  echo "Native AppKit tests require an isolated GitHub-hosted desktop. Run just macos-test locally." >&2
  exit 1
fi

export QUOTALINE_TEST_DESKTOP=github-hosted
export TEST_RUNNER_QUOTALINE_TEST_DESKTOP=github-hosted
export TEST_RUNNER_GITHUB_ACTIONS=true
export TEST_RUNNER_RUNNER_ENVIRONMENT=github-hosted
