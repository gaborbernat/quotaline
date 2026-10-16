#!/usr/bin/env bash
set -euo pipefail

script="$(cd "$(dirname "$0")" && pwd)/ci-change-base.sh"
probe="$(mktemp -d "${TMPDIR:-/tmp}/quotaline-ci-base.XXXXXX")"
trap 'rm -rf "$probe"' EXIT
git init -q "$probe"
cd "$probe"
git config user.email fixture@example.invalid
git config user.name Fixture
git config commit.gpgsign false
git commit -qm base --allow-empty
export CI_BASE_SHA CI_HEAD_SHA CI_PR_NUMBER=17 GITHUB_RUN_ID=10 GITHUB_OUTPUT="$probe/output" GITHUB_STEP_SUMMARY="$probe/summary"
CI_BASE_SHA="$(git rev-parse HEAD)"
git commit -qm tested --allow-empty
tested="$(git rev-parse HEAD)"
git commit -qm changed --allow-empty
CI_HEAD_SHA="$(git rev-parse HEAD)"
git checkout -qb divergent "$CI_BASE_SHA"
git commit -qm divergent --allow-empty
divergent="$(git rev-parse HEAD)"
git checkout -q --detach "$CI_HEAD_SHA"

run="$(jq -n --arg head "$tested" --arg base "$CI_BASE_SHA" '
  {id:1, run_number:1, event:"pull_request", conclusion:"success", head_sha:$head,
   display_title:("CI / PR 17 / base " + $base)}')"

expect() {
  local expected="$1"
  : > "$GITHUB_OUTPUT"
  bash "$script"
  grep -Fxq "base=$expected" "$GITHUB_OUTPUT"
}

jq -n --argjson run "$run" '{workflow_runs:[$run]}' | expect "$tested"
echo '{"workflow_runs":[]}' | expect "$CI_BASE_SHA"
for change in '.conclusion="failure"' '.conclusion="cancelled"' '.conclusion=null' \
  '.event="push"' '.id=10' '.display_title="CI / PR 18 / base different"' \
  '.display_title="CI / PR 17 / base changed"' '.head_sha="0000000000000000000000000000000000000000"'; do
  jq -n --argjson run "$run" "{workflow_runs:[\$run | $change]}" | expect "$CI_BASE_SHA"
done
jq -n --argjson run "$run" --arg head "$divergent" \
  '{workflow_runs:[$run | .head_sha=$head]}' | expect "$CI_BASE_SHA"
jq -n --argjson run "$run" --arg head "$CI_HEAD_SHA" \
  '{workflow_runs:[$run, ($run | .id=2 | .run_number=2 | .head_sha=$head)]}' | expect "$CI_HEAD_SHA"
jq -n --argjson run "$run" --arg head "$CI_HEAD_SHA" \
  '{workflow_runs:[$run, ($run | .id=2 | .run_number=2 | .head_sha=$head | .conclusion="failure")]}' | expect "$tested"
echo 'CI baseline selection holds'
