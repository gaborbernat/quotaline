#!/usr/bin/env bash
set -euo pipefail

: "${CI_BASE_SHA:?}" "${CI_HEAD_SHA:?}" "${CI_PR_NUMBER:?}" "${GITHUB_RUN_ID:?}" "${GITHUB_OUTPUT:?}"
baseline="$CI_BASE_SHA"
title="CI / PR $CI_PR_NUMBER / base $CI_BASE_SHA"

# The run title records the base at dispatch; the API's pull_requests entry tracks the PR's current base instead.
while IFS= read -r candidate; do
  if git cat-file -e "$candidate^{commit}" 2> /dev/null && git merge-base --is-ancestor "$candidate" "$CI_HEAD_SHA"; then
    baseline="$candidate"
    break
  fi
done < <(jq -r --arg title "$title" --arg run "$GITHUB_RUN_ID" '
  [.workflow_runs[] | select(.event == "pull_request" and .conclusion == "success"
    and .display_title == $title and (.id | tostring) != $run)] |
  sort_by(.run_number) | reverse | .[].head_sha')

echo "base=$baseline" >> "$GITHUB_OUTPUT"
echo "Changed paths: $baseline → $CI_HEAD_SHA" >> "${GITHUB_STEP_SUMMARY:-/dev/null}"
