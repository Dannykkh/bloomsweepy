# Swift retirement and reference-app cleanup

Date: 2026-10-22. Source: Codex. Exact session ID and original user-turn timestamps were not supplied. This focused record does not read or duplicate the excluded daily conversation files.

## User request

The user asked to remove the reference RemoveMacAI app and its downloaded profile files, reduce Git disk usage, and remove the non-Rust Swift version. This means retiring the current Swift checkout, not rewriting shared history or uninstalling Apple's developer tools.

The user subsequently requested removing Swift from the Git server too. This authorizes a normal cleanup commit and push to `main`; old commits, tags and releases remain available for recovery.

## Outcome

- Named app/profile files and the legacy Swift source/project moved to macOS Trash; recoverable, no Trash emptying or OS preference changes.
- Old plans preserved in `docs/archive/swift-legacy/`. Swift CI removed; four README languages and current build diagrams updated. Rust code/assets and installed app unchanged.
- Baseline218 and final220 frontend tests, TypeScript and frontend build PASS. Native rebuild/new remote CI at the local-cleanup snapshot NOT RUN; the subsequent push triggers fresh remote CI.
- Git repack preserved all12210objects, refs, reflog and HEAD at maintenance time. `.git`255860→232228KiB. No history rewrite or pruning. The later normal cleanup commit intentionally advances `main`, under the user's separate publication request.
- [Verification](../docs/qa/2026-10-22-swift-retirement.md) · [Decision027](../memory/architecture/027-rust-only-source-tree.md)

#tags: swift-retirement, rust-only, reference-cleanup, git-repack, arch:027
