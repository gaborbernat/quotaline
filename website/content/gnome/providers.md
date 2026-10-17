---
title: Provider access on GNOME
description: Find existing client credentials and understand GNOME-specific discovery.
weight: 2
---

Sign in through the provider's client. Quotaline reads existing local credentials; it does not ask for passwords or
store access tokens. Quota requests go directly to the provider. The [provider reference](/reference/providers/) covers
reported data and account-specific limits.

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
