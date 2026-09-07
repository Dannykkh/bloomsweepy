# Changelog

All notable changes to this project will be documented in this file.

## [1.7.0] - 2026-09-07

### Features

- **desktop**: Connect conversational empty-folder scans, bounded candidate pages, selection review and explicit final confirmation to the app-owned Trash executor. AI cannot approve or directly delete items. General conversational file moves and renames remain unsupported. ([7408b19](https://github.com/Dannykkh/bloomsweepy/commit/7408b19))
- **management**: Add reviewed folder Trash actions, safe file/folder opening and location reveal, installed-app search and platform-specific removal entry points. macOS app-bundle removal and identifier-matched related-data review are separate; Windows opens official uninstall settings. ([7408b19](https://github.com/Dannykkh/bloomsweepy/commit/7408b19))
- **macos**: Add a native AppKit menu-bar popover with bounded CPU/RAM/disk sampling, persisted RAM-title visibility, window restore and quit controls, without an additional WebView. ([7408b19](https://github.com/Dannykkh/bloomsweepy/commit/7408b19))
- **trash**: Add separately confirmed operating-system Trash emptying. This is irreversible, includes other apps' trashed items, and is not exposed to AI/CLI/MCP. ([7408b19](https://github.com/Dannykkh/bloomsweepy/commit/7408b19))

### Performance and safety

- **windows**: Wait for the root separator before inspecting a verbatim drive prefix during folder revalidation. Keep disposable action fixtures outside protected AppData and add a Windows regression without weakening protected-folder policy. ([2d5cb20](https://github.com/Dannykkh/bloomsweepy/commit/2d5cb20))
- **windows**: Normalize canonical local drive paths for file inspection and OS dispatch while retaining UNC/device/alternate-stream/alias rejection. Add canonical-path dispatch regression coverage and isolate folder-plan fixtures from protected AppData. ([b44eb14](https://github.com/Dannykkh/bloomsweepy/commit/b44eb14))
- **core**: Bound streaming traversal, metadata retention and index storage; add cooperative host-memory, available-memory and free-disk guards. Isolate document parsing behind a 128 MiB Rust allocation budget and a 15-second deadline, preserving the previous completed index after failed rebuilds. These are not a whole-process-tree OS memory quota or proof of leak elimination. ([be47f5d](https://github.com/Dannykkh/bloomsweepy/commit/be47f5d))
- **core**: Revalidate folder identity, protected/cloud/link boundaries and short-lived one-shot plans before OS Trash operations. Path-based OS calls retain a residual TOCTOU boundary. ([be47f5d](https://github.com/Dannykkh/bloomsweepy/commit/be47f5d))

### Documentation

- Align four README languages around conversational file management, local processing and AI data-sharing limits, token-efficiency caveats, new navigation and platform-specific behavior. Existing screenshots are explicitly labeled v1.6.0 references, not captures of the new menus. ([ac9e747](https://github.com/Dannykkh/bloomsweepy/commit/ac9e747))

### Verification and limitations

- Local Apple Silicon regression: 273 Rust tests and 43 frontend tests passed; one opt-in live CLI diagnostic was ignored. Formatting, workspace Clippy, TypeScript checking and the production frontend build passed. Release-commit CI results are linked from the GitHub release.
- Real Codex end-to-end operation of the new conversational tools, long-running app/WebView/CLI memory behavior, destructive operations on actual user apps/Trash and every native menu-bar interaction remain unverified. Windows CI is not interactive Windows runtime verification.
- App total size, installation date and selectable name/date/size sorting are not implemented; substring search is available. macOS ARM64 builds are ad-hoc signed and not Apple-notarized; Windows installers are unsigned.
- See [v1.7.0 scope and verification boundaries](docs/releases/v1.7.0.md).

## [1.6.1] - 2026-09-06

### Fixed

- **build**: Align the shared Tauri configuration with the common `macos-private-api` Cargo feature so Windows `cargo clippy` and test builds no longer fail the feature/configuration consistency check. Preserve macOS native glass and opaque window defaults on other platforms. ([738cd1d](https://github.com/Dannykkh/bloomsweepy/commit/738cd1d))

### Tests

- Add regressions for the shared feature/configuration contract and platform-specific window materials. The frontend suite now contains 37 tests. ([738cd1d](https://github.com/Dannykkh/bloomsweepy/commit/738cd1d))

The v1.6.0 Mac release and tag remain unchanged. Its Windows CI failed before installers were produced; v1.6.1 carries the build fix. The UI is unchanged, so the v1.6.0 documentation screenshots still apply. Platform-specific runtime and signing limitations remain as documented below.

## [1.6.0] - 2026-09-06

### Features

- **desktop**: Make the Rust/Tauri app the primary experience, with glass-inspired navigation, large one-click scan actions, and animated multi-drive cards. ([cb6cf47](https://github.com/Dannykkh/bloomsweepy/commit/cb6cf47))
- **performance**: Add CPU and memory rings, bounded top-app metrics, smooth sample transitions, and reduced-motion support. On macOS, expose current-process allocator relief and separately confirmed normal app-exit requests; unsupported platforms stay read-only. ([cb6cf47](https://github.com/Dannykkh/bloomsweepy/commit/cb6cf47))
- **treemap**: Add individual-file review actions backed by scan-time identity, path and content revalidation, and the existing Trash journal. Empty-folder discovery stays read-only. ([e4f3ad8](https://github.com/Dannykkh/bloomsweepy/commit/e4f3ad8), [cb6cf47](https://github.com/Dannykkh/bloomsweepy/commit/cb6cf47))

### Fixed

- **scanning**: Exclude known cloud-sync roots and online-only entries before recursive traversal in storage scans, drive summaries, treemaps, file catalogs, and document indexes. Reject direct cloud-root selection and cloud aliases without misclassifying cloud-only parents as empty. ([e4f3ad8](https://github.com/Dannykkh/bloomsweepy/commit/e4f3ad8))
- **assistant**: Distinguish missing, broken, incompatible, and signed-out CLI providers; skip unusable launch candidates and preserve actionable provider errors, cancellation, and conversation history. ([cb6cf47](https://github.com/Dannykkh/bloomsweepy/commit/cb6cf47))
- **volumes**: Hide macOS disk-image installer volumes while preserving physical and read-only media, and stabilize selected-drive path matching. ([cb6cf47](https://github.com/Dannykkh/bloomsweepy/commit/cb6cf47))

### Documentation

- Refresh all four READMEs with 20 localized captures of the real React views using synthetic data, plus a reproducible documentation-only preview. Keep Swift as a legacy reference and clarify CLI installation, AI data sharing, cloud exclusions, and memory-cleanup limits. ([378ec22](https://github.com/Dannykkh/bloomsweepy/commit/378ec22))

### Verification and limitations

- Local Apple Silicon macOS validation: 193 Rust tests and 35 frontend tests passed; one opt-in live CLI diagnostic remains ignored. Rust formatting, workspace clippy, TypeScript checking, and the frontend production build passed. Both platform CI workflows now run the complete frontend test set. ([e4f3ad8](https://github.com/Dannykkh/bloomsweepy/commit/e4f3ad8), [cb6cf47](https://github.com/Dannykkh/bloomsweepy/commit/cb6cf47))
- The installed Mac app was checked with synthetic local/cloud scan fixtures and the Codex conversation flow. This is not verification of every provider, real cloud-service configuration, macOS login cycle, or normal app-termination flow. Current Windows runtime verification remains separate.
- App-memory cleanup affects only unused malloc pages owned by the BroomSweepy host process, not system RAM, other applications, WebView helpers, swap, or leaks. A zero-byte allocator return is a valid result. The Mac distribution build is ad-hoc signed and is not Apple-notarized.

## [1.5.0] - 2026-09-03

### Features

- **startup**: Let users explicitly enable or disable launch at login on Windows and macOS, with the operating-system registration state rechecked after every change. (`b86a689`)
- **memory**: Add a read-only system-memory panel for total, available, used, and platform-reported swap metrics. (`b86a689`)

### Reliability

- Start login launches with the main window hidden from creation time, keep one Windows process through an early named mutex and foreground event, and restore the existing window on a normal second launch. (`b86a689`)
- Move Windows tray and window mutations away from nested Wry main-thread dispatch paths that could stall hide, restore, or exit handling. (`b86a689`)

### Safety

- Do not expose Working Set trimming, cache pressure, other-process purging, or memory-leak cleanup as a memory-cleaning action. (`b86a689`)
- Label the Windows swap value as a commit-based estimate rather than current pagefile usage. (`b86a689`)

### Documentation

- Document startup defaults, platform lifecycle behavior, memory-metric limits, and the boundary with the legacy SwiftUI memory-pressure helper in all four README languages. (`b86a689`)

### Testing

- Validate the Windows hidden-start, background and foreground second-launch paths, tray close and restore behavior, early-launch race, Rust workspace, frontend catalogs, and MSI/NSIS packaging. (`b86a689`)

## [1.4.0] - 2026-09-03

### Features

- **localization**: Use English by default and let users switch the desktop UI, Windows tray menu, number and date formatting, and requested AI response language between English, Korean, Japanese, and Simplified Chinese. (`3c2384d`)
- **branding**: Replace the app icon set with a clearer broom silhouette that remains recognizable at taskbar, tray, Dock, and installer sizes. (`3c2384d`)

### User Interface

- Localize the main screens and progress descriptions while retaining one installer and storing the selected display language only on the current computer. (`3c2384d`)
- Show `BroomSweepy 1.4.0` in the native window title and use English as the first-run language. (`3c2384d`)

### Documentation

- Add English, Korean, Japanese, and Simplified Chinese README navigation with matching settings screenshots and consistent safety guidance. (`3c2384d`)

### Testing

- Validate all 846 localized message keys and placeholders, four-language switching and persistence, compact-window overflow, Rust desktop behavior, and Windows installer packaging. (`3c2384d`)

## [1.3.0] - 2026-09-02

### Features

- **mcp**: Package the local MCP helper with the desktop app and add explicit Codex and Claude Code registration controls. (`c6bfbad`)
- **cleanup**: Let external AI request pathless cleanup summaries and short-lived review plans while keeping exact paths and final approval inside BroomSweepy. (`c6bfbad`)
- **desktop**: Show the package version in the native window title. (`c6bfbad`)

### Safety

- Keep approval, deletion, trash movement, registry mutation, and trash emptying out of the MCP tool surface. (`c6bfbad`)
- Revalidate the current scan generation and file state before reusing the existing journaled operating-system trash executor. (`c6bfbad`)

### Build

- Validate the versioned MCP sidecar inside Windows MSI and NSIS bundles and in the macOS build workflow. (`c6bfbad`)

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
