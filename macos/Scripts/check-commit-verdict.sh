#!/usr/bin/env bash
set -euo pipefail

ref="${1:?usage: check-commit-verdict.sh <ref>}"
: "${REPOSITORY:?}"

sha="$(gh api "repos/$REPOSITORY/commits/$ref" --jq .sha)"
runs="$(gh api --paginate --slurp "repos/$REPOSITORY/actions/workflows/ci.yml/runs?head_sha=$sha&per_page=100" | jq '[.[].workflow_runs[]]')"

# A cancelled run carries no verdict either way, and pushing to a branch cancels the runs its own commits started, so
# the newest run that reached a conclusion decides. A re-run that repaired an infrastructure failure counts, and a
# genuine failure cannot hide behind an older success.
verdict="$(jq -r '[.[] | select(.status == "completed" and .conclusion != "cancelled")][0].conclusion // "none"' <<< "$runs")"
if [[ "$verdict" == success ]]; then
  full="$(jq -r '[.[] | select((.event == "schedule" or .event == "workflow_dispatch") and .conclusion != "cancelled")][0] | if . == null then "none" elif .status != "completed" then "pending" else .conclusion end' <<< "$runs")"
  if [[ "$full" == success ]]; then
    echo "CI passed on $sha (full matrix)" >> "${GITHUB_STEP_SUMMARY:-/dev/null}"
    exit 0
  fi
  echo "::error::Full-matrix CI for $ref ($sha) concluded $full; run CI manually on this commit before publishing" >&2
  exit 1
fi

if [[ "$verdict" == none ]]; then
  pending="$(jq -r '[.[] | select(.status != "completed")] | length' <<< "$runs")"
  if ((pending > 0)); then
    echo "::error::CI for $ref ($sha) has not finished, so this commit is not ready to publish" >&2
  else
    echo "::error::No CI run reached a verdict on $ref ($sha), so nothing tested the commit this release builds" >&2
  fi
  exit 1
fi

echo "::error::CI for $ref ($sha) concluded $verdict, so this commit is not ready to publish" >&2
exit 1
