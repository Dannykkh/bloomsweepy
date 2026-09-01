# Changelog

All notable changes to this project will be documented in this file.

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
