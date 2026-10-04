# macOS completion verification

Date: 2026-09-06. Scope: this Apple Silicon Mac, BroomSweepy 1.5.0, current dirty worktree. The user subsequently excluded Claude validation and requested Codex as the reference common chat flow. Claude authentication is no longer a completion prerequisite. The newer cloud-scan exclusion fix is tracked separately; these automated numbers describe the preceding baseline, not its final verification.

## CLI startup correction

- Found `/Users/dannysmacair/.local/share/claude/versions/2.1.147` was an **x86_64** executable. `claude --version` emitted a Bun AVX warning and took **12.64s**, exceeding BroomSweepy's per-probe 10s deadline.
- Replaced only the architecture, retaining **Claude Code 2.1.147**. Downloaded `darwin-arm64/claude` from the vendor release location identified by the [official installer](https://claude.ai/install.sh), following the [vendor installation/integrity guidance](https://code.claude.com/docs/en/setup).
- SHA-256 matched the 2.1.147 release manifest: `94a81554195edc33c2587f106bfc2e301f450f52a05cbfaed8b20f6f0882697c`. `file` reports arm64, and strict code-signature verification passed before and after installation.
- The existing public symlink is unchanged. Previous Intel binary is recoverable at `/Users/dannysmacair/.Trash/claude-2.1.147-x64-before-arm64-20260906-094804`. No login/config file was read or manually rewritten.
- First native version invocation took **6.42s**; two subsequent observations were **0.77s** and **0.06s**, without the AVX warning. These are observed samples, not a guarantee under every system load. Application timeouts were not increased.
- The opt-in installed-provider diagnostic passed in **1.13s**: Codex 0.153.4 and Claude 2.1.147 both passed version/options/local-login checks. Uninstalled providers remained correctly marked missing.
- A real native-app Claude question using the existing synthetic folder still received the structured expired-login error. The app showed `로그인 필요` and blocked further sends. Session messages increased from 16 to 17; no prior message was removed. This distinguishes startup repair from server authentication.
- Started `claude auth login --claudeai`, then cancelled it with Ctrl-C after the user excluded Claude. No credentials were requested in chat or entered by the agent. Actual successful Claude response is **OUT OF SCOPE**, not a pass. The user reports Windows validation; that is not this Mac's test evidence.

## Native core regression added

`crates/bloomsweepy-core/tests/macos_resource_stability.rs` uses only test-owned temporary data. Default CI run is 20 repetitions; `BROOMSWEEPY_MAC_SOAK_CYCLES=1000` extends it, bounded to 1–2000.

| Check | Result |
|---|---|
| 4,096 files in 32 Korean-named directories; scan, treemap, catalog build/search repeated 1,000 times | Pass, exact totals and search results each cycle |
| Open descriptors, file cycles | 4 → 4 |
| Resident memory, file cycles | 13,632 → 12,928 KiB |
| File-cycle elapsed time | 151.74s |
| 128 text documents, index/search repeated 1,000 times | Pass, 128 matches each cycle |
| Open descriptors, document cycles | 4 → 4 |
| Resident memory, document cycles | 13,264 → 7,056 KiB |
| Cancel during real traversal; subsequent complete scan | Pass |
| 8GiB sparse logical file with <1MiB allocated, symlink cycle | Pass, exact logical size and no link traversal |
| Sixteen 256KiB identical files, 20 duplicate scans | Pass, one duplicate group each time |
| Entire extended native test | Pass, 161.36s |

This is a bounded regression, **not an all-day soak**, peak-memory proof, or physical 8GiB content scan. Descriptor/RSS samples are before and after warmed cycles; thread counts were not measured. Synthetic temporary data was automatically removed by the test. No user drive was scanned or cleaned.

## Complete automated checks

- `cargo test --workspace --quiet`: **186 passed**, 1 opt-in provider diagnostic ignored by default. The opt-in diagnostic was separately executed successfully above.
- `npm run check`: pass.
- New `npm run test:all`: **35 passed**, 0 failed. macOS CI now invokes this full frontend set instead of the previous partial set.
- **221 ordinary automated tests passed** in total. Extended 1,000-cycle and installed-provider diagnostics are additional runs, not additional distinct tests in that total.
- `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`, `git diff --check`: pass.
- `npm run build`: pass; existing 701KB JS chunk advisory remains. JS/CSS output names match the previously installed header-fix build. No production UI or Rust application logic changed in this follow-up.
- Existing `/Applications/BroomSweepy.app` passes strict/deep signature verification and its binary matches the prior built ARM64 bundle. No unnecessary app replacement was performed for test/CI-only changes.

## Remaining native checks and limits

- Created a disposable Cocoa test app containing no user document to test normal termination from the Performance screen. During background UI automation, the screen stayed at its initial measurement state; the hook skips polling while `document.visibilityState` is hidden. No backend performance call was evident in a 2s host process sample. Background visibility is a suspected explanation, **not a proven product defect**.
- App-raise, Window menu, title-bar activation and Finder-open attempts did not yield a current native process table in this run. Therefore actual performance-screen termination is **UNVERIFIED**, not passed. The owned probe process was explicitly stopped afterward; that cleanup is not evidence of the app's termination feature. No user application was closed.
- Earlier installed CPU/RAM and own-memory-clean checks remain documented in [the performance log](../design-refs/2026-09-04-impl-log-rust-performance.md); those were not silently promoted into a new live pass.
- Claude testing is excluded per the latest user instruction. Preserve the existing conversation; further common-flow work uses Codex and synthetic data only.
- Remaining broad limits: physical large-volume I/O, protected-folder/TCC scenarios, network/removable failures, all-day use, other Mac hardware/OS versions, and Apple distribution notarization. Current local signing is not an external distribution certificate.
- Working free space reached roughly 1.4GiB during validation. The verified redundant 201MiB Claude download was removed; the installed native binary and recoverable old binary were preserved. Existing target directories and user files were not cleaned.
- No commit, push, CI dispatch, model change, or Windows installation claim.
