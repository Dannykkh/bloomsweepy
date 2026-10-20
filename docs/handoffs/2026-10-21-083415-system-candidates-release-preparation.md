# System candidates and v1.8.0 preparation

- Time: 2026-10-21 08:34 KST
- Project: BroomSweepy
- Branch: main
- Origin: User approved stable v1.8.0, then requested system-candidate screen inspection and repair before release. Exact session ID unavailable; scope is this explicit request, not the daily conversation backlog.

## Current state

System candidates repair is committed locally as `03b644c`, not yet pushed. Version files and four READMEs/CHANGELOG/release notes are being prepared for 1.8.0. No v1.8.0 tag or release exists yet. Private `.codex/`, `demo-assets/` and the excluded daily conversation files are untouched.

## Feature / Flow / Decision Snapshot

The existing AppShell → cleanup workspace → CleanupView composition is preserved. An explicit full-width grid boundary prevents nested inline-size container collapse. Platform-specific source filters and copy, named row areas, readable wrap and early partial-result warnings improve system candidates; app-owned scan, selection, developer-cache acknowledgement and final Trash execution boundaries remain unchanged. One initial scan action replaces duplicate calls to action.

Actual installed 1.7.0 local development app was scanned read-only before repair. After-repair browser fixtures use actual React components with synthetic Mac/Windows data and reject native deletion. Evidence and 12 measured states: `docs/qa/system-candidates-2026-10-21/review.md`. Frontend 192/192, TypeScript and diff whitespace checks passed. No actual user files were selected or deleted. Shared confirmation dialog focus return is a known pre-existing gap, documented rather than claimed fixed.

## Release blocker and next actions

Existing remote macOS CI run `39512056620` failed Clippy on Rust 1.99.1, before packaging. Windows run `39512056627` succeeded but does not verify this future release commit or interactive runtime. Local Rust is 1.98.1. Fix only the concrete lint issues without changing safety policies: test-only wrappers, collapsible conditions, narrow argument-count annotations, needless borrow, and the worker allocator's deprecated atomic update. Preserve its 128 MiB live-request limit with a compatible compare-exchange loop and focused tests.

Own local Cargo test run was interrupted before result because source fixes invalidate it; rerun final tests and release Clippy serially with `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 CARGO_PROFILE_RELEASE_DEBUG=0`, offline/locked. ARM64 packaging, installed after-repair native check and exact-release-head CI/artifact validation remain pending. Disk last observed 8.1 GiB free; no parallel Cargo/debug cache growth.

Then finish package/release evidence, commit only owned paths, push via explicit HTTPS with command-scoped HTTP/1.1/postBuffer settings (stored SSH remote remains untouched), verify exact-head Windows/Mac CI, create annotated v1.8.0 and publish stable GitHub release with ARM64 DMG, Windows MSI/EXE and SHA256SUMS. Do not publish before validation. Earlier v1.7.0 artifacts stay unchanged.

Temporary official GH CLI and credential-safe runner are in `/private/tmp/broomsweepy-release-oLLiJc/`; credentials are never written or printed. Use `node /private/tmp/broomsweepy-release-oLLiJc/gh-auth.mjs ...`. Current installed app is still old 1.7.0; preserve databases/permissions and keep a recoverable app backup if replacing it.

## Session Memory Review

Project root verified by Git and existing architecture 019 contains substantive body. Last doctor visit 2026-09-29 is under 30 days; no duplicate doctor run. Existing decisions are preserved. New system-candidate repair evidence is in the committed QA report and DESIGN.md. No gotcha/learned backlog was distilled because exact session provenance is unavailable. Project-skill candidate: none; no project-local skill target established. Reusable layout boundary can be indexed after final packaging evidence settles, without creating a premature verification claim.

#tags: system-candidates, release-1.8.0, handoff, ci, verification
