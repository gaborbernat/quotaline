#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
probe_directory="$(mktemp -d "${TMPDIR:-/tmp}/quotaline-desktop-guard.XXXXXX")"
trap 'rm -rf "$probe_directory"' EXIT
source Scripts/profile-ui-tests.sh
for command in \
  '/fixture/Quotaline.app/Contents/MacOS/QuotalineDirect --verify-ui' \
  '/fixture/Quotaline.app/Contents/MacOS/QuotalineDirect --relaunched --verify-ui -detailedLogging YES'; do
  is_verification_command "$command"
done
for command in \
  '/fixture/QuotalineDirect' \
  '/fixture/QuotalineDirect --relaunched' \
  '/fixture/QuotalineDirect --verify-ui-invalid' \
  '/fixture/AnotherApp --verify-ui'; do
  if is_verification_command "$command"; then
    echo "Profile guard accepted a non-verification command: $command" >&2
    exit 1
  fi
done
xcrun clang -Wall -Wextra -Werror tests/QuotalineNativeGuard/NativeDesktop.c \
  tests/NativeDesktopGuardProbe/main.c -o "$probe_directory/probe"

reject() {
  if "$@" > "$probe_directory/stdout" 2> "$probe_directory/stderr"; then
    echo "Desktop guard unexpectedly accepted: $*" >&2
    exit 1
  fi
  [[ ! -s "$probe_directory/stdout" ]]
  grep -Eq 'require.*GitHub-hosted desktop' "$probe_directory/stderr"
}

clean=(env -u QUOTALINE_TEST_DESKTOP -u GITHUB_ACTIONS -u RUNNER_ENVIRONMENT)
reject "${clean[@]}" "$probe_directory/probe"
reject "${clean[@]}" QUOTALINE_COMPILE_NATIVE_TESTS=1 "$probe_directory/probe"
reject "${clean[@]}" QUOTALINE_TEST_DESKTOP=local-verification "$probe_directory/probe"
reject "${clean[@]}" QUOTALINE_TEST_DESKTOP=github-hosted "$probe_directory/probe"
reject "${clean[@]}" QUOTALINE_TEST_DESKTOP=github-hosted GITHUB_ACTIONS=true \
  RUNNER_ENVIRONMENT=self-hosted "$probe_directory/probe"
reject "${clean[@]}" QUOTALINE_TEST_DESKTOP=local GITHUB_ACTIONS=true \
  RUNNER_ENVIRONMENT=github-hosted "$probe_directory/probe"
reject "${clean[@]}" QUOTALINE_TEST_DESKTOP=github-hosted GITHUB_ACTIONS=false \
  RUNNER_ENVIRONMENT=github-hosted "$probe_directory/probe"
"${clean[@]}" QUOTALINE_TEST_DESKTOP=github-hosted GITHUB_ACTIONS=true \
  RUNNER_ENVIRONMENT=github-hosted "$probe_directory/probe" > "$probe_directory/stdout"
[[ "$(< "$probe_directory/stdout")" == 'test body reached' ]]

xcrun clang -Wall -Wextra -Werror -DQUOTALINE_APPLICATION_UI_TESTS=1 \
  tests/QuotalineNativeGuard/NativeDesktop.c tests/NativeDesktopGuardProbe/main.c -o "$probe_directory/application-probe"
reject "${clean[@]}" "$probe_directory/application-probe"
"${clean[@]}" QUOTALINE_TEST_DESKTOP=local-verification \
  "$probe_directory/application-probe" > "$probe_directory/stdout"
[[ "$(< "$probe_directory/stdout")" == 'test body reached' ]]

reject "${clean[@]}" just _macos-with-test-desktop "$probe_directory/probe"
reject "${clean[@]}" GITHUB_ACTIONS=true RUNNER_ENVIRONMENT=self-hosted \
  just _macos-with-test-desktop "$probe_directory/probe"
"${clean[@]}" GITHUB_ACTIONS=true RUNNER_ENVIRONMENT=github-hosted \
  just _macos-with-test-desktop "$probe_directory/probe" > "$probe_directory/stdout"
[[ "$(< "$probe_directory/stdout")" == 'test body reached' ]]
reject "${clean[@]}" TMB_TMPDIR="$probe_directory/coverage" just macos-coverage-prepared
reject "${clean[@]}" Scripts/profile-ui-tests.sh "$probe_directory/probe"
reject "${clean[@]}" Scripts/with-ui-artifacts.sh "$probe_directory/probe"
reject "${clean[@]}" Scripts/install-ocr.sh
reject "${clean[@]}" Scripts/with-render-checks.sh 12 --capture-only "$probe_directory/probe"
reject "${clean[@]}" GITHUB_ACTIONS=true RUNNER_ENVIRONMENT=self-hosted \
  Scripts/with-render-checks.sh 12 --capture-only "$probe_directory/probe"
reject "${clean[@]}" Scripts/check-cached-test-guard.sh
# A rejected desktop must not need the binding build's inputs.
mkdir -p "$probe_directory/guarded/macos/Scripts"
cp ../justfile "$probe_directory/guarded/justfile"
cp Scripts/check-test-desktop.sh "$probe_directory/guarded/macos/Scripts/"
for recipe in macos-test-native macos-coverage; do
  reject "${clean[@]}" just --justfile "$probe_directory/guarded/justfile" "$recipe"
done

echo "Desktop guard passed: native tests reject local execution before the test body"
