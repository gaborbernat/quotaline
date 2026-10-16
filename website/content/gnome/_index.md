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

At later logins, GNOME starts the enabled extension and activates the service on demand. Use GNOME Extensions to control
whether the panel item starts at login.

Open **Settings → Demo data** to see all six provider cards without signing in. Turn demo data off before checking your
own accounts.

## Providers

Sign in through the provider's client. Quotaline reads existing local credentials; it does not ask for passwords or
store access tokens. Quota requests go directly to the provider.

| Provider       | Credential source                                                                                        | Usage shown                                                 |
| -------------- | -------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------- |
| Claude         | `~/.claude/.credentials.json` or `CLAUDE_CONFIG_DIR`                                                     | Session, weekly, model and other reported limits            |
| Codex          | Secret Service or `~/.codex/auth.json`; respects `CODEX_HOME`                                            | Session, weekly, code review and additional reported limits |
| Gemini CLI     | `~/.gemini/oauth_creds.json` by default; Secret Service when encrypted storage is enabled                | Per-model quota buckets                                     |
| Antigravity    | Running IDE language server, Secret Service, CLI or XDG app credentials, or `ANTIGRAVITY_AUTH_FILE` JSON | Reported quota pools                                        |
| Cursor         | `~/.cursor/auth.json` or Cursor's XDG `state.vscdb`                                                      | Plan, on-demand, overall and team quotas when reported      |
| GitHub Copilot | Environment token, `copilot-cli` Secret Service item, `~/.copilot/config.json` or XDG editor credentials | Reported quota snapshots                                    |

Antigravity local discovery reads only same-user process and socket metadata, then talks to the discovered loopback
server. Its cloud fallback reads the `service=gemini`, `username=antigravity` Secret Service item, the Antigravity CLI
credential file at `~/.gemini/antigravity-cli/antigravity-oauth-token`, the XDG `Antigravity/auth.json` file, or
`ANTIGRAVITY_AUTH_FILE`. It uses provider tokens for requests and excludes them from the Shell process, snapshots,
settings, history and diagnostics.

Gemini follows the CLI's storage choice: the standard OAuth file, or the Secret Service item when
`GEMINI_FORCE_ENCRYPTED_FILE_STORAGE=true` is in the service environment. `GEMINI_CLI_HOME` changes the file location.

## Using the menu

- **Usage** shows the latest available quota reading, reset countdown and per-provider refresh action. Failed requests
  retain the last reading as stale.
- **History** charts collected quota samples, Codex daily analytics and Claude local transcript metrics for today, 7, 30
  or 60 days. Quota samples have Minute, Hour and Day rollups with local or UTC calendar grouping. Daily additive
  metrics support stacked bars. The reading inspector steps through recorded points, and the legend can isolate a
  series. Focus the chart and use Left and Right to move through readings; Backspace clears the selection. The service
  caps quota chart responses at 800 points per series; chart CSV export contains every retained sample for the selected
  period. Settings can also export all retained samples and daily analytics to a location chosen in the desktop save
  dialog. The service stores history in SQLite for 60 days by default.
- **Settings** controls providers, their display order, model order, polling intervals, displayed windows, short labels,
  menu format, retention, notifications, logs and demo data. The model list can hide windows with no retained activity
  and a current quota at zero. Provider checks run every 1–30 minutes; at most two network requests run at once. The
  panel requests model activity and logs only while Settings is open. Each provider has connection details and a local
  setup guide. The log view filters by level and text, copies the visible entries, updates on new entries, and can clear
  them. Detailed refresh timing is off by default.
- **Quick Settings** shows a cached quota snapshot in small, medium or large form. Choose the size in Settings; click
  the tile to open Quotaline or its arrow to view the snapshot. Opening it does not fetch provider data.

While the menu is open, Ctrl+R refreshes providers, Ctrl+F opens Settings with the model filter focused, and Escape
closes the menu.

Usage shows pacing, credit and spend details when providers report them. The service collects Codex daily counts,
tokens, credits and surface metrics. It reads Claude transcript token counts, estimated costs, message counts and local
session totals without storing message text. Token refresh is off by default. The Settings opt-in can refresh expired
Claude Code file credentials, Codex and Gemini CLI file or Secret Service credentials, and Antigravity cloud
credentials. Antigravity keeps its refreshed access token in memory; Cursor and Copilot credentials remain read-only.
The service has no account or telemetry.

## Interface preview

These captures come from an isolated GNOME Shell test session with demo data.

{{< shot gnome="gnome-usage" alt="GNOME Usage tab with Claude and Antigravity quota cards" >}}

{{< shot gnome="gnome-history" alt="GNOME History tab with a usage chart" >}}

{{< shot gnome="gnome-settings" alt="GNOME Settings tab with provider and display controls" >}}

## Data and support

`~/.local/libexec/quotaline-gnome --usage-json` prints the last cached quota snapshot for scripts without starting the
service or making network requests. The output contains provider IDs, plans, timestamps and quota windows; it omits
credentials and account identities. Enabling **Settings → Write usage.json** writes the same document to
`~/.local/share/quotaline/usage.json` after updates.

- Settings: `~/.config/quotaline/settings.json`
- History: `~/.local/share/quotaline/history.sqlite3`
- Snapshot: `~/.cache/quotaline/snapshot.json`
- Logs: `journalctl --user -u quotaline-gnome`
- Local log: `~/.local/share/quotaline/log.txt` and up to two rotated copies; Settings shows the last seven days

The service creates settings, snapshot, history and exported CSV files with owner-only permissions. To report a problem,
use [GitHub Issues](https://github.com/quotaline/quotaline/issues) and include the service log without credential
contents.

For an isolated preview and the full screenshot gallery, see
[documentation images](/contributing/screenshots/#gnome-shell). The [build guide](/contributing/build/) covers the
development setup.
