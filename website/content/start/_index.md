---
title: Get started
description: Install a release, connect a provider and choose your menu bar models.
weight: 1
aliases: [/start/install/]
---

## Install

Pick your desktop in the header; the steps below follow it.

{{< platform macos >}}

The macOS app targets versions 15, 26 and 27 on Apple silicon and Intel.

Earlier releases shipped as Token Menu Bar from the archived
[macOS repository](https://github.com/quotaline/token-menu-bar-macos/releases).

| Channel         | Install and update                                                                                                                                                                                                                    |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Direct download | Download `Quotaline.dmg` from the [latest release](https://github.com/quotaline/quotaline/releases/latest), drag the app to `/Applications`, then open it. This build updates itself through Sparkle.                                 |
| Homebrew        | Tap this repository, trust the cask, then install it. The commands are below. This build carries no updater, so `brew upgrade` brings each new version.                                                                               |
| App Store       | The App Store edition moves to a new listing named Quotaline; the Token Menu Bar listing gets no further updates. The sandbox keeps the new app out of the old one's container, so it starts with default settings and empty history. |

```sh
brew tap quotaline/quotaline https://github.com/quotaline/quotaline
brew trust --cask quotaline/quotaline/quotaline
brew install --cask quotaline
```

Homebrew refuses casks from a tap it does not trust, hence the middle command. Its own cask repository takes software
that has drawn a following, around 75 stars or 30 forks or 30 watchers, so `brew install --cask quotaline` without the
tap does not find this app yet.

{{< /platform >}}

{{< platform gnome >}}

The GNOME Shell 50 extension installs from source with a local Rust service and needs no root access:

```sh
git clone https://github.com/quotaline/quotaline
cd quotaline
mise install --locked
mise exec -- just install
```

[GNOME Shell](/gnome/) covers prerequisites, uninstalling, and where settings and history live.

{{< /platform >}}

For unreleased changes, [build from source](/contributing/build/).

## First use

1. [Connect a provider](/start/connect/) through the client you use.
1. Choose models and short labels in [Menu bar settings](/reference/settings/menu-bar/).
1. Read current limits in [Usage](/reference/usage/) or compare periods in [History](/reference/history/).

{{< platform macos >}}

**Launch at login** and its adjacent **Open Login Items** button are in Settings > About.

{{< /platform >}} {{< platform gnome >}}

GNOME starts enabled extensions when you sign in; Settings > About links to the Extensions app.

{{< /platform >}}

Demo data starts off; the checkbox in Settings > About is its one control.
