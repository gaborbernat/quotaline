#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
mkdir -p "${TMPDIR:-../target/tmp}"
scratch="$(mktemp -d "${TMPDIR:-../target/tmp}/quotaline-ui-inventory.XXXXXX")"
trap 'rm -rf "$scratch"' EXIT
jq -e 'type == "object" and length > 0 and all(.[]; type == "array" and length > 0)' \
  tests/ui-groups.json > /dev/null
if [[ -n "${UI_GROUP:-}" ]]; then
  jq -e --arg group "$UI_GROUP" 'has($group)' tests/ui-groups.json > /dev/null
fi
if [[ -n "${UI_TEST:-}" ]]; then
  [[ -n "${UI_GROUP:-}" ]]
  jq -e --arg group "$UI_GROUP" --arg test "$UI_TEST" '.[$group] | index($test) != null' \
    tests/ui-groups.json > /dev/null
fi
for suite in functional performance; do
  if [[ "$suite" == performance ]]; then
    directory=tests/Performance
    selection='.performance[]'
  else
    directory=tests/ApplicationUI
    selection='del(.performance)[][]'
  fi
  jq -r "$selection" tests/ui-groups.json | LC_ALL=C sort > "$scratch/selected"
  for file in "$directory"/*.swift; do
    name="$(basename "$file" .swift)"
    sed -nE "s/.*func (test[A-Za-z0-9_]+)\\(.*/$name\/\1/p" "$file"
  done | LC_ALL=C sort > "$scratch/declared"
  test -s "$scratch/declared"
  diff -u "$scratch/declared" "$scratch/selected"
  echo "$suite suite covers $(wc -l < "$scratch/selected" | tr -d ' ') methods exactly once."
done
