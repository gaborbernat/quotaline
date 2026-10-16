---
title: GNOME interface
description: Use the panel menu, Quick Settings tile and Shell shortcuts.
weight: 3
---

**Usage** shows the latest quota reading, reset countdown and per-provider refresh action. Failed requests retain the
last reading as stale. **History** charts collected quota samples, Codex daily analytics and Claude local transcript
metrics for today, 7, 30 or 60 days. Quota samples have Minute, Hour and Day rollups with local or UTC calendar
grouping. Daily additive metrics support stacked bars.

The reading inspector steps through recorded points, and the legend can isolate a series. Focus the chart and use Left
and Right to move through readings; Backspace clears the selection. The service caps quota chart responses at 800 points
per series; chart CSV export contains every retained sample for the selected period. Settings can export all retained
samples and daily analytics to a location chosen in the desktop save dialog. The service stores history in SQLite for 60
days by default.

**Settings** controls providers, display order, model order, polling intervals, visible windows, short labels, menu
format, retention, notifications, logs and demo data. The model list can hide windows with no retained activity and a
current quota at zero. Provider checks run every 1–30 minutes; at most two network requests run at once. The panel
requests model activity and logs only while Settings is open. Each provider has connection details and a local setup
guide. The log view filters by level and text, copies visible entries, updates on new entries and can clear them.
Detailed refresh timing is off by default.

**Quick Settings** shows a cached quota snapshot in small, medium or large form. Choose the size in Settings; click the
tile to open Quotaline or its arrow to view the snapshot. Opening it does not fetch provider data.

While the menu is open, Ctrl+R refreshes providers, Ctrl+F opens Settings with the model filter focused, and Escape
closes the menu.

Usage shows pacing, credit and spend details when providers report them. The service collects Codex daily counts,
tokens, credits and surface metrics. It reads Claude transcript token counts, estimated costs, message counts and local
session totals without storing message text. Token refresh is off by default. The Settings opt-in can refresh expired
Claude Code file credentials, Codex and Gemini CLI file or Secret Service credentials, and Antigravity cloud
credentials. Antigravity keeps its refreshed access token in memory; Cursor and Copilot credentials remain read-only.
The service has no account or telemetry.

These captures come from an isolated GNOME Shell test session with demo data:

{{< shot gnome="gnome-history" alt="GNOME History tab with a usage chart" >}}

{{< shot gnome="gnome-settings" alt="GNOME Settings tab with provider and display controls" >}}

The [shared reference](/reference/) describes the controls common to macOS and GNOME.
