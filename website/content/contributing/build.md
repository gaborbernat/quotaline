---
title: Build from source
description: Fallback installation and a mock-data development setup.
weight: 1
---

Use a published [release](/start/) for a normal install. Build from source to work on the app or try unreleased changes.

## Tools

[mise](https://mise.jdx.dev) pins Rust, Swift, Node.js and the development tools in `mise.lock`.
[just](https://just.systems) drives the workflows.

```sh
git clone https://github.com/quotaline/quotaline
cd quotaline
mise install --locked
```

Install [Xcode 26 or later](https://developer.apple.com/xcode/) for the macOS SDK and application builds. Package checks
use the Swift toolchain pinned by mise; application builds use Xcode's compiler. Use mock verification for development
checks.

## GNOME build and install

On GNOME Shell 50, install `gnome-extensions` and `systemctl --user`, then run:

```sh
mise exec -- just install
```

The install stays in your home directory. See the [GNOME guide](/gnome/) for the service and extension paths.

## macOS build and install

Install `jq`, `tesseract`, and `webp` with Homebrew. Run these commands from the repository root:

```sh
QUOTALINE_BUILD_JOBS=2 mise exec -- just macos-app
open "macos/dist/Quotaline.app"
```

This opens a development build with your provider data. Run `just macos-install` to install it in `/Applications` and
open it. Local bundles have no Sparkle updater or widget extension. Build commands use two compiler jobs; set
`QUOTALINE_BUILD_JOBS=1` on a memory-constrained machine.

macOS asks before a build reads another CLI's Keychain item, such as Claude Code's. It pins **Always Allow** to the
app's team, and an ad-hoc build has none, so every rebuild asks again. `just macos-app` and `just macos-install` sign
with the first Apple Development identity in your keychain, or with `QUOTALINE_SIGNING_IDENTITY`, and fall back to ad
hoc. Create a free identity under Xcode > Settings > Accounts > Manage Certificates, then approve the prompt once.

## Isolated preview

Use `just macos-run-demo` to explore generated data in a separate verification instance with mock credentials, its own
preferences and a temporary support directory. Quit that instance when finished.

Read [Testing](/contributing/testing/) before running application UI tests. Compile-only preparation does not open a
window.
