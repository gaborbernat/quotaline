---
title: Codex
description: Codex sign-in, session and weekly limits, reset credits, API analytics and local fallback.
provider: codex
weight: 2
---

## Connect

Run `codex login`, then enable Codex in **Settings > Providers**. Use **Show all providers** if it is absent.

The app reads `~/.codex/auth.json`, Keychain service `Codex Auth` and discovered Codex homes. Local rollouts live under
the selected home's `sessions` directory. **Authentication** and **Connection details** identify the source in use.
`CODEX_HOME` selects a different root; see [custom credential paths](/start/connect/#custom-credential-paths).

### What macOS asks

- **Keychain password**, when a `Codex Auth` Keychain item exists. The app reads that item first unless
  `~/.codex/config.toml` sets `cli_auth_credentials_store = "file"`, then falls back to `auth.json`. See
  [Keychain access](/start/connect/#keychain-access).
- **App Store build.** Grant `~/.codex` once; see [folder grants](/start/connect/#folder-grants).

## Quota and credits

Usage shows session, weekly and scoped model limits, reported credits, spend controls and reset-credit expiry. Weekly
restrictions mark affected menu bar cells even if a selected session has quota left. Reset countdowns use exact
deadlines; a day-only expiry remains a date.

Credits, token counts and quota percentages have different units. An exhausted weekly meter does not imply a zero credit
balance.

[ChatGPT Work and Codex share usage and credits](https://learn.chatgpt.com/docs/pricing). The **Credits** balance
includes available usage credits that the provider reports, including promotional grants. **Limit resets** are separate
credits that reset a quota window. Their expiry dates do not establish when usage credits expire; the current balance
API does not expose usage-credit grant expiries. Check ChatGPT **Settings > Usage** for those dates.

Pro plans have no five-hour limit. The app displays the windows the provider returns, including accounts with only a
weekly window, and follows changes to the included allowance through the reported percentage. It does not calculate
subscription limits from API token prices. Pro labels match Codex: **Pro**, **Pro (More)** and **Pro (Max)**.

Select **History > Credit balance** to track the last fetched balance for each UTC day. Recording starts after the app
fetches a balance; the app does not reconstruct earlier balances from credit purchases or consumption. Missing,
unlimited and stale local-log balances do not add points. A reported zero does.

## History and fallback

The API supplies surface usage, model credits, turns, threads, tokens, skills, plugins and code reviews. Input,
cached-input and output tokens can share a chart with Claude, but Codex may report a total where Claude reports model
series. Daily aggregates retain UTC buckets; see [History](/reference/history/).

Model names come from the provider, so new releases such as GPT-6 Sol, GPT-6 Luna and GPT-6.1 Sol appear without an app
update. Usage percentages and reported credits reflect model and speed differences.
[Conversations with dots](https://learn.chatgpt.com/docs/dots) do not count toward ChatGPT usage limits. Tasks a dot
starts or manages in Work or Codex count toward those products' limits.

Local rollouts can supply last-known quota while the API is unavailable. These values remain stale until a current fetch
succeeds. Local records do not establish your subscription bill; a missing monetary cost metric stays absent.

## Expired credentials

Use **Sign in…** to run `codex login` for a supported CLI source. Other sources open setup instructions. **Refresh
expired tokens on my behalf** starts off. Enabling it permits token refresh and saving to the original credential
source. A CLI holding the previous token may need another sign-in after rotation; read the
[shared-token trade-off](/explanation/privacy/#token-refresh).

## Polling and network

The default quota interval is two minutes, with a one-minute floor while the panel is open. Analytics use the separate
[collection clock](/explanation/collection/).

- Quota: `chatgpt.com/backend-api/wham/usage`.
- Credits: `chatgpt.com/backend-api/wham/rate-limit-reset-credits`.
- Analytics: `chatgpt.com/backend-api/wham/usage/daily-token-usage-breakdown` and `wham/analytics/*`.
- Optional token refresh: `auth.openai.com/oauth/token`.

These vendor APIs can change. Use [diagnostics](/troubleshooting/diagnostics/) to report missing returned data.
