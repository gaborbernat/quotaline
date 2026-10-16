---
title: Data and support on GNOME
description: Find cached usage, settings, history and service logs.
weight: 4
---

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
contents. [Privacy](/explanation/privacy/) describes the data in snapshots and diagnostics.

For an isolated preview and the full screenshot gallery, see
[documentation images](/contributing/screenshots/#gnome-shell). The [build guide](/contributing/build/) covers the
development setup.
