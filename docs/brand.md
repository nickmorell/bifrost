# BiFrost brand

BiFrost is a calm, roomy desktop app for Bluesky and the Atmosphere, and everything
around it (icon, profile images, posts, README) should feel the same. This page is the
reference; the files live in `assets/brand`.

## Name and tagline

- The name is **BiFrost**: capital B, capital F, no umlaut. Use it the same way in the
  README, window title, About text and Bluesky bio.
- Tagline: **"One desktop app for Bluesky and the Atmosphere."**
- Say "for Bluesky" or "a Bluesky client" as a description, never as part of the name,
  and don't use Bluesky's butterfly. That keeps BiFrost clear of Bluesky's trademark.
- Be upfront that BiFrost is built with AI help (see "How BiFrost is made" in the README).

## The mark

Two arcs, one over the other: the bridge in the name, drawn as a rainbow with its colour
taken out. The outer arc is sage at full strength, the inner one sage at 45%.

| File | Use |
|---|---|
| `icon.svg` | The app icon: the mark on a dark rounded square. The master for every export. |
| `icon-small.svg` | One bolder arc, for 16 to 24 px where two arcs blur together. |
| `icon-macos.svg` | The icon inset on Apple's grid (824 px tile in a 1024 px canvas). |
| `mark.svg` | The arcs alone, transparent. The app tints it with the theme's accent. |
| `bifrost.ico` | Windows program icon, 16 to 256 px (16 to 24 from `icon-small.svg`). |
| `bifrost.icns` | macOS app bundle icon, 16 to 1024 px. |
| `png/bifrost-*.png` | 16 to 1024 px, for Linux icon themes and anything else. |
| `tray.png` | 32 px from `icon-small.svg`, the tray icon before its unread badge. |

Where each one is used:

- **Windows:** `build.rs` embeds `bifrost.ico` and the name into `bifrost.exe`.
- **macOS:** the release workflow copies `bifrost.icns` into `BiFrost.app`.
- **Linux:** the release archive includes `bifrost.png` and `bifrost.desktop`; the window
  names that desktop entry so docks show the icon.
- **Every system:** the window icon (`png/bifrost-128.png`), the tray (`tray.png`), Windows
  notifications (`png/bifrost-256.png`), and the sidebar and login screen (`mark.svg`).

Don't stretch, rotate, outline or recolour the mark, apart from the theme's accent in the
light theme. Give it clear space of at least the outer arc's thickness on every side.

## Colour

Taken from the app's dark theme (`src/theme/dark.rs`).

| Name | Hex | Use |
|---|---|---|
| Sage | `#a8d5ba` | The accent: the mark, buttons, links, highlights |
| Graphite | `#151517` | Backgrounds; the icon tile fades from `#232428` down to this |
| Surface | `#1e1e21` | Panels and cards on graphite |
| Text | `#e9e9ec` | Headings and body text on dark |
| Muted | `#9a9ca3` | Secondary text, taglines |
| Rose | `#f2a9ba` | Likes only |
| Sage, light theme | `#3d7a57` | The accent on light backgrounds |

Lavender was the second choice for the accent and is still the one to try if sage ever
needs a partner. Avoid blue: it reads as Bluesky's own.

## Type

[JetBrains Mono](https://www.jetbrains.com/lp/mono/) (OFL), the app's own font in
`assets/fonts`: SemiBold for the name and headings, Regular for body text, Medium for
small labels. No other typefaces.

## Look and voice

- Dark first, lots of space, one accent colour, rounded corners, no gradients beyond the
  icon tile's soft fade.
- Short, plain sentences. Say what BiFrost does, not how it compares with other clients.
- Screenshots use made-up sample accounts (`@zen.test`, `@ari.test` and so on), never a
  real person's handle.

## Social images

Made from the same parts, kept outside the repository:

| Image | Size | Where it goes |
|---|---|---|
| GitHub social preview | 1280 x 640 | Repository Settings, General, Social preview |
| Bluesky avatar | 1000 x 1000 | The BiFrost account's profile picture (shown as a circle) |
| Bluesky banner | 3000 x 1000 | The BiFrost account's banner; text stays in the middle third |
