#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
probe="$(mktemp -d "${TMPDIR:-/tmp}/quotaline-commit-verdict.XXXXXX")"
trap 'rm -rf "$probe"' EXIT
export GITHUB_STEP_SUMMARY="$probe/summary" REPOSITORY=quotaline/quotaline PATH="$probe/bin:$PATH"

mkdir -p "$probe/bin"
cat > "$probe/bin/gh" << 'STUB'
#!/usr/bin/env bash
case "$*" in
  */commits/*) echo 3d3c42e5aac5ba805825da76410c181273ba90b1 ;;
  *) cat "$RUNS" ;;
esac
STUB
chmod +x "$probe/bin/gh"

expect() {
  local name="$1" expected="$2" actual=0
  : > "$GITHUB_STEP_SUMMARY"
  bash Scripts/check-commit-verdict.sh v9.9.9 > "$probe/out" 2> "$probe/err" || actual=$?
  ((actual == expected)) || {
    echo "$name: expected exit $expected, got $actual" >&2
    cat "$probe/err" >&2
    exit 1
  }
}

echo '[{"event":"schedule","status":"completed","conclusion":"success"}]' > "$probe/runs.json"
RUNS="$probe/runs.json" expect "a passing run publishes" 0
grep -Fq 'CI passed on 3d3c42e5aac5ba805825da76410c181273ba90b1' "$GITHUB_STEP_SUMMARY"

echo '[{"event":"schedule","status":"completed","conclusion":"failure"}]' > "$probe/runs.json"
RUNS="$probe/runs.json" expect "a failing run stops the release" 1
grep -Fq 'concluded failure' "$probe/err"

echo '[{"event":"schedule","status":"in_progress","conclusion":null}]' > "$probe/runs.json"
RUNS="$probe/runs.json" expect "an unfinished run stops the release" 1
grep -Fq 'has not finished' "$probe/err"

echo '[]' > "$probe/runs.json"
RUNS="$probe/runs.json" expect "an untested commit stops the release" 1
grep -Fq 'No CI run reached a verdict' "$probe/err"

echo '[{"event":"schedule","status":"completed","conclusion":"success"},{"event":"schedule","status":"completed","conclusion":"failure"}]' > "$probe/runs.json"
RUNS="$probe/runs.json" expect "the newest run decides" 0

echo '[{"event":"schedule","status":"completed","conclusion":"cancelled"},{"event":"schedule","status":"completed","conclusion":"success"}]' > "$probe/runs.json"
RUNS="$probe/runs.json" expect "a cancelled run defers to the verdict behind it" 0

echo '[{"event":"schedule","status":"completed","conclusion":"cancelled"},{"event":"schedule","status":"completed","conclusion":"failure"}]' > "$probe/runs.json"
RUNS="$probe/runs.json" expect "a cancelled run cannot hide a failure" 1
grep -Fq 'concluded failure' "$probe/err"

echo '[{"event":"schedule","status":"completed","conclusion":"cancelled"}]' > "$probe/runs.json"
RUNS="$probe/runs.json" expect "cancellation alone is no verdict" 1
grep -Fq 'No CI run reached a verdict' "$probe/err"

echo '[{"event":"push","status":"completed","conclusion":"success"}]' > "$probe/runs.json"
RUNS="$probe/runs.json" expect "a reduced push run cannot authorize publication" 1
grep -Fq 'Full-matrix CI' "$probe/err"

echo '[{"event":"pull_request","status":"completed","conclusion":"success"}]' > "$probe/runs.json"
RUNS="$probe/runs.json" expect "a reduced pull request run cannot authorize publication" 1

echo '[{"event":"push","status":"completed","conclusion":"success"},{"event":"workflow_dispatch","status":"completed","conclusion":"failure"}]' > "$probe/runs.json"
RUNS="$probe/runs.json" expect "reduced success cannot hide a full-matrix failure" 1

echo '[{"event":"push","status":"completed","conclusion":"success"},{"event":"workflow_dispatch","status":"in_progress","conclusion":null},{"event":"schedule","status":"completed","conclusion":"success"}]' > "$probe/runs.json"
RUNS="$probe/runs.json" expect "a newer full-matrix run must finish" 1

echo '[{"event":"push","status":"completed","conclusion":"success"},{"event":"workflow_dispatch","status":"completed","conclusion":"success"}]' > "$probe/runs.json"
RUNS="$probe/runs.json" expect "manual full-matrix success authorizes publication" 0

echo "commit verdict gate holds"
