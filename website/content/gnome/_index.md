---
title: GNOME Shell
description: Install and use Quotaline on GNOME Shell 50.
weight: 2
---

Quotaline runs as a GNOME Shell extension backed by a local Rust service. The panel item opens **Usage**, **History**,
and **Settings** without launching a separate window. A Quick Settings tile shows the latest cached snapshot.

## Install locally

Use GNOME Shell 50 with [mise](https://mise.jdx.dev/installing-mise.html), `gnome-extensions`, and `systemctl --user`.

```sh
git clone https://github.com/quotaline/quotaline
cd quotaline
mise install --locked
mise exec -- just install
```

The installer builds the Rust service, installs it under `~/.local/libexec`, starts the service, and installs the
extension under `~/.local/share/gnome-shell/extensions`. It enables the extension for your next login and does not need
root access. A running Wayland Shell may not discover a newly installed extension and may retain previously loaded
JavaScript after an update. Log out and back in to load the new extension code.

Use `mise exec -- just uninstall` to remove the extension and service. Your settings and history remain in your home
directory.

## Use the panel

- **Usage** shows each provider's current limits, reset times, pace, credits, and a refresh control. Failed requests
  keep the most recent reading and mark it stale.
- **History** compares quota samples and local usage over time. Chart inspection, period selection, series isolation,
  rollups, and CSV export work from retained data.
- **Settings** controls providers, model visibility, labels, menu format, polling, notifications, history retention, and
  local logs. Demo data lets you inspect the layout before signing in.

The six supported providers are Claude, Codex, Gemini CLI, Antigravity, Cursor, and GitHub Copilot. The service reads
credentials already saved by their clients. It does not ask for a password or send usage data to Quotaline servers.

While the menu is open, Ctrl+R refreshes providers, Ctrl+F focuses the Settings model filter, and Escape closes it.

## Interface preview

These captures come from an isolated GNOME Shell test session with demo data.

{{< shot gnome="gnome-usage" alt="GNOME Usage tab with Claude and Antigravity quota cards" >}}

{{< shot gnome="gnome-history" alt="GNOME History tab with a usage chart" >}}

{{< shot gnome="gnome-settings" alt="GNOME Settings tab with provider and display controls" >}}

## Data and troubleshooting

The service keeps settings in `~/.config/quotaline`, history in `~/.local/share/quotaline/history.sqlite3`, and a cached
snapshot in `~/.cache/quotaline/snapshot.json`. Read service errors with `journalctl --user -u quotaline-gnome`.

To inspect current Shell code without changing the login session, run `mise exec -- just screenshots`. It writes a
gallery under `target/shell-screenshots/` in an isolated test session. See [build from source](/contributing/build/) for
the development setup.
