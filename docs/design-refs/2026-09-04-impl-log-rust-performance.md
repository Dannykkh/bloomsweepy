# Implementation Log: Rust Performance Instrument

## 2026-09-05 Smooth-value Motion Follow-up

- Added a 900ms ease-out CSS transition only to performance-meter `stroke-dashoffset`, with an explicit reduced-motion `transition:none` override.
- Kept existing snapshot cadence, actual numeric values, stable component identity, backend semantics, and equal-ring layout unchanged.
- Added opt-in moving fixture data for runtime start/mid/end and retarget checks. Evidence and limitations are in [the motion contract](2026-09-05-motion-performance-rings.md).
- Rebuilt, signature-verified and installed the macOS ARM64 application; native Performance screen launch passed. Previous install was retained in Trash at `BroomSweepy-before-smooth-rings-20260905-114430.app`.

## 2026-09-05 Equal-ring Delta

- Shared system-performance heading, equal-width CPU/RAM panes, equal-sized semantic meters; RAM replaces the old bar with a blue ring.
- CPU contains core count, sample time, and a work-reduction explanation without a fictitious CPU-clean action. RAM contains used/total, available, and swap bytes.
- Renamed the CTA to `앱 메모리 정리` with `BroomSweepy 전용` visible at every breakpoint; kept current-process allocator semantics and process-termination safeguards unchanged.
- Updated Korean/English/Japanese/Chinese labels and existing fixture locale/zero-return controls. Existing frontend tests (32), TypeScript, and Experience Contract validation passed. No Rust behavior changed in this delta.
- Actual UI geometry and Enter/pointer activation were checked through the browser; final installed-build evidence is appended after packaging below.

### Equal-ring Installed Verification

- Final ARM64 Tauri bundle and production TypeScript/Vite build passed. Strict/deep ad-hoc signature verification passed for the built and installed bundles; installed host binary matches the built binary byte-for-byte.
- `/Applications/BroomSweepy.app` 1.5.0 was relaunched and its Performance view visually inspected at 1154×768. Both equal circular meters showed actual Mac data (CPU 19%, RAM 62%, 4.9/8 GB used at the captured instant), with available and swap bytes below.
- One real `앱 메모리 정리 · BroomSweepy 전용` click completed successfully with zero allocator-releasable bytes; the installed UI reported that exact outcome. Other applications were not terminated.
- Previous install remains recoverable at `/Users/dannysmacair/.Trash/BroomSweepy-before-dual-rings-20260905-1138.app`.
- Existing Vite >500 kB chunk-size advisory remains; no new Rust behavior or permissions were introduced. Rust tests were not rerun for this UI-only delta (the earlier 170-test run below belongs to the preceding implementation).

## Planned Scope

- Persistent Rust `sysinfo` sampler for global CPU, memory, swap, and bounded process usage.
- Cross-platform read-only metrics with macOS-only normal GUI-app termination for the first release.
- Opaque snapshot/target identity, expiry and execution-time revalidation; no raw PID command.
- Glass `성능` view, dashboard status entry, stale/error states and cancel-first confirmation dialog.
- macOS-only one-click current-process allocator relief with no pressure allocation, plus honest busy/unsupported/zero-byte states.

## Product Design Gate

- Product Design plugin: absent.
- Adapter: local React/Tauri using the user-selected SwiftUI golden master.
- Three-direction exploration: exempted because the direction was already selected by the user.

## Verification Ledger

- [x] Experience Contract validator
- [x] Rust workspace tests and Clippy with warnings denied
- [x] TypeScript check, all frontend tests, catalog tests and production build
- [x] 1280×820 release-CSS fixture render review
- [x] 760×600 responsive render review
- [x] semantic table, dialog focus/Escape restoration and reduced-motion code review
- [x] ad-hoc signed macOS ARM64 installation launch
- [x] one-click memory action at 1280×820 and 760×600, including live-region and no-horizontal-overflow audit
- [x] installed-app macOS allocator-relief smoke invocation

## Implemented Result

- `PerformanceMonitorState` keeps one `sysinfo::System` sampler so CPU values are based on elapsed samples instead of a new zero-history object on every request.
- The snapshot contains actual global CPU, memory, swap and a bounded union of the highest CPU/RAM processes. macOS GUI helper processes are grouped under their running application where an identity can be established.
- The frontend adds a glass `성능` route with a large CPU ring, a single-color memory rail, CPU/RAM sorting, stale/error recovery and a semantic process table.
- macOS termination is a two-step, normal AppKit request. Opaque target and preview IDs expire and are single-use; the `Retained<NSRunningApplication>` captured with the snapshot is kept through preview and execution, revalidated, and asked to `terminate()` without a PID re-lookup. Force termination is not implemented.
- The old Swift memory-pressure routine was not ported. `clean_app_memory` calls only `malloc_zone_pressure_relief(NULL, 0)` for the current BroomSweepy host process, rejects overlap with an atomic lease and returns stable completed/busy/unsupported outcomes.
- The memory pane adds a full-width one-click action, progress lock, positive/zero-byte result copy and platform capability. Only the allocator-reported bytes are presented as returned; RSS and system available-memory observations are not labeled as reclaimed memory.

## Evidence

- Desktop: `docs/assets/screenshots/v1.5.0-performance-1280x820.png`
- Compact: `docs/assets/screenshots/v1.5.0-performance-760x600.png`
- Confirmation: `docs/assets/screenshots/v1.5.0-performance-termination-dialog-1280x820.png`
- Installed app: `/Applications/BroomSweepy.app`, version `1.5.0`, bundle ID `com.broomsweepy.desktop`, ARM64, strict/deep signature verification passed and process remained running after launch.
- Recoverable prior install: `/Users/dannysmacair/.Trash/BroomSweepy-before-performance-2026-09-04-1550.app`.
- Recoverable pre-hardening performance build: `/Users/dannysmacair/.Trash/BroomSweepy-performance-pre-hardening-2026-09-04-1625.app`.
- Recoverable pre-memory-clean install: `/Users/dannysmacair/.Trash/BroomSweepy-before-memory-clean-2026-09-05-105026.app`.
- Recoverable pre-accessibility-polish intermediate: `/Users/dannysmacair/.Trash/BroomSweepy-memory-clean-pre-a11y-2026-09-05-105536.app`.
- Final verification: Rust workspace 170 tests, frontend 32 tests, performance presentation 6 focused tests, TypeScript, production build, Experience Contract validation and Clippy with warnings denied all passed.
- Installed smoke: the public allocator symbol linked in the ARM64 binary; a real one-click call completed with zero releasable bytes and the UI showed the explicit zero-byte success state before refreshing the snapshot.
