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

The command writes the headless Shell gallery to `target/shell-screenshots/`. It captures Usage, History and Settings at
several scroll positions, menu bar formats, Quick Settings sizes and narrow monitors. Open popover captures include
`-menu.png` crops for comparison with macOS. The website's GNOME images come from these unscaled menu crops; review the
raw output before converting it to WebP.

Use the filename to find a specific state:

| Prefix or example                                                                     | What it shows                                                     |
| ------------------------------------------------------------------------------------- | ----------------------------------------------------------------- |
| `usage-*`, `history-*`, `settings-*`                                                  | Main flows and scroll positions.                                  |
| `*-light.png`, `*-high-contrast.png`                                                  | GNOME appearance variants.                                        |
| `*-help.png`                                                                          | Tooltips for quota, panel, footer, History and Settings controls. |
| `claude-organization-menu.png`, `claude-masked-menu.png`                              | Account chips before and after privacy masking.                   |
| `history-empty.png`, `history-hovered-menu.png`, `history-keyboard-selected-menu.png` | Empty, pointer and keyboard chart states.                         |
| `provider-visibility-*.png`, `settings-undiscovered-provider.png`                     | Provider discovery and setup controls.                            |
| `settings-full-log-sparse-menu.png`, `local/settings-full-log-retained-menu.png`      | Expanded logs with one or many entries.                           |
