# v1.8.1 release verification — 2026-10-21

## Current status

Stable/latest [v1.8.1](https://github.com/Dannykkh/bloomsweepy/releases/tag/v1.8.1) was published at 2026-10-21T08:00:51Z (17:00:51 KST). GitHub returned draft=false, prerelease=false and four uploaded assets with the verified digests below. The annotated tag peels to tested source `96e1202b43ea36f3364c23d306e6ea84fe44ff5a`. This Mac was also replaced with the exact CI DMG app and launched. The existing v1.8.0 release/tag/assets remain unchanged. Subsequent documentation-only commits do not move the tag or replace its tested binaries.

## Local evidence before release version bump

- Menu-bar/i18n patched local1.8.0: frontend216PASS; native source-component12PASS; release-library Clippy, TypeScript, frontend build and ARM64 combined build/signature PASS.
- The user accepted the real native component preview's material/readability. At that earlier snapshot the Mac was locked and still ran Regular 1.8.0; no force termination or lock bypass was used. The later normal Quit and 1.8.1 replacement are recorded separately below.
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

## Exact-source CI and original artifacts

| Workflow | Source | Result |
|---|---|---|
| [macOS 39556095853](https://github.com/Dannykkh/bloomsweepy/actions/runs/39556095853) | `96e1202b43ea36f3364c23d306e6ea84fe44ff5a` | Entire workflow PASS, including workspace fmt/all-targets Clippy/tests, frontend 218/218, native bundle/DMG and legacy Swift checks |
| [Windows 39556095857](https://github.com/Dannykkh/bloomsweepy/actions/runs/39556095857) | Same exact SHA | Entire workflow PASS, including workspace fmt/all-targets Clippy/tests, frontend 218/218, MSI/NSIS/helper presence/version/heap checks |

Original downloaded ZIP SHA256 values match the GitHub artifact digests. Both artifacts were non-expired and their names/workflow metadata contained the exact source SHA:

- Mac artifact 12325963777: `57e7baf67f1d2c40989faf226368fd90c4b5ad7b69f5e2819e469bcb4b2db3e8`.
- Windows artifact 12326044487: `f7053e30c8937cccdbc112d15156b6454c3924cf02254a7a1d8464ffd66c9426`.
- ZIP integrity checks passed. The downloaded Mac checksum sidecar verified; `hdiutil verify` passed. Read-only DMG mounting confirmed bundle ID `com.broomsweepy.desktop`, version 1.8.1, all three ARM64 executables, both helpers' 1.8.1 versions and `heap-budget-rejection-ok`. Strict/deep ad-hoc signature verification passed. Windows installer hashes also match their CI log values. No installer was rebuilt or re-signed after these checks.

## Published asset hashes

| Asset | Bytes | SHA256 |
|---|---:|---|
| `BroomSweepy_1.8.1_aarch64.dmg` | 15473953 | `f389cc5abfa865ed2d07e2f5720e9cde12f35dd0fc1eb09980bb207db4969027` |
| `BroomSweepy_1.8.1_x64-setup.exe` | 9734544 | `13cb54e7888ec4e24e24c5d79b29aea3b52313fb2790e65541f031c78cc0cd40` |
| `BroomSweepy_1.8.1_x64_en-US.msi` | 13148160 | `832e3d170399f072109b8f42f1fb5478d3ce9f1e305a750d244d0c65b4201bd9` |
| `SHA256SUMS.txt` | 292 | `ddfb36aced551d0a56b0896387addc6f7efc5b2ca54159b60c1db376d4e7a85a` |

All four uploaded asset digests were compared before publishing, then the public latest-release endpoint was re-read. Remote tag object `8b0a6fb481faf61047100f45ccf34fc8bd1be4e6` peels to the tested SHA above.

## Actual Mac installation and focused runtime checks

- Existing app normally quit with CmdQ. Exact host process absence confirmed; no force kill or lock bypass. Previous installed bundle remains recoverable at `/private/tmp/broomsweepy-release-1.8.1.UFtVYX/Previous-Installed-BroomSweepy.app` (temporary recovery location, not a permanent backup).
- DMG app copied without re-signing to `/Applications/BroomSweepy.app`. Installed main SHA256 `cea692c9b44b10af5bc2bf702bb3ebfae8004c70425c93cbdfa177b230c0f257` matches the staged original. Installed strict/deep signature passed. DMG detached normally.
- Conversation, permission and developer-tool SQLite DBs were byte-identical before/after replacement while closed; 3/3 PASS, hashes kept private and DB contents not opened. App startup may subsequently perform its ordinary writes; this is not a claim that every application-support file is immutable.
- Actual native launch and embedded HTML both advertise 1.8.1. English remains selected; existing autostart/memory-display settings remain On and were not toggled. This is not a tested login cycle.
- [Actual English Mac preferences](i18n-2026-10-21/02-installed-1.8.1-english-mac.png): previously Korean warning is now English. Installed app rows also expose English protected/system/helper reasons. No Finder value, scan target, cleanup selection or permission was changed.
- [Actual installed bubble](menu-bar-2026-10-21/09-installed-1.8.1-bubble.png): beak, three colored gauges and English controls/version are present. Actual AX progress values and live sampling are visible; Settings-open, reopening, Esc-close and Open-main return PASS. Window-only gray/white backdrop does not reproduce desktop glass compositing and is not final physical/material acceptance.
- Installed sidecar MCP stdio initialize returns serverInfo.version 1.8.1 and tools/list returns 12 tools; no app action or file access was requested by that protocol check. Not a new end-to-end AI/deletion test.

## Remaining verification

Publication and focused Mac installation checks above are complete. Mac ad-hoc signing is not notarization; Windows installers are unsigned. Full physical outside-click/direct-icon and actual desktop material, pre26, native Windows GUI/installation, Japanese/Chinese visual runtime and long memory-soak checks remain separate. Earlier failed popover/panel trials remain historical evidence, not passes copied to this installed revision.
