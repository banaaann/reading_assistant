# Reading Assistant Pro

Reading Assistant Pro 是一个 Windows 桌面阅读辅助工具，基于 Tauri、React 和 Rust 构建。

它可以监听用户选中的文本，弹出一个紧凑的查询窗口，帮助完成句子翻译、单词查询、关键词提取和生词收藏等操作。

## 功能特性

- 支持通过全局快捷键或鼠标触发对选中文本进行查询。
- 提供轻量级弹窗，支持调整宽度、最大高度和字体缩放比例。
- 弹窗高度会根据内容自动适配，只有当内容超过配置的最大高度时才启用滚动。
- 支持句子翻译、原文显示、关键词提取和逐词查询。
- 支持生词收藏列表，并使用本地 SQLite 数据库存储。
- 优先使用本地词典查询，并提供短超时的网络兜底查询。
- 支持系统托盘行为，以及关闭主窗口时最小化到托盘。

## 开发

环境要求：

- Node.js
- Rust
- Tauri 构建所需的 Windows 构建工具

安装依赖：

```bash
npm install
```

启动 Vite 前端：

```bash
npm run dev
```

启动 Tauri 应用：

```bash
npm run tauri:dev
```

如果 `127.0.0.1:1420` 上已经有 Vite 服务正在运行，可以使用：

```bash
npm run tauri:dev:shell
```

## 检查

```bash
npm run lint
npm run build
cd src-tauri && cargo fmt --check && cargo check
```

## 构建

```bash
npm run tauri:build
```

构建产物会生成在 `src-tauri/target/` 目录下，并且该目录已被 Git 忽略。

## 仓库说明

本仓库会忽略本地构建产物、生成的 Tauri schemas、本地 Zig 工具链或压缩包，以及和当前机器相关的辅助脚本。

在推送代码之前，请确认以下内容没有被加入暂存区：

- `zig.zip`
- `zig-x86_64-windows-*`
- `node_modules`
- `dist`
- `src-tauri/target`


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

