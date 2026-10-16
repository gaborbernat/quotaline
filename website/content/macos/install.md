---
title: Install on macOS
description: Choose a direct download, Homebrew or the App Store.
weight: 1
---

Quotaline runs on macOS 15, 26 and 27 on Apple silicon and Intel. The menu bar item opens a SwiftUI popover with Usage,
History and Settings. Desktop widgets mirror selected readings without fetching provider data.

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

For unreleased changes, [build from source](/contributing/build/).

## Use the app

[Connect a provider](/start/connect/) through its client. Choose quota cells in
[Display settings](/reference/settings/menu-bar/), then open [Usage](/reference/usage/) or
[History](/reference/history/). **Launch at login** and **Open Login Items** are in Settings > About.

The [shared reference](/reference/) describes controls used by both frontends.
