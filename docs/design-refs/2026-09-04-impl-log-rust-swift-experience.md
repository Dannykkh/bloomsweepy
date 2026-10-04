# Implementation Log: Rust + Swift Experience

## Scope

- Rust/Tauri remains the only product and release base.
- The SwiftUI build is a visual and interaction golden master only.
- This pass concentrates on the first-view dashboard, navigation hierarchy, native macOS material, one-click scan entry, and accessible waiting/error states.

## Product Design Gate

- Exact Product Design MCP: absent.
- Adapter: local Swift reference screenshot and repository-native implementation.
- Three-direction exploration: exempted because the user explicitly selected the existing SwiftUI product as the target direction.

## Implemented

- Replaced the dense dashboard opening with a large storage ring, one dominant full-scan action, a cleanup-candidate action, and four result-oriented quick actions.
- Kept large-file bytes, confirmed duplicate waste, likely-safe cleanup bytes, and review-required bytes as separate evidence instead of inventing a combined reclaimable total.
- Collapsed global navigation into Dashboard, Space cleanup, File management, AI assistant, optional Docker management, and Settings; leaf routes remain available through local section navigation.
- Added macOS native `underWindowBackground` material, transparent canvas, transparent titlebar, and active-window material state.
- Kept destructive work behind the existing explicit confirmation, revalidation, Trash, and journal path; dashboard actions only scan or navigate.
- Added stacked scan errors, visible cancel actions, reduced-motion view-transition guard, phase-only live announcements, mobile drawer focus restoration, inert background handling, and long-locale heading balancing.
- Added a multi-drive deck: the system drive starts as the single large ring, physical sibling drives appear as compact ring cards, and selecting one swaps the card roles without scanning or clearing the current report.
- Bound the primary dashboard scan explicitly to the currently enlarged drive, with a root override that avoids stale React state after selection.
- Added deterministic system/fixed-drive fallback, Windows mount normalization, cloud-volume exclusion, macOS internal-mount suppression, disappearing-external-drive handling, focus restoration, and reduced-motion behavior.
- Replaced the subtle snapshot crossfade with an interruptible 460ms FLIP exchange after user review: the compact card expands from its measured rail rectangle while the previous large card shrinks into the vacated compact slot; only compositor transforms animate.

## Render Review

- Reference: `demo-assets/01_dashboard.png`.
- Previous Rust render: `docs/assets/screenshots/v1.2.0-dashboard.png`.
- Current local render reviewed at 1280 px width through the in-app browser. The main hierarchy now reads as navigation → storage ring → two scan actions → quick actions → supporting evidence.
- Iteration 1 exposed overlapping full-scan and cleanup-scan errors; they now stack in one error region.
- Iteration 2 exposed early English title truncation in quick actions; titles now wrap while descriptions remain two-line clamped.
- Native process launch was verified from `/Applications/BroomSweepy.app`. Native material cannot be captured by the browser renderer, so final vibrancy contrast still requires a human glance on the running window.
- Multi-drive fixture review at 1280px confirmed one dominant ring plus a compact vertical rail; selecting `Projects` promoted it to the large card and returned `Macintosh HD` to the small rail. At 580px the rail moved below the large card for horizontal access.

## Verification

- `npm run check`: pass.
- Frontend policy, volume, Docker intent/format, assistant text, and locale tests: 24 passed, 0 failed.
- Production Vite build: pass; existing chunk-size advisory remains for the 648 kB main bundle.
- `cargo fmt --check`: pass.
- `cargo check --workspace`: pass, 0 warnings.
- `cargo test --workspace`: 157 passed, 0 failed.
- `cargo clippy --workspace --all-targets -- -D warnings`: pass, 0 warnings.
- ARM64 app-only Tauri bundle: pass.
- Main executable and MCP sidecar: both ARM64, version 1.5.0.
- Local ad-hoc code signature: deep strict verification passed.
- DMG: checksum verification passed.
- Installed app: launched from `/Applications/BroomSweepy.app` and remained running without error/fault logs.

## Packaging Notes

- This host has ARM64 macOS and Rust but an x86_64 Node/Tauri CLI. The deterministic local build command is `npm run tauri -- build --target aarch64-apple-darwin --bundles app`.
- Direct macOS distribution may use the private native-material path. Mac App Store distribution must use a separate opaque build flavor because Tauri's transparent macOS window path requires private API support.
- The local artifact is ad-hoc signed for installation testing. Public distribution still requires a Developer ID signature and notarization.
