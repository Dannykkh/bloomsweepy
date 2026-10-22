# Swift retirement and local repository cleanup

Date: 2026-10-22 KST. Scope: retire unused Swift code, remove the named reference app/profile files, and compress local Git without losing history. The initial local cleanup did not commit or push. A subsequent explicit user request authorized publishing the cleanup to remote `main` through a normal commit and push, not a new release or history rewrite.

## Before / after

| Item | Before | After / boundary |
|---|---|---|
| Swift code | 56 production files /15900lines | No current `.swift` sources |
| Legacy checkout | 75tracked source/resource files plus `project.yml`, about2.3MiB; generated Xcode project about60KiB | Moved to macOS Trash; historical source remains in [v1.8.1](https://github.com/Dannykkh/bloomsweepy/tree/v1.8.1/BroomSweepy) |
| Old plans | Root `plan.md` and six section documents | Preserved in [archive](../archive/swift-legacy/README.md); original Git blobs and moved files match |
| Local `.git` | 255860KiB | 232228KiB; reduction23632KiB, about23.1MiB |
| Reference app | `/Applications/RemoveMacAI.app`, about3.1MiB | Moved to Trash; not running, no forced termination |
| Downloaded profiles | Downloads and user-temp `RemoveMacAI.mobileconfig`,9694bytes each | Both moved to Trash |

Trash moves do not themselves free disk blocks. The existing Trash was not emptied. The current user's installed-profile list reported none; no device-wide profile or OS preferences were changed. Deleting a reference app does not reverse its earlier settings changes.

## Build composition

- Rust/Tauri/React sources in `apps/desktop/` and `crates/` have no dependency on the retired Swift folder/project. Independent Tauri icons, entitlements, native sources, MCP and document worker remain intact.
- Mac CI no longer installs XcodeGen or generates/checks/builds/signs Swift. SDK checks, Rust fmt/Clippy/tests, frontend build/tests, Tauri app, helper/version/heap/signature validation, DMG checks and exact-SHA artifact upload are preserved. Windows CI is unchanged.
- Two regression tests guard Swift-free packaging and existence/ownership of Tauri assets. Four README languages and active design/build/resident-shell diagrams updated. Old Swift parity and QA remain historical records.

## Git preservation

Executed `git -c pack.threads=1 -c pack.windowMemory=64m repack -a -d --keep-unreachable`. Valid loose/packed unreachable objects are included, not discarded. No reflog expiration, pruning, garbage-file deletion, filter-repo, rebase or force push.

The following fingerprints compare the repository immediately before and after local repacking, before the subsequent normal cleanup commit. That new commit intentionally advances `main` and adds objects/ref-log entries; it does not rewrite older commits or tags.

- All12210unique object IDs: sorted fingerprint `ce6616b7b70023998deaa78d57b2515feeeca11e83e3d28455d428b4c03e9c55` unchanged.
- All refs fingerprint `449110976e91f911459a9520aeb5b6586e054ae85ab14a7780754937ca5391d5` unchanged.
- Reflog fingerprint `7d3425dec20185bbe345da1803ef99e37e677bf15dfa1ed9c1647a7656e261a6` unchanged.
- HEAD remained `4b2177bc74d21ec409fa9a64416515d855e9f3d7` during repacking; tags/releases unchanged. Deleting current Swift files does not remove older GitHub commits.
- Two pre-existing temporary-object garbage files (about15.76MiB) were intentionally retained.

## Actual checks and limits

- Initial frontend check could not run because `node_modules`/`tsc` was absent. Restored locked npm dependencies; baseline TypeScript and218/218tests PASS.
- After removal: TypeScript,220/220frontend tests and production frontend build PASS. Existing Vite large-chunk warning remains.
- The npm dependencies installed for this check (127MiB) and generated frontend output (3.4MiB) were removed again after confirming no open files/dev server. They were absent/generated inputs for this task, not application data. A future local build starts with `npm ci`; no native build cache was recreated.
- Mac CI YAML parsed with Ruby; Rust/test/helper/DMG gates retained. `git diff --check` PASS.
- Installed Rust app: bundleID`com.broomsweepy.desktop`, version1.8.1, main SHA256`cea692c9b44b10af5bc2bf702bb3ebfae8004c70425c93cbdfa177b230c0f257` unchanged; strict/deep signature PASS. No user DB swap or app reinstallation.
- Cargo manifests/locks, Rust implementation and npm manifest/lock unchanged. Native rebuild/new remote CI at the local-cleanup snapshot: NOT RUN; avoided recreating the4.8GiB native cache. The subsequent push triggers fresh remote CI; published1.8.1 CI does not validate this later workflow edit.
- Automatic code-map regeneration: NOT RUN; no generator was available. Generated historical snapshots were not manually rewritten.
- Existing dependency audit: Vite development dependency has one High finding; production-only audit reports zero. No dependency upgrade/security fix was attempted under cleanup scope. This is not runtime exploit verification.

## Recovery

Named app/profile and Swift source/project items are recoverable from macOS Trash; tracked sources also from the preserved tag. The authorized remote update removes only the current Swift checkout and its build configuration, preserving historical recovery. Removing Swift does not mean every legacy feature was ported to Rust.

#tags: swift-retirement, hestia, rust-only, git-storage, arch:027
