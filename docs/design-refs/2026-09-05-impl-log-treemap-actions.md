# Treemap actions: implementation and verification

## Prompt and preserved scope

Implement the approved `2026-09-05-experience-treemap-actions.md` delta: reveal files, navigate folders, link map/ranking interaction, and review one regular file for journaled OS-trash movement. Preserve proportional geometry, bounded aggregation, Swift-inspired glass, scan engine and platform boundary. No new dependencies, global preferences, commit or push.

## Implementation

- Map blocks and ranking share active path and color lookup; aggregate insertion no longer shifts ranking colors. Tiny tiles retain accessible names and do not gain artificial padding area.
- File activation reveals in the OS file manager, never executes the file. Folder activation drills down. Visible 44px more buttons and right-click/Shift+F10 expose the same menu.
- Menu uses a body portal, viewport clamping, keyboard navigation, Escape/outside dismissal, cleanup and focus restoration. Confirmation reuses SafetyActionDialog in a body portal, defaults to cancel and blocks the map beneath it.
- Rust retains one bounded directory result with a generation, non-serialized scan identity and precise modification timestamp. It rejects unlisted paths, folders, replaced identities, changed metadata, links and redirected parents. Execution reuses content-fingerprinted validation, cancellation, journal and OS trash. Frontend invalidates reports and rescans the current folder after successful movement.
- Storage heading is in normal flow; drilldown no longer scrolls the map under a translucent sticky header. English, Korean, Japanese and Simplified Chinese copy is covered.

## Automated evidence

- TypeScript and all frontend Node tests: 32 pass, including locale catalog/placeholder checks.
- `cargo test --workspace`: 174 pass (17 control, 44 core, 2 responsiveness, 101 desktop, 10 MCP).
- Four new Rust tests cover cancellation, identity replacement with preserved size/mtime, changed contents, folders, missing membership, hardlinks, symlinks, redirected parent, stale generation and invalidation. Private identity fields are absent from serialization.
- Workspace Clippy with warnings denied, cargo fmt check and git diff whitespace check: pass.

## Render and interaction evidence

- Actual browser renders inspected at 1280×720 and 760×600. Compact body width=760; main scrollWidth=clientWidth=678. Menu buttons measure 44×44; compact menu stayed within the viewport. Map, ranking, menu and confirmation screenshots viewed live.
- Aggregate fixture places 141 MB “other” ahead of report.pdf; both report.pdf representations remain tone 4 with identical `oklch(0.5 0.08 155)` color and shared highlight.
- Verified arrow navigation, Escape return focus, Shift+F10, cancel-default confirmation, reverse-Tab wrap, harmless cancellation, file reveal callback, successful mock removal with focus recovery, folder navigation and breadcrumbs.
- Failure fixture retained the file, displayed the changed-file error and rescan guidance, and closed cleanly. Browser error/warning logs were empty.
- CSS highlight is 120ms with explicit reduced-motion removal. Forced OS reduced-motion and frame profiling were not run. No per-frame JS or additional IPC polling.
- ARM64 Tauri build succeeded (existing >500 KB chunk warning remains; JS 698.06 KB / gzip 203.72 KB). Installed `/Applications/BroomSweepy.app`, verified strict/deep ad-hoc signature and binary equality. Previous install is recoverable at `/Users/dannysmacair/.Trash/BroomSweepy-before-treemap-20260905-134753.app`.
- Native app scanned the generated fixture folder and revealed its 172-byte disposable file in Finder, selected at the correct path. Actual confirmation displayed exactly one file, 172 B, full path and OS-trash recovery information, with cancel focused.

## Pending authorization / limits

- No real file has been moved by the new treemap action yet. Native final confirmation is paused awaiting user approval for `/Users/dannysmacair/Documents/broomsweepy-treemap-qa.UPERVg/broomsweepy-treemap-disposable-20260905.txt`.
- The control fixture and Nested/inside.txt must remain untouched. Browser mock success is not a real OS-trash end-to-end pass.
- Windows runtime verification remains not run; platform-independent tests are not a Windows installation claim.
- Temporary browser tabs closed, viewport reset and owned Vite process stopped.

## Critique

Preserved the map-first hierarchy and existing glass. More buttons are visible without hovering; the action menu is opaque enough for legibility. Compact rows compress long names with full-path tooltips, while confirmation retains the full target. No unrelated redesign or global design-rule mutation.
