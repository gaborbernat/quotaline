#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
mkdir -p "${TMPDIR:-../target/tmp}"
probe="$(mktemp -d "${TMPDIR:-../target/tmp}/quotaline-ui-inventory-probe.XXXXXX")"
trap 'rm -rf "$probe"' EXIT
export TMPDIR="$probe"
mkdir -p "$probe/tests/ApplicationUI" "$probe/tests/Performance" "$probe/Scripts"
cp tests/ApplicationUI/*UITests.swift "$probe/tests/ApplicationUI/"
cp tests/Performance/*.swift "$probe/tests/Performance/"
cp Scripts/check-ui-test-inventory.sh "$probe/Scripts/"
cp tests/ui-groups.json "$probe/tests/"
bash "$probe/Scripts/check-ui-test-inventory.sh"
while IFS= read -r group; do
  UI_GROUP="$group" bash "$probe/Scripts/check-ui-test-inventory.sh"
done < <(jq -r 'keys[]' tests/ui-groups.json)
while IFS=$'\t' read -r group method; do
  UI_GROUP="$group" UI_TEST="$method" bash "$probe/Scripts/check-ui-test-inventory.sh"
done < <(jq -r 'to_entries[] | .key as $group | .value[] | [$group, .] | @tsv' tests/ui-groups.json)
for group in '' usage lifecycle; do
  if UI_GROUP="$group" UI_TEST=LiveControlAuditUITests/testHistoryDatesAndControlsRespond \
    bash "$probe/Scripts/check-ui-test-inventory.sh" > "$probe/result" 2>&1; then
    echo "UI inventory accepted a method outside its diagnostic group: $group" >&2
    exit 1
  fi
done
if UI_GROUP=lifecycle UI_TEST=Unknown/testMissing \
  bash "$probe/Scripts/check-ui-test-inventory.sh" > "$probe/result" 2>&1; then
  echo "UI inventory accepted an unknown diagnostic method" >&2
  exit 1
fi
if UI_GROUP=missing-group bash "$probe/Scripts/check-ui-test-inventory.sh" > "$probe/result" 2>&1; then
  echo "UI inventory accepted an unknown diagnostic group" >&2
  exit 1
fi
for mutation in 'del(.performance[0])' '.performance += [.performance[0]]' '.performance[0] = "Unknown/testMissing"' '.lifecycle += [.performance[0]] | .performance |= .[1:]'; do
  jq "$mutation" tests/ui-groups.json > "$probe/tests/ui-groups.json"
  if bash "$probe/Scripts/check-ui-test-inventory.sh" > "$probe/result" 2>&1; then
    echo "UI inventory accepted a missing, duplicate, or unknown test: $mutation" >&2
    exit 1
  fi
  grep -Eq '^[-+][^-+].*/test' "$probe/result"
done
echo "UI inventory rejected missing, duplicate, and unknown tests."
