# v1.8.0 release verification — 2026-10-21

Release source/tag: `2d34e8ec587109ca0d8ce2339ca7add2f32d2b4d`. This report is a post-build documentation record; it does not change that tested source.

## Publication

[v1.8.0](https://github.com/Dannykkh/bloomsweepy/releases/tag/v1.8.0) was published at 2026-10-21 00:38:38 UTC (09:38:38 KST). GitHub reports draft=false and prerelease=false; the latest-release endpoint resolves to v1.8.0. The remote annotated tag peels to the exact tested source above. All four uploaded assets have the expected byte sizes and SHA256 digests, including `SHA256SUMS.txt` (292 bytes, `1dc32b1e2aad0cf86134dd817324b16fff7d57d83672e95bc82a3c1c9963e285`). The published notes include the native verification and remaining document-title limitation. No old release assets were replaced.

## Exact-commit CI

- [macOS 39518413639](https://github.com/Dannykkh/bloomsweepy/actions/runs/39518413639): success; Rust494 PASS/6ignored, frontend192 PASS. fmt/Clippy/type/build, ARM64 app/helper1.8.0/strict ad-hoc signatures, heap rejection, DMG verify/checksum/upload and legacy Swift checks passed.
- [Windows 39518413628](https://github.com/Dannykkh/bloomsweepy/actions/runs/39518413628): success; Rust452 PASS/9ignored, frontend192 PASS. fmt/Clippy/type/build, MSI/NSIS generation and included/versioned MCP+document worker/heap checks passed. No interactive Windows installation was performed.
- Artifacts: Mac12310331519, Windows12311081852; both report the exact source SHA above and expired=false. Artifact ZIP digests are distinct from installer file digests.

## Downloaded files

| File | Bytes | SHA256 |
|---|---:|---|
| BroomSweepy_1.8.0_aarch64.dmg | 15448711 | f1f48f46bb6e990ec69ada383690381c9da9b8cdfd57a6395b85cb5897e5c68f |
| BroomSweepy_1.8.0_x64_en-US.msi | 13115392 | c8594ae0e37b1f112fff5c784891a7d9e7c0da94b7a6efae11140076abdae283 |
| BroomSweepy_1.8.0_x64-setup.exe | 9742049 | 97e7d96d1d2ee48cb522bfd0b46335d0e70d51da35c126ce439a5aa9002c0031 |

All match CI file checksums; aggregate `SHA256SUMS.txt` is292bytes and local `shasum -c` passes for all3files. Mac DMG `hdiutil verify`, read-only mount, identifier `com.broomsweepy.desktop`, plist1.8.0, all3executablesARM64 and deep/strict signatures passed on this Mac. Included helper `--version` commands return1.8.0 and document worker `--check-heap-budget` returns `heap-budget-rejection-ok`. The verification mount was detached.

## Installed Mac and screen

The old1.7.0 app was quit through its native Quit menu, kept recoverably at `/private/tmp/broomsweepy-install-1.8.0-PzVM0y/Previous-BroomSweepy.app`, and replaced with the verified DMG app. The closed assistant-sessions/permission-settings/developer-tools databases were byte-identical across the swap; no contents were printed or modified. Reopening uses the exact `/Applications/BroomSweepy.app` path because multiple old build copies share the bundle ID.

The real1.8.0 empty system screen, completed status/summary and scrolled candidate rows retain full width, wrap paths and keep controls in their own columns. A read-only scan completed in4.6s over67,157entries, yielding38candidates and an explicit partial-result warning before selection. This is not full disk coverage. Selection0/deletion0; no native destructive action was tested. [Screen evidence17/18](system-candidates-2026-10-21/review.md) is separate from synthetic data; private row screenshots were not committed.

## Limits

The local full package build was intentionally interrupted under severe8GiB-host swap pressure; exact-commit remote packaging was used instead. Local release all-target Clippy/fmt, frontend192 and worker boundary/overflow/concurrency3tests passed. This is not a whole-process memory-soak result or OS hard quota.

No notarization, Windows interactive runtime, actual Finder writes/restores or full new-package Codex end-to-end pass is claimed. Existing chat-provenance/document-disclosure and modal focus-return gaps remain. Plist/window version is1.8.0, but the embedded HTML document title still says1.7.0; this non-window metadata follow-up remains open. No source or installer was silently patched after CI.

#tags: release-1.8.0, ci-artifacts, installed-mac, system-candidates, arch:020
