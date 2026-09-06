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

BroomSweepy is a desktop storage explorer and cleanup reviewer for Windows and macOS. **Rust + Tauri 2 + React is the primary project**, carrying forward the spacious cards, clear icons, and glass-inspired experience of the original SwiftUI app. The Swift source in `BroomSweepy/` remains a legacy reference implementation.

Large-file analysis, verified duplicates, and document search run locally in Rust. AI is optional. One installation supports English, Korean, Japanese, and Simplified Chinese; English is the first-run default. Change it under `Settings > Display language`.

## Quick start

1. Select a drive on the dashboard, or choose a local folder in `Space cleanup`. Choosing a folder builds its storage map.
2. Follow the largest rectangles and click folders to drill down. The item menu can reveal a location or start review of an individual file's Trash move.
3. Run large-file and duplicate analysis together, then select the items you want to review. Moving files requires final confirmation inside the app.
4. Use `Performance` for CPU and memory, `File management` for names and document text, and `AI assistant` for natural-language questions.

## Preview

These captures render the current Rust app's actual React components with synthetic documentation data. Drives, files, metrics, and conversations are examples, not real AI responses or user files. Browser captures do not reproduce macOS native window materials.

### Multi-drive dashboard

![Multi-drive dashboard](docs/assets/screenshots/v1.6.0-dashboard-en.png)

### Storage treemap

![Storage treemap](docs/assets/screenshots/v1.6.0-overview-en.png)

### CPU and memory

![CPU and memory](docs/assets/screenshots/v1.6.0-performance-en.png)

### AI assistant

![AI assistant](docs/assets/screenshots/v1.6.0-assistant-en.png)

### Settings and display language

![Settings and display language](docs/assets/screenshots/v1.6.0-settings-en.png)

## What's new in v1.6.0

- Dark glass-inspired surfaces, prominent action buttons, clear icons, and streamlined navigation.
- A large selected-drive card beside compact drive cards. Selecting another drive exchanges their positions and sizes with an animated transition.
- CPU and memory rings, top-app usage, smooth transitions between samples, and reduced-motion support.
- macOS app-memory cleanup returns only unused allocator pages from the BroomSweepy host process. Zero bytes returned is a valid completion.
- Treemap navigation and individual-file actions with identity/path revalidation and a Trash-operation journal.
- Distinct missing, broken, incompatible, and sign-in-required AI CLI states, plus improved conversation, cancellation, and saved-history handling.

## Safety and cloud exclusions

Scans do not modify files. Large-file/duplicate scans, drive summaries, treemaps, file catalogs, and document indexes exclude known cloud-sync roots and online-only entries. The policy prunes macOS `~/Library/CloudStorage`, `~/Library/Mobile Documents`, and recognized Google Drive, iCloud, OneDrive, and Dropbox paths before traversal; selecting a recognized cloud root directly is also rejected. Even downloaded files inside recognized cloud roots are excluded. Arbitrarily relocated sync folders and every possible provider cannot be identified reliably.

Duplicates pass size grouping, partial/full BLAKE3, and final byte comparison. File moves require selection, revalidation, final confirmation, and journaling before using the OS Trash or Recycle Bin. Empty-folder discovery is read-only. There is no general permanent-delete, Trash-emptying, or automatic registry-deletion action. Trashed logical bytes do not equal newly available disk space.

Memory cleanup does not purge system RAM, other apps, WebView helper processes, swap, or memory leaks. There is no CPU-cleanup action. On macOS, requesting a normal app exit is a separate confirmation flow with no force-kill fallback. Windows performance is read-only; swap is a commit-based estimate, not current pagefile usage.

Docker management is off by default. When enabled, cleanup uses fixed commands, excludes volumes, and requires a separate irreversible-action confirmation.

## AI, CLI, and MCP

Installing the Codex desktop app does not establish that the Codex CLI is installed. Check your provider's CLI installation, compatible version, and sign-in state in BroomSweepy. This Mac conversation flow was verified with Codex. Adapters also exist for Claude Code, Grok, Antigravity, and Ollama, but not all were live-tested for this Mac release.

In-app chat sends a bounded folder summary containing item names and sizes, plus your question and conversation history, to the selected provider. A local CLI does not mean offline model processing. MCP cleanup tools expose anonymous candidate IDs and bounded summaries, with no approval or deletion-execution tools. Separately enabling file or document search can expose paths and matching excerpts to external clients. Final file actions stay behind confirmation in the app.

## Platforms and development

Requirements: Rust stable, Node.js 22+, and npm; WebView2 and MSVC Build Tools on Windows; Xcode Command Line Tools on macOS.

- `apps/desktop/`: primary Tauri/React app
- `crates/bloomsweepy-core/`: shared Rust analysis engine
- `crates/bloomsweepy-control/`, `apps/bloomsweepy-mcp/`: local control protocol and CLI/MCP bridge
- `BroomSweepy/`: legacy SwiftUI reference

This update was built and installed on Apple Silicon macOS, with local-file scans and the Codex conversation flow checked. Current Windows runtime verification is separate; Windows installers are built by Windows CI. The Mac validation build is ad-hoc signed, not Apple-notarized. See [Releases](https://github.com/Dannykkh/bloomsweepy/releases) for downloads and platform-specific caveats.

```sh
cd apps/desktop
npm ci
npm run tauri dev
```

```sh
# Repository root
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd apps/desktop
npm run check
npm run test:all
npm run build
npm run tauri build
```

## Documentation

[Changelog](CHANGELOG.md) · [CLI connection and control](docs/cli-control.md) · [Performance and memory boundaries](docs/architecture/startup-memory-status.md) · [Safe Trash actions](docs/architecture/safe-trash-actions.md) · [Document search](docs/architecture/document-search.md) · [File search](docs/architecture/fast-file-search.md) · [Design](DESIGN.md) · [Reproduce screenshots](docs/assets/screenshots/README.md)

## Important: data loss and recovery responsibility

BroomSweepy is designed to act only on items the user selected and confirmed. Recovery can still depend on operating-system permissions, Trash settings, sync services, and external or network-drive behavior. Docker cleanup does not use the operating-system Trash and completed steps cannot be restored.

Back up important data and verify every selected path, file, and Docker category before running cleanup. The project providers and contributors are not responsible for data loss or failed recovery caused by user-initiated file moves, deletion, Trash emptying, or Docker cleanup, except where liability cannot legally be excluded.
