# Art

A lightweight desktop app that displays museum artwork fullscreen across all your monitors with a single keyboard shortcut.

Press `Cmd+Shift+Up` (configurable) and a random painting from world-class collections fills every screen. Press it again — or hit `Escape` — to dismiss.

## Sources

Artwork is pulled from four open-access museum APIs:

- **The Metropolitan Museum of Art** — Met Collection API
- **Art Institute of Chicago** — AIC Public API + IIIF
- **Cleveland Museum of Art** — Open Access API
- **National Gallery of Art** — Embedded catalog + IIIF

Images are prefetched in the background so they appear instantly.

## Features

- Fullscreen overlay on every connected monitor
- Background prefetch with history navigation (arrow keys)
- System tray icon — no dock icon clutter
- Configurable global hotkey
- Hides dock and menu bar during display (macOS)

## Tech

Tauri 2 + Svelte 5 frontend, Rust backend. Requires no server or account.

## Development

```sh
bun install
bun run tauri dev
```

## Build

```sh
bun run tauri build
```

Produces `.dmg` / `.app` (macOS), `.deb` / `.AppImage` (Linux), and `.nsis` (Windows).
