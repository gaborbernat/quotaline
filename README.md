# Quotaline

Quotaline tracks coding quota usage on GNOME and macOS. The GNOME app is a Shell panel extension backed by a local Rust
service; its menu has **Usage**, **History**, and **Settings** tabs and targets GNOME Shell 50. The macOS app and its
tests are in [macos](macos/README.md). Shared quota models, pacing, analytics, exports, and notification rules live in
topic crates under `crates/`.

## Local install

Prerequisites: GNOME Shell 50, [mise](https://mise.jdx.dev/installing-mise.html), `gnome-extensions`, and
`systemctl --user`.

```sh
mise install --locked
mise exec -- just install
```

The installer builds a release binary with Cargo, installs it in `~/.local/libexec`, installs the extension for your
account, and starts a user service. At later logins, GNOME starts the enabled extension and activates the service on
demand. Use GNOME Extensions to control whether the panel item starts at login. The install is entirely within your home
directory. Run `mise exec -- just uninstall` to remove the service and extension; it keeps your settings and history.

The installer enables the panel item for your next login. A running Wayland Shell may not discover a newly installed
extension and may retain previously loaded JavaScript after an update. Log out and back in to load the new extension
code. To inspect the new build without logging out, use `mise exec -- just screenshots` to inspect a fresh Shell in an
isolated test session. For manual interaction, run `mise exec -- just manual-preview` and close that window when
finished. The manual preview starts a second desktop portal under your account, which can make RustDesk select the wrong
Wayland session. Do not use it for automated tests.

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

## Development

See the [build guide](website/content/contributing/build.md) for locked toolchains and the
[architecture guide](website/content/contributing/architecture.md) for the Rust and platform boundaries.

Refresh the full headless Shell gallery in [target/shell-screenshots](target/shell-screenshots) with
`mise exec -- just screenshots`. These include the top, middle and bottom of Usage, History and Settings, each menu bar
format, and all three Quick Settings snapshot sizes. Narrow monitor captures include Usage and the top and bottom of
History and Settings. Each open popover capture also has a `-menu.png` crop for side-by-side comparison with the macOS
reference. These can be reviewed without restarting the desktop session. `claude-organization-menu.png` and
`claude-masked-menu.png` show the account chips before and after the privacy setting. `quota-help.png` shows the quota
tooltip with usage, remaining amount and pace detail. `settings-help.png` shows contextual help on the menu bar
controls; `demo-help.png` and `notifications-help.png` show help in the lower Settings sections. `log-help.png` shows
help on the Log severity control. `history-help.png` shows metric help in the History view. `panel-help.png` shows the
status item summary on hover. `footer-help.png` shows help placed above its footer control. `history-empty.png` shows
History before any readings have been recorded. `settings-provider-details-menu.png` shows the expanded credential
source and setup control. `history-hovered-menu.png` shows the chart with one legend series highlighted.
`history-keyboard-selected-menu.png` shows a reading selected from the chart with the keyboard. `usage-light.png`,
`history-light.png`, `settings-light.png`, `usage-high-contrast.png`, `history-high-contrast.png` and
`settings-high-contrast.png` show the menu under GNOME's light and high contrast settings.
`stale-claude-setup-light-card.png` shows a retained reading and its muted panel value under the light setting.
`provider-visibility-mixed.png`, `provider-visibility-empty.png`, and `settings-undiscovered-provider.png` show provider
discovery and setup controls without logging out. `settings-full-log-sparse-menu.png` and
`local/settings-full-log-retained-menu.png` show the expanded log with one and many entries.

MIT licensed.
