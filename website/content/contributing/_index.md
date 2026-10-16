---
title: Contributing
description: Build, test, document or release the app.
weight: 6
aliases: [/guides/, /guides/contributing/]
---

Start with the [build guide](/contributing/build/) to install the locked toolchains and prepare a development checkout.
All recipes live in the root `justfile`; the `macos-` recipes select their working directory automatically.

## Code changes

Put provider rules and other cross-platform decisions in the topic crate under `crates/` that owns them. Keep
operating-system access in platform adapters or native code. Rust passes facts to the interfaces; Swift and GNOME
TypeScript own visible copy, colors, and layout. The [architecture guide](/contributing/architecture/) describes those
boundaries, and [provider development](/contributing/providers/) covers new integrations.

Add behavior tests at the public boundary of each changed crate. Development and verification use mock providers; keep
real accounts and credentials out of tests. Before requesting review, run the source checks:

```sh
mise exec -- just pre-commit
```

Run the platform and coverage checks relevant to your change from the [testing guide](/contributing/testing/). It also
explains which macOS tests need a GitHub-hosted desktop. For visual changes, inspect the captures described in
[documentation images](/contributing/screenshots/).

## Documentation changes

Keep README files and CONTRIBUTING.md short and link to the hosted documentation. Maintain installation, usage, and
contribution instructions in `website/content/` so readers find them on Read the Docs.

Across the repository, split prose paragraphs longer than 150 words at topic boundaries, aiming for 80–150 words per
resulting paragraph. Keep shorter paragraphs concise; do not pad summaries, instructions, or list items to reach 80
words. Tables and code blocks are not prose paragraphs. Format Markdown with `mdformat --wrap 120` after editing it.

The [website guide](/contributing/website/) covers previews and site assets. Run `mise exec -- just site-check` before
requesting review; the site build checks internal page links and screenshot references.

## Security reports

Report vulnerabilities privately through
[GitHub security advisories](https://github.com/quotaline/quotaline/security/advisories/new), following the
[security policy](https://github.com/quotaline/quotaline/blob/main/SECURITY.md). Do not include credentials in public
issues or pull requests.
