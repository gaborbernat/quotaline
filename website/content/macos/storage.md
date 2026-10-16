---
title: Local storage
description: Find history, snapshots and logs for each macOS distribution channel.
weight: 3
---

Direct and Homebrew builds keep history, snapshots and logs under `~/Library/Application Support/Quotaline/`. The App
Store build uses `~/Library/Containers/tech.bernat.quotaline/Data/Library/Application Support/Quotaline/`. They have
separate settings and history. An App Store install therefore starts with empty history even if you used an earlier
direct build.

The App Store widget uses the `group.tech.bernat.quotaline` app group. Direct and Homebrew builds use
`<Team ID>.tech.bernat.quotaline`. The channels do not share widget snapshots. [Widgets](/macos/widgets/) read cached
data and do not fetch providers themselves.

The [privacy guide](/explanation/privacy/) describes what each file contains and what a diagnostic report can include.
