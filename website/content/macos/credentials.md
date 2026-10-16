---
title: Credential access
description: Handle Keychain prompts, App Store folder grants and custom paths.
weight: 2
---

## Keychain access

Several clients keep their sign-in in the macOS login keychain. macOS guards each item with a list of trusted apps, and
a client trusts only itself. The first time Quotaline reads such an item, macOS shows a dialog that names the item and
asks for your login keychain password:

- **Always Allow** approves this app for that item.
- **Allow** approves one read, so the dialog returns at the next poll.
- **Deny** refuses. The app reports the denial in **Settings > Providers** and waits 30 minutes before it reads that
  item again.

The password goes to macOS; the app never sees it. A client that rewrites its item drops earlier approvals, so the
dialog can return; [Claude](/reference/providers/claude/#what-macos-asks) does this when it renews its sign-in.

## Folder grants

The App Store build runs in the macOS sandbox and cannot open another app's files until you pick them. In **Settings >
Providers**, turn on **Show all providers** if yours is missing and check its enable box. Its **Grant** buttons appear
then, one per folder the provider can use, so you pick only the folders you use. Each folder is labelled **Needed**,
**Optional** (it adds History and costs, not quota) or **Grant the one you use** (any one of several is enough), and
hovering its name explains why the app asks. Confirm the folder the panel proposes; press **Grant Again** if a folder
moved. macOS remembers the choice. Direct and Homebrew builds need no grants.

## Custom credential paths

Environment variables must exist in the app process. Finder and login-item launches do not read `.zshrc`. Each
[provider page](/reference/providers/) names its supported overrides.

For example, to select a Codex home for subsequent launches in this login session:

```sh
launchctl setenv CODEX_HOME /path/to/codex-home
```

Quit and reopen Quotaline, then check the resolved source in Settings. Remove the override when finished:

```sh
launchctl unsetenv CODEX_HOME
```

Do not move or copy secrets to troubleshoot a path.
