#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

reject_matches() {
  message="$1"
  pattern="$2"
  shift 2
  if matches="$(rg -n "$pattern" "$@")"; then
    echo "$message" >&2
    echo "$matches" >&2
    exit 1
  else
    status=$?
    if ((status != 1)); then
      echo "test isolation scan failed with status $status" >&2
      exit "$status"
    fi
  fi
}

reject_matches \
  "tests and benchmarks must use isolated credential, HTTP, notification, and login-item clients" \
  'SecItem(Add|CopyMatching|Delete|Update)|URLSession(\.shared|[[:space:]]*\()|SystemHTTPTransport\.make|KeychainCredentialClient\.system|keychain:[[:space:]]*\.system|LaunchAtLoginService\.backend|UNUserNotificationCenter\.current|SystemLoginLauncher\.open' \
  tests --glob '*.swift'
reject_matches \
  "tests must use TestDefaults, which never touches ~/Library/Preferences" \
  'UserDefaults[[:space:]]*\([[:space:]]*suiteName:' \
  tests --glob '*.swift'
reject_matches \
  "tests must inject process boundaries instead of launching subprocesses" \
  '\b(Process|LibprocProcessScanner)[[:space:]]*\(' tests --glob '*.swift'
reject_matches \
  "persistent preferences must use the injectable Core boundary" \
  'UserDefaults[[:space:]]*\([[:space:]]*suiteName:' \
  src --glob '*.swift' --glob '!PersistentDefaults.swift'

reject_matches \
  "native presentation tests belong in tests/QuotalineUITests/Native, excluded from local builds" \
  '\b(NSWindow|NSPanel|NSPopover|NSOpenPanel|NSSavePanel|NSAlert|NSWindowController|NSStatusBar|StatusItemController|LogWindowController|AppController|AppDelegate|DeferredAppDelegate|TooltipPanel|TooltipPresenter)[[:space:]]*\(|NSStatusBar\.system|\.(orderFrontRegardless|orderFront|orderBack|makeKeyAndOrderFront|showWindow|runModal|beginModalSession|activate)[[:space:]]*\(' \
  tests --glob '*.swift' --glob '!**/Native/**' --glob '!**/ApplicationUI/**' --glob '!**/Performance/**' --glob '!**/UITestSupport/**'

reject_matches \
  "local window subclasses must use the presentation-rejecting fixture" \
  ':[[:space:]]*(NSWindow|NSPanel)\b' \
  tests --glob '*.swift' --glob '!**/Native/**' --glob '!**/ApplicationUI/**' --glob '!**/Performance/**' --glob '!**/UITestSupport/**' --glob '!NonpresentingWindow.swift'

global_mutation='method_exchangeImplementations|URLProtocol\.(registerClass|unregisterClass)|setClass\(|(NSApplication\.shared|NSApp|application)\.delegate[[:space:]]*='
if mutation_files="$(rg -l "$global_mutation" tests --glob '*.swift')"; then
  while IFS= read -r file; do
    if ! rg -q '@Suite\([.]serialized\)' "$file"; then
      echo "$file mutates process-global state outside a serialized suite" >&2
      rg -n "$global_mutation" "$file" >&2
      exit 1
    fi
  done <<< "$mutation_files"
else
  status=$?
  if ((status != 1)); then
    echo "test isolation scan failed with status $status" >&2
    exit "$status"
  fi
fi
