# BroomSweepy

<p align="center">
  <img src="apps/desktop/src-tauri/icons/app-icon-master.png" width="112" alt="BroomSweepy broom app icon">
</p>

<p align="center">
  <strong>English</strong> |
  <a href="README.md">한국어</a> |
  <a href="README.ja.md">日本語</a> |
  <a href="README.zh-CN.md">简体中文</a>
</p>

BroomSweepy is a desktop storage explorer and cleanup reviewer for Windows and macOS. It finds large files, byte-verified duplicates, empty folders, cleanup candidates, file names, and text inside supported documents. The Rust scanning core performs the work locally; AI connections are optional.

There is one installer, not a separate installer for each language. A new installation starts in English. Choose English, 한국어, 日本語, or 简体中文 under `Settings > Display language`; the app UI, Windows tray menu, and requested AI response language change immediately, and the choice is stored only on this computer.

## Quick start

1. Open `Storage` and choose a folder.
2. Select `Build storage map`. Larger rectangles use more space; select a folder rectangle to move deeper.
3. Run `Find large files and duplicates` only when you need detailed results. BroomSweepy reads file contents only for duplicate candidates.
4. Review every selected item and confirm once more before BroomSweepy moves it to the operating-system Trash or Recycle Bin.

Use `Find files` for name and path searches, and `Search documents` for text searches. These features work without AI, CLI, or MCP connections.

## Preview

![BroomSweepy settings in English](docs/assets/screenshots/v1.3.0-settings-en.png)

The language selector changes the existing application; it does not install another edition or change operating-system settings.

## Highlights in v1.3.0

- One local UI with English as the default and complete English, Korean, Japanese, and Simplified Chinese catalogs.
- The selected language also updates HTML language metadata, the Windows tray menu, and the response-language request sent to an installed AI CLI.
- A clearer broom silhouette at 16–32 px for the taskbar, tray, Dock, and installer assets.
- A bundled `bloomsweepy-mcp` helper in Windows installers, with user-controlled Codex and Claude Code registration from Settings.
- Pathless, bounded cleanup summaries for external AI; exact paths and final approval remain inside BroomSweepy.
- The window title includes the package version so the running build is easy to identify.

## Main features

- Dashboard with drive usage, free space, recent Trash activity, and recently added files.
- A proportional `storage treemap` with folder drill-down.
- Large-file ranking and duplicate verification using size groups, partial BLAKE3, full BLAKE3, and final byte comparison.
- Local SQLite FTS catalogs for fast file-name/path search and supported document-content search.
- TXT, Markdown, source code, PDF text layers, DOCX, XLSX, PPTX, and HWPX indexing.
- Windows installation inventory from read-only registry data and macOS application inventory from `.app/Contents/Info.plist`.
- Reviewable Temp, cache, AppData, and leftover-uninstaller candidates without automatic registry deletion.
- Background scanning, bounded memory and result counts, cancellation checkpoints, and stale-file revalidation.
- Operating-system Trash or Recycle Bin moves with a synchronized JSONL journal and interrupted-operation review.
- An optional Docker view for images, stopped containers, and old build cache. Docker volumes are never pruned.

## AI, CLI, and MCP boundary

BroomSweepy performs scanning, indexing, revalidation, and Trash moves on the local computer. Codex, Claude Code, Grok, Antigravity, or Ollama can summarize bounded results and suggest what to review, but they do not receive an unrestricted filesystem tool from the app.

MCP cleanup tools expose anonymous candidate IDs and bounded summaries only. They do not expose approval, permanent-delete, Trash-execution, registry-write, or Trash-emptying tools. An exact path becomes visible only in the app, and a cleanup runs only after the user confirms it there.

## Docker is opt-in

Docker support is off by default. While it is off, BroomSweepy does not locate or run the Docker CLI. When enabled, the separate Docker view can inspect `docker system df` data and preview only fixed, allowlisted cleanup commands for old build cache, dangling images, and stopped containers. Docker cleanup bypasses the operating-system Trash and therefore requires a separate irreversible-action confirmation.

## Platforms and source layout

| Path | Role |
|---|---|
| `BroomSweepy/` | Existing native macOS SwiftUI application |
| `crates/bloomsweepy-core/` | Cross-platform Rust scanning core |
| `crates/bloomsweepy-control/` | Local app/CLI control protocol |
| `apps/desktop/` | Tauri 2, React, and TypeScript desktop application |
| `apps/bloomsweepy-mcp/` | Thin CLI and MCP bridge for a running app |

Windows installers are built on Windows. A signed and notarized `.dmg` must be built on macOS.

## Development

Requirements: Rust stable, Node.js 22 or later, npm, WebView2 and MSVC Build Tools on Windows.

```powershell
cd apps/desktop
npm install
npm run tauri dev
```

Core validation:

```powershell
cargo fmt --all -- --check
cargo test --workspace
cd apps/desktop
npm run check
npm run build
```

For detailed architecture and safety notes, see the [Korean reference README](README.md), [CLI control](docs/cli-control.md), [cross-platform architecture](docs/architecture/cross-platform-desktop.md), and [safe Trash actions](docs/architecture/safe-trash-actions.md).

## Important: data loss and recovery responsibility

BroomSweepy is designed to act only on items the user selected and confirmed. Recovery can still depend on operating-system permissions, Trash settings, sync services, and external or network-drive behavior. Docker cleanup does not use the operating-system Trash and completed steps cannot be restored.

Back up important data and verify every selected path, file, and Docker category before running cleanup. The project providers and contributors are not responsible for data loss or failed recovery caused by user-initiated file moves, deletion, Trash emptying, or Docker cleanup, except where liability cannot legally be excluded.
