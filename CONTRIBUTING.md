# Contributing

The [architecture](website/content/contributing/architecture.md), [build](website/content/contributing/build.md), and
[testing](website/content/contributing/testing.md) guides cover the workspace and both desktops.

Install the tools pinned by the root `mise.toml` and `mise.lock`:

```sh
mise install --locked
mise exec -- just
```

Put provider rules and other cross-platform decisions in the topic crate under `crates/` that owns them. Keep OS access
in platform adapters or native code. Rust passes facts to the interfaces; Swift and GNOME JavaScript own visible copy,
colors, and layout. Add behavior tests at the public boundary of each changed crate. Do not use live provider accounts
or credentials in tests.

Before requesting review, run the local checks that match the change:

```sh
mise exec -- just lint
mise exec -- just coverage
mise exec -- just coverage-crates
mise exec -- just coverage-macos-ffi
```

For macOS package tests, run `mise exec -- just test` from `macos/`. Native application tests use the guarded workflow
described in the [testing guide](website/content/contributing/testing.md). GNOME visual captures come from
`mise exec -- just screenshots` at the repository root; inspect the files in `target/shell-screenshots/`.

Report vulnerabilities through
[GitHub security advisories](https://github.com/quotaline/quotaline/security/advisories/new) under the
[security policy](SECURITY.md), not through a public issue.
