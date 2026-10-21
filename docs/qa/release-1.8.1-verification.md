# v1.8.1 release verification — 2026-10-21

## Current status

User approved stable v1.8.1 publication, commit and push. Source changes are being committed, versions synchronized, and exact-commit CI/package verification prepared. No v1.8.1 release, tag, installer upload or local installation has yet been reported as complete. The existing v1.8.0 release/tag/assets remain unchanged.

## Local evidence before release version bump

- Menu-bar/i18n patched local1.8.0: frontend216PASS; native source-component12PASS; release-library Clippy, TypeScript, frontend build and ARM64 combined build/signature PASS.
- The user accepted the real native component preview's material/readability. The final installed app is still the earlier Regular1.8.0 because the Mac was locked before normal Quit. No force termination or lock bypass was used, and user databases were not swapped.
- These checks are not relabeled as v1.8.1 installed-package or interactive Windows tests.

## Release gates

1. App/four Rust packages/locks/Tauri and HTML titles/MCP initialization agree on1.8.1.
2. Exact release source passes Mac and Windows workflows, including frontend, workspace fmt/Clippy/tests and platform package/helper validation.
3. Mac CI DMG and Windows MSI/NSIS artifacts report that exact SHA; downloaded files match their hashes, Mac DMG verifies, helper versions/ARM64/signatures/heap boundary pass.
4. Tag points to the tested source. Publish stable/non-prerelease with all three installers and aggregate SHA256SUMS.txt, then re-read release metadata and asset digests.
5. Record local installation separately. Mac lock or absent interactive Windows must remain explicit, not block a user-approved disclosed release or become a false runtime pass.

## v1.8.1 local source checks

- TypeScript checking, frontend218/218 tests and production frontend build PASS after the version bump. The two additional tests cover all four Rust package/lock versions, npm lock metadata, Tauri/HTML titles and MCP initialization metadata.
- `cargo fmt --all -- --check`, `cargo metadata --offline --locked --filter-platform aarch64-apple-darwin --format-version 1` and `git diff --check` PASS. No dependency versions were upgraded.
- The Vite build retains its existing large-chunk warning; this is not a failed build or a completed memory-soak test.

## Remaining verification

CI, package checks, publication and1.8.1 local installation are pending at this snapshot. Mac ad-hoc signing is not notarization; Windows installers are unsigned. Full physical outside-click/direct-icon, pre26, Japanese/Chinese visual runtime and long memory-soak checks remain separate.
