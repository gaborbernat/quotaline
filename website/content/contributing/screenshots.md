---
title: Documentation images
description: Capture the current UI with sample data and review the output.
weight: 5
---

## macOS

```sh
just macos-shots
```

The script rebuilds the app, then runs `--export-menubar` and `--export-popover` with isolated demo state, disabled
network transport and mock credentials. It waits for History data before drawing. The eight WebP assets cover the menu
bar and three tabs in both appearances, bounded to a laptop-sized viewport. Commit them with UI changes that affect the
documented screens; website CI builds those assets but does not regenerate them.

Use `APP_BINARY=/absolute/path/to/Quotaline just macos-shots` only to capture a binary you have verified matches the
source. Compare Settings > About's source version with the binary you intend to document. Inspect the full images for
loading states, clipped text and missing sections before publishing. The site build rejects missing shortcode images and
unresolved internal page links.

These exports do not create an `NSPopover` or a window-server surface. They cannot verify the arrow, control bezels,
focus rings, screen selection, or top-edge anchoring.

The social-preview image has a separate SVG source beside the rendered PNG. After editing it, install `librsvg` and run:

```sh
rsvg-convert website/static/brand/og.svg --output website/static/brand/og.png
rsvg-convert website/static/brand/icon.svg --width 32 --height 32 --output website/static/brand/icon-32.png
rsvg-convert website/static/brand/icon.svg --width 180 --height 180 --output website/static/brand/icon-180.png
```

Inspect the 1200 × 630 PNG before committing both files; social previews use the PNG.

## GNOME Shell

To inspect the new build without logging out, run `mise exec -- just screenshots` from the repository root. It starts a
fresh Shell in an isolated test session. For manual interaction, run `mise exec -- just manual-preview` and close that
window when finished. The manual preview starts a second desktop portal under your account, which can make RustDesk
select the wrong Wayland session. Do not use it for automated tests.

Refresh the full headless Shell gallery in `target/shell-screenshots/` with `mise exec -- just screenshots`. These
include the top, middle and bottom of Usage, History and Settings, each menu bar format, and all three Quick Settings
snapshot sizes. Narrow monitor captures include Usage and the top and bottom of History and Settings. Each open popover
capture also has a `-menu.png` crop for side-by-side comparison with the macOS reference. These can be reviewed without
restarting the desktop session. `claude-organization-menu.png` and `claude-masked-menu.png` show the account chips
before and after the privacy setting. `quota-help.png` shows the quota tooltip with usage, remaining amount and pace
detail. `settings-help.png` shows contextual help on the menu bar controls; `demo-help.png` and `notifications-help.png`
show help in the lower Settings sections. `log-help.png` shows help on the Log severity control.

`history-help.png` shows metric help in the History view. `panel-help.png` shows the status item summary on hover.
`footer-help.png` shows help placed above its footer control. `history-empty.png` shows History before any readings have
been recorded. `settings-provider-details-menu.png` shows the expanded credential source and setup control.
`history-hovered-menu.png` shows the chart with one legend series highlighted. `history-keyboard-selected-menu.png`
shows a reading selected from the chart with the keyboard. `usage-light.png`, `history-light.png`, `settings-light.png`,
`usage-high-contrast.png`, `history-high-contrast.png` and `settings-high-contrast.png` show the menu under GNOME's
light and high contrast settings. `stale-claude-setup-light-card.png` shows a retained reading and its muted panel value
under the light setting. `provider-visibility-mixed.png`, `provider-visibility-empty.png`, and
`settings-undiscovered-provider.png` show provider discovery and setup controls without logging out.
`settings-full-log-sparse-menu.png` and `local/settings-full-log-retained-menu.png` show the expanded log with one and
many entries.
