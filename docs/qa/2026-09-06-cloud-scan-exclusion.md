# Cloud subtree exclusion — macOS fix

2026-09-06. **Installed and verified on this Apple Silicon Mac.** `/Applications/BroomSweepy.app` now contains the cloud exclusion fix. Full automated rerun: 193 Rust + 35 frontend = **228 passed**, one opt-in CLI diagnostic ignored. Native synthetic-folder checks passed; this is not a claim of all cloud providers or Windows runtime validation.

## Cause and immediate containment

The existing TypeScript `cloudVolumePolicy` filters dashboard volume cards (including truncated Windows Google Drive labels). None of the five Rust scan/index walkers used it or an equivalent subtree guard. A scan rooted at `/` therefore entered cloud directories beneath user homes. Google documents the File Provider location as `~/Library/CloudStorage` in its [macOS guide](https://support.google.com/drive/answer/12178485?hl=en). Apple also documents [online-only iCloud files and downloads](https://support.apple.com/en-euro/guide/mac-help/mchl1a02d711/mac).

The actual installed app was scanning `/`, showing 4,746,257 files / 535.4GB and full-content duplicate verification. Used the dashboard's `검사 중단`; the native accessibility state subsequently reported `파일 검사를 취소했습니다.` and re-enabled `이 드라이브 검사`. These totals do not measure cloud downloads. Filesystem free space was 324MiB initially and 175MiB after bounded compilation. No claim is made that cloud hydration caused the disk shortage.

Memory search did not find a historical Windows recursive-exclusion implementation. Current source and Windows-named regression tests establish the earlier **volume-list hiding** behavior, not complete Windows traversal safety.

## Changes

- New shared `scan_policy.rs` lexically excludes `Library/CloudStorage`, `Library/Mobile Documents`, and known legacy provider roots directly under user homes or `/Volumes`; supports multiple accounts/users and `/System/Volumes/Data` aliases.
- jwalk `process_read_dir` removes entries **before** child directory jobs are scheduled. Filtering only yielded results would be too late for a parallel walker.
- Applies to duplicate/large-file scan, drive category scan, treemap, portable filename catalog and document content indexing. Windows NTFS name-catalog full/incremental record filtering also uses the lexical policy.
- Direct root selection is guarded before metadata/canonicalization of known cloud paths. Canonical roots are checked again so an innocently named symlink cannot bypass the policy. Rejected roots do not create an index.
- macOS `SF_DATALESS` and Windows offline/recall metadata flags exclude placeholders. Content opening rechecks the policy before hashing, comparing or extracting. Ordinary local sparse files are not classified as cloud merely because allocation is small.
- Treemap emptiness is checked before cloud filtering: a parent with only excluded children is not offered as an empty folder.
- Dashboard explains the scan exclusion in all four existing UI languages. No layout or provider authentication changes.
- Existing indexes are replaced by the normal successful refresh generation; the new integration test confirms files moved under a cloud root disappear on refresh. Previously cached results are not proactively erased or silently treated as new scan results.

## Verification

| Check | Result |
|---|---|
| Core unit tests, `CARGO_INCREMENTAL=0 cargo test -p bloomsweepy-core --lib` | 47 PASS, including 3 new policy tests |
| `cloud_scan_exclusion` integration tests | 4 PASS |
| New cloud integration fixture | Nine cloud subtree variants; exactly 2 local files counted across all 5 engines, cloud content excluded, no false empty folders |
| Before-descent assertion | PASS: callback never observes cloud read_dir scheduled; direct cloud content open returns PermissionDenied |
| Direct root and symlink alias rejection | PASS; index database not created |
| Existing local entry moved into cloud, catalog/document refresh | PASS; entry removed from both refreshed indexes |
| Frontend `npm run test:all` | 35 PASS, including existing Windows volume tests |
| TypeScript / production frontend build | PASS; pre-existing >500KB chunk advisory remains |
| Core clippy all-targets with warnings denied / rustfmt / whitespace | PASS |
| New native app build/install | PASS — ARM64 Tauri app and bundled MCP sidecar, ad-hoc signed, installed bundle matches build |

The first focused pass had 86 tests. After the user approved cache removal, `CARGO_INCREMENTAL=0 cargo test --workspace --quiet` passed **193** Rust tests (17 control, 47 core unit, 4 cloud, 2 index responsiveness, 1 macOS resource, 112 desktop, 10 MCP); the frontend rerun passed all **35**. The macOS resource test used its default 20 cycles in this rerun. The earlier 1,000-cycle report remains a pre-fix baseline, not a repeated 1,000-cycle claim. Full-workspace clippy with warnings denied, rustfmt, whitespace, TypeScript and production build also passed.

## Installation and native verification — 12:03–12:10 KST

- User approved removal of **only** `/Users/dannysmacair/Documents/git/BroomSweepy/target/debug/incremental`, measured at 5.7GB. Its canonical path and directory identity were checked; no running cargo/rustc process was found before removal. A forced-removal command was rejected by the tool; plain non-force `rm -r` then removed the exact approved cache successfully. Build later recreated an empty 0B incremental directory. This regenerable cache was permanently removed; source and user data were not.
- Filesystem free space was 1.2GiB at the beginning of this continuation, 4.6GiB after removal, and 4.2GiB after builds/tests. These observations differ from the earlier 175MiB sample; no cloud-download attribution is made.
- `CARGO_INCREMENTAL=0 TAURI_ENV_TARGET_TRIPLE=aarch64-apple-darwin npm run tauri -- build --target aarch64-apple-darwin --bundles app` passed. App and MCP binaries both report Mach-O arm64; bundled MCP reports 1.5.0.
- Applied local ad-hoc signing to the generated app and verified strict/deep signatures. Confirmed no active scan/request and an empty chat draft, quit normally, then moved the previous app to `/Users/dannysmacair/.Trash/BroomSweepy-before-cloud-exclusion-20260906-1205.app` and installed the new bundle. Installed signature and binary `cmp` passed again. This is not Developer ID signing/notarization.
- Native fixture `/tmp/broomsweepy-cloud-qa.YIiHHS/disk` contained two 11-byte local text files plus three separate files under mock Google Drive (`Library/CloudStorage`), iCloud (`Library/Mobile Documents`) and OneDrive roots. These are ordinary test-owned directories, not live provider locations.
- Selected the fixture through the installed app's native folder picker. Treemap showed **22B, 2 files, 0 empty folders, 0 access errors**. Screenshot visually checked. The complete large-file/duplicate workflow also completed; dashboard showed **2 files checked**. Files were below the user's existing size thresholds, so native large/duplicate result lists were correctly empty; actual duplicate hashing is covered by the automated low-threshold fixture, not claimed from this UI run.
- Directly selected the mock Google Drive root: installed app returned `클라우드 동기화 폴더와 온라인 전용 항목은 검사하지 않습니다. 로컬 폴더를 선택해 주세요`, with controls responsive. No real cloud provider was browsed for validation.
- Quit, removed only the five owned fixture files and their temporary directory, and reopened. Temporary root/error state cleared. Existing chat still contains **17 messages** and the prior answers. Selected Codex as previously requested, observed `Codex · CLI 준비됨`, and left the app on the dashboard with no scan or AI request active. No new AI message, authentication change, saved-index deletion or user-file cleanup occurred.

## Limits

The approved build/install and native synthetic smoke are complete. A real whole-drive rescan was not automatically launched. No commit/push or Windows installation occurred.

Actual Windows runtime, live cloud-provider hydration, custom arbitrarily named mirrored folders outside known roots, and eviction between the final metadata check and OS open are not fully verified/guaranteed. This is not a general filesystem sandbox. Cleanup-candidate traversal and OS trash semantics were not redesigned in this change. Existing saved indexes adopt the policy on a successful refresh; cached results are not proactively erased.
