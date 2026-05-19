# Reading Assistant Pro

Reading Assistant Pro is a Windows desktop reading assistant built with Tauri, React, and Rust. It watches the selected text, opens a compact lookup popup, and helps with sentence translation, word lookup, keywords, and starred vocabulary.

## Features

- Global hotkey or mouse trigger for selected text lookup.
- Lightweight popup with adjustable width, max height, and font scale.
- Auto-sizing popup height with scrolling only when content exceeds the configured max height.
- Sentence translation, original text display, keyword extraction, and per-word lookup.
- Starred vocabulary list with local SQLite storage.
- Local-first dictionary lookup with a short network fallback timeout.
- Tray behavior and close-to-tray main window.

## Development

Requirements:

- Node.js
- Rust
- Windows build tools required by Tauri

Install dependencies:

```bash
npm install
```

Run the Vite frontend:

```bash
npm run dev
```

Run the Tauri app:

```bash
npm run tauri:dev
```

If a Vite server is already running on `127.0.0.1:1420`, you can use:

```bash
npm run tauri:dev:shell
```

## Checks

```bash
npm run lint
npm run build
cd src-tauri && cargo fmt --check && cargo check
```

## Build

```bash
npm run tauri:build
```

Build artifacts are generated under `src-tauri/target/` and are intentionally ignored by Git.

## Repository Notes

The repository ignores local build outputs, generated Tauri schemas, local Zig toolchains/archives, and machine-specific helper scripts. Before pushing, make sure `zig.zip`, `zig-x86_64-windows-*`, `node_modules`, `dist`, and `src-tauri/target` are not staged.
