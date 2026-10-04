# Experience Contract: Treemap actions delta

## Source Mode

- Mode: product-derived; delta to the approved Rust/Swift experience and simple-storage contract.
- Preserve the current glass, proportional layout, 15-block cap, aggregation, drilldown and scan engine.
- Product Design gate: UNKNOWN (no verified exact selector in this session); local implementation, no installation. Three new directions are unnecessary for this bounded behavior correction.

## Product Facts

| Claim | Source | Status | Allowed presentation |
|---|---|---|---|
| Swift opens folders, reveals files and links hover between map and ranking | BroomSweepy/Views/StorageTreemapView.swift | verified 2026-09-05 | Interaction reference |
| Rust currently aggregates size and supports drilldown but not file actions | apps/desktop/src/components/StorageTreemapPanel.tsx | verified before implementation | Baseline only |
| OS trash is recoverable; it does not immediately free the selected bytes | Existing SafetyActionDialog and trash_actions.rs | verified | Explain before confirmation |

## Page Goal

Find the space-consuming item and reveal its location or deliberately move one file to OS trash.

## Audience and Tasks

Desktop users exploring storage visually. Folder click navigates deeper; file click reveals the item without executing it. Extra actions are discoverable in a visible menu button as well as right-click.

## Header and Navigation

Preserve the global sidebar and storage tabs. Keep the storage heading in normal document flow so translucent sticky layers cannot overlap the tabs. Do not auto-scroll after each drilldown.

## Core Message

Large rectangles mean more bytes. Folder click goes inside; file click opens its location. Trash is a separate, confirmed action.

## Content Integrity

- verified: Report bytes, paths, item counts and operation result only.
- No health score, invented recovered space, or claims of current Windows runtime verification.

## Section Order

Heading → storage tabs → map heading and action hint → breadcrumbs → summary → map/ranking → empty directories → scope/footer.

## CTA Strategy

Keep the scan CTA. Primary item action is open-folder/reveal-file. A 44px ranking-row menu provides reveal and file-only trash review. Never execute a file or trash on ordinary click.

## Trust Strategy

Server retains the current directory generation and scan-time file identity. Reject unknown, stale, changed, linked, protected and directory targets. Reuse journaled trash execution and revalidation. Confirmation shows the exact path, size and OS-trash recovery boundary; default focus is cancel.

## Asset Provenance

Existing Lucide icons, local fonts, React and CSS only. No new runtime dependency.

## Desktop Structure

1280×820 main reference; 760×600 compact native window. Preserve proportional canvas plus ranking. Shared hover/focus gives both representations an inset outline; color identity is keyed by path, including when an aggregate is sorted between real items.

## Mobile Transformations

| Element | Operation | Result | Reason |
|---|---|---|---|
| Map and ranking | reorder | Map followed by ranking | Preserve usable labels |
| Ranking action | retain | 44px menu button per row | Touch and keyboard discovery |
| Context menu | compress | Clamp to viewport | No offscreen actions |
| Header | remove | Remove sticky positioning, not content | Prevent overlap on short screens |

## States

| State | Presentation | Recovery |
|---|---|---|
| loading | Existing scan status; item actions disabled | Cancel scan |
| empty | Existing no-size result | Choose another folder |
| success | Map, ranking, reveal result | Continue exploration |
| error | Local action error with rescan guidance | Close confirmation and rescan |
| stale | Server rejects old generation or changed identity | New scan required |
| confirming | Exact single-file path and bytes; cancel focused | Cancel without mutation |
| moving | Disable repeat confirmation; cancellable backend work | Show partial/error result honestly |

## Performance Budget

At most 15 map blocks and 12 ranking rows, bounded menu, no animation library. CSS highlight transition 120ms only; reduced-motion removes it. Event listeners cleaned on menu close; no per-frame React updates.

## Accessibility Contract

Explicit accessible names even on tiny blocks. Shared keyboard focus highlight. Menu supports Enter, Escape, arrows, Home/End, ContextMenu/Shift+F10 and outside dismissal with focus restoration. Confirmation traps focus and defaults to cancel. No hover-only destructive control. Existing compact layout remains scrollable without horizontal overflow.

## Prompt Contract

GOAL — Close the Swift interaction gap without changing the storage visualization.
TASK — Reveal files, navigate folders, link hover/focus, confirm safe single-file trash.
FACTS — Use runtime report and operation result only.
CONTENT_INTEGRITY — Never execute a file, immediately delete, or claim bytes freed by trashing.
ASSETS — Existing project icons, fonts, glass surfaces.
RESPONSIVE — Preserve the map, reorder ranking and clamp action menus.
STATES — Loading, empty, error, stale, confirmation, moving and completion remain explicit.
SUCCESS — Native Mac navigation/reveal and confirmation work; disposable fixture validates trash; safety regression tests pass.

## Success Checks

- Same path highlights in both representations; aggregation cannot change its color identity.
- Folder and breadcrumb navigation work; normal file activation reveals rather than runs.
- Menu reachable without right-click, can be dismissed and restores focus.
- Scan replacement, inode replacement, symlinks, folders, outside paths and changed file metadata cannot be trashed.
- Native screenshot confirms heading and tabs do not overlap; compact browser check has no horizontal overflow.

## Module Coverage

Core source modules resolved in the preceding design work under `/Users/dannysmacair/.codex/.olympus/source-skills`: frontend-design, mermaid-diagrams, ui-ux-auditor, web-design-guidelines. Existing Data Instrument/layout and motion contracts retained. Conditional modules not requested. Full external guideline audit is not claimed; this delta checks keyboard, focus, state, responsive layout and reduced motion.
