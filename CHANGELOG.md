# Changelog

All notable changes to this project will be documented in this file.

## [1.2.0] - 2026-09-02

### Features

- **dashboard**: Add an at-a-glance drive dashboard with usage rings, completed cleanup history, and recently discovered files. (`643b527`)
- **assistant**: Persist folder- and Docker-scoped conversations in SQLite, support local Codex, Claude Code, Grok, Antigravity, and Ollama providers, and keep all file operations inside BroomSweepy. (`643b527`)
- **docker**: Add an opt-in `Docker 용량` workspace for Docker-reported image, container, volume, and build-cache usage with a folder-free Docker conversation entry point. (`643b527`)
- **search**: Sort physical drives consistently, hide common cloud virtual volumes from the dashboard, and preserve recent-file comparison baselines. (`643b527`)

### Safety

- Restrict Docker cleanup to explicit seven-day builder, dangling-image, and stopped-container prune commands; exclude volumes and require an irreversible-action acknowledgement. (`643b527`)
- Surface bounded trash-operation history and startup recovery evidence without adding permanent file deletion or registry mutation. (`643b527`)
- Keep direct file opening on a document and media allowlist while revealing executable, link, package, and ambiguous entries in the file manager. (`643b527`)

### Documentation

- Add sanitized dashboard, Docker, and Docker conversation screenshots, a v1.2.0 feature summary, and an explicit data-loss and recovery disclaimer to the README. (`643b527`)

## [1.1.0] - 2026-09-01

### Bug Fixes

- **ci**: Gate NTFS-only catalog helpers so the Rust workspace builds cleanly on macOS. (`32dd698`)
- **macos**: Keep parallel cache and large-file scans inside the cancellation operation. (`be630f9`)
- **macos**: Add explicit result types to the concurrent Swift scan pipeline. (`ea08cb2`)
- **macos**: Resolve Swift tuple-return and duplicate-keeper type inference errors found by Xcode. (`2741073`)

## [0.1.0] - 2026-09-01

### Features

- Add the cross-platform Tauri desktop app with storage scans, a navigable storage treemap, large-file results, verified duplicates, cleanup candidates, fast filename search, and local document-content search. (`563c942`)
- Simplify the main workflow to folder selection, storage map, and detailed results; group large files, duplicates, and cleanup candidates under one storage section. (`563c942`)
- Add Windows tray behavior and a local CLI/MCP bridge that delegates all file work to the running BroomSweepy app. (`563c942`)
- Add Windows MSI and NSIS packaging and a macOS build-verification workflow. (`563c942`)

### Safety

- Move selected items through the operating-system trash with preflight revalidation, bounded journals, partial-failure reporting, and startup recovery checks. (`563c942`)
- Harden the existing SwiftUI macOS app with identity snapshots, private staging, cancellation ownership, bounded recovery, and review-only handling for folders or app data that cannot be proven safe. (`563c942`)
- Keep registry findings read-only, exclude shell execution from the control protocol, and require per-run approval for external searches and scans. (`563c942`)

### User Interface

- Keep all visible interface text at 14px or larger, remove the oversized mouse-focus box from search, and verify the native Windows app at 1280×820 and 760×600 without horizontal overflow. (`563c942`)
- Move optional local CLI setup and permissions to a separate `AI 도우미` screen so every core feature remains usable without an AI connection. (`563c942`)
