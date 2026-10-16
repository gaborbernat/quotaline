---
title: Testing
description: Rust, GNOME Shell, and macOS checks before release.
weight: 4
---

## Rust and GNOME Shell

Run these commands from the repository root after `mise install --locked`:

```sh
mise exec -- just test-unit
mise exec -- just coverage
mise exec -- just coverage-crates
mise exec -- just coverage-macos-ffi
mise exec -- just screenshots
```

`test-unit` runs Rust and standalone TypeScript tests without a Shell. `coverage` also starts isolated headless GNOME
Shell sessions and requires 100% Rust and extension line and branch coverage. Extension coverage is measured on the
compiled JavaScript in `target/extension/`; `tsc` only erases types there, so each line and branch maps to one in the
TypeScript source. `lint-extension` runs Biome, strict type-aware ESLint with GJS checks, and `tsc --noEmit`. The two
crate-specific commands check the shared core and macOS UniFFI adapter independently. `screenshots` writes the current
visual review gallery to `target/shell-screenshots/`; it does not replace the login Shell.

GNOME unit tests mirror the source layout: `gnome/extension/src/a.ts` has tests in `gnome/extension/tests/a.test.ts`.
Node runs these TypeScript tests directly against the compiled extension. Scenarios that exercise several components in
a real Shell live under `tests/integration/`; `tsc` compiles those for GJS. Shared fixtures and runtime helpers live
under `tests/support/`. Both test suites use strict TypeScript and ESLint.

Run `mise exec -- just pre-commit` for source checks across the repository. It includes Biome, ESLint, TypeScript,
Clippy, Ruff, shell and workflow linting, spelling, and file hygiene. On macOS it also runs Apple `swift format` and
SwiftLint; Linux skips those two hooks. `mise exec -- just macos-lint-swift` runs the Swift checks directly.

All recipes live in the root `justfile`. The `macos-` recipes select their working directory, so they work from the
repository root. `just macos-test-python` checks the typed helpers and their tests with ty and 100% line coverage.
Helpers declare their Python version and dependencies inline and run through `uv run`.

## macOS package and application

From the repository root, `mise exec -- just macos-test` runs nonpresenting Swift package tests with render checks. Its
wrapper checks captured text with Tesseract and `jq`; bare `swift test` does not set up those checks. Render fixtures
open no windows. `mise exec -- just macos-ui-local --prepare-only all` builds the verification app without launching it.

Native package tests, full Swift coverage, and the scripted application suite require a GitHub-hosted desktop. The
guarded recipes refuse to run on a developer Mac. For a named local application UI test, `just macos-ui-local` uses mock
providers and temporary support files. That route requires an unlocked desktop and preconfigured unattended XCTest
Automation Mode; it does not request permissions or change security settings.

The macOS coverage job requires 100% measured Swift lines and regions in Core, UI, and Widgets. Swift currently emits no
branch counters, so an `llvm-cov` report showing zero Swift branches does not establish branch coverage. Rust and the
GNOME extension have separate 100% line and branch gates. See the
[Swift compiler issue](https://github.com/swiftlang/swift/issues/81730) for the instrumentation limit.

## Performance benchmarks

`QuotalinePerformanceTests` is a separate target with its own `Quotaline-Benchmarks` scheme and test plan. The
functional suite and normal CI exclude it.

Run `just macos-benchmark` for mock-only measurements on the latest deployed macOS, or
`just macos-ui-local --suite performance --prepare-only all` to compile without launching. For a hosted run, dispatch CI
with `ui_group=performance`; it selects the latest deployed runtime regardless of the diagnostic runtime input.
Benchmarks report startup, tab-switch timing, CPU and memory. These measurements guide development and do not impose
performance thresholds on CI.

## Panel acceptance

Run this check on macOS 15, 26, and 27 before a release:

```sh
just macos-run-demo
```

The demo launch uses seeded providers, a separate defaults suite, and a temporary support directory. Do not use
`just macos-run`, which loads the current account's provider data, for verification.

1. Open the status item near the left edge, centre, and right edge of the menu bar. The arrow must meet the status item
   at each position.
1. Switch through Usage, History, and Settings. The panel's top edge and width must stay fixed while its bottom edge
   moves. Repeat on a short display and a secondary display with a different scale.
1. Make each tab exceed the screen height. The body must scroll without clipping the tab control or moving the arrow.
1. Press each action button. It must show a bezel, pressed state, and non-accent label. Check light, dark, increased
   contrast, and reduced transparency appearances.
1. Enable Keyboard navigation under System Settings > Keyboard. Use Tab and Shift-Tab to reach each control, Command-R
   to refresh, Command-F to focus the Settings model filter, and Escape to close the panel. Focus rings must remain
   visible.
1. Leave the panel open for two minutes. Activity Monitor should show stable memory and no sustained CPU work while the
   data remains unchanged.

The functional application UI suite checks all three tabs for accessibility faults, controls and panel positioning. It
uses bounded waits to catch hangs and removes its defaults and support files after each test. Pull requests run four
application UI shards on GitHub-hosted macOS 26. Scheduled full runs also cover macOS 15 and, once GitHub completes its
image rollout, macOS 27. Each job checks its runtime version; the `xcode-27` label alone does not prove macOS 27
coverage.
