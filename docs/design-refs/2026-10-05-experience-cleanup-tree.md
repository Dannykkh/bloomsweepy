# Experience Contract: Cleanup candidate tree

## Source Mode

Mode: product-derived. Existing approved glass/Data Instrument direction and DESIGN.md tokens are retained. This bounded new state inside the existing cleanup tab does not need three rebranding directions. Product Design gate: UNKNOWN; local plugin catalog was queried but no exact Product Design selector verified. No installation/settings mutation.

## Product Facts

Verified: directory generation and assistant revision identify app-owned snapshots; existing trash pipeline validates and journals moves. Prototype: the new tree fixture uses synthetic data and never touches OS Trash. Hypothesis: hierarchical selection reduces accidental parent deletion; usability still requires real use.

## Page Goal

Review exact cleanup candidates and deliberately select a recoverable Trash operation without selecting by size alone.

## Audience and Tasks

Single desktop owner. Inspect folders, expand descendants, select all currently known root candidates or individual entries, exclude a child, review precise targets, cancel or confirm. System roles: NOT APPLICABLE.

## Header and Navigation

Preserve sidebar and storage tabs. Cleanup page switches between folder tree and existing system candidates. Storage map and AI file list provide explicit review entry points, with no automatic selection.

## Core Message

You choose what moves. Partial selection never moves the parent itself. Trash movement is not immediate recovered disk space.

## Content Integrity

Verified runtime names, paths, snapshot time, selection count and logical bytes only. Unknown size remains unknown. Protected, unreadable, truncated and unscanned contents are not presented as safe. Prototype labels appear only in fixture, never fake production data.

## Section Order

Existing page title/tabs → mode and source actions → snapshot/scope → bounded tree → selection summary → review CTA → actual operation result.

## CTA Strategy

Primary action is review selected candidates. Final Trash action is in a separate exact-path dialog, with Cancel initially focused. File open/reveal remains separate. Select-all states its known-candidate scope.

## Trust Strategy

Server owns IDs, inheritance and frontier. TTL/revision/source/identity are revalidated. Protected descendants block whole-folder review. Final approval is main Webview only, one-shot and invalidated by changed selection. No automatic retry after ambiguous mutation failure.

## Asset Provenance

Existing Lucide icons, fonts, glass tokens, SafetyActionDialog and TrashResultPanel. No assets, animation or runtime dependencies added.

## Desktop Structure

1280×820 reference and 760×600 compact native window. Full-width tree rows with fixed checkbox/disclosure area, flexible name/path and aligned logical-size column. Indentation is bounded and long paths wrap or truncate with accessible full labels.

## Mobile Transformations

- retain: disclosure, checkbox, name and explicit protected status.
- compress: indentation and size column at compact width.
- reorder: actions/summary into wrapping rows.
- defer: unexpanded descendants until explicitly requested.

## States

Loading disables competing actions and provides cancel when scanning. Empty invites an explicit source scan. Error supports deliberate refresh. Stale/expired requires reopening current source. Partial shows omitted/unknown branches. Mixed checkbox expresses child exclusions. Review separates nested-content acknowledgement. Moving prevents duplicate confirmation. Completion success reports actual moved/failed/skipped results.

## Performance Budget

One bounded backend tree workspace, 50 entries/page, 200 visible DOM rows maximum. Expansion does not replace storage-map snapshots. No per-frame React work or new motion library. Ordinary disclosure/checkbox feedback only; signature motion NOT APPLICABLE and reduced-motion honored.

## Accessibility Contract

Native checkbox with indeterminate property and accessible row labels. Nested lists/disclosures with aria-expanded, not an incomplete ARIA tree. Keyboard-reachable controls, 44px targets, visible focus, readable contrast in existing themes. Dialog traps focus, Cancel is default, closes without mutation and restores focus. Error/result updates are announced without relying on color.

## Prompt Contract

GOAL — Accurate, inspectable cleanup selection.
TASK — Reuse snapshots, expand, include/exclude, review and confirm.
FACTS — App snapshots and operation results only.
CONTENT_INTEGRITY — No invented safe/recovered-size assertions.
ASSETS — Existing approved product UI.
RESPONSIVE — Wrap actions, compress indentation, preserve checkboxes.
STATES — Empty/loading/error/partial/stale/review/moving/result.
SUCCESS — Partial parent never moves; protected content survives; source/selection replay fails; rendered controls work.

## Success Checks

Core/frontier regression tests pass. Production map and AI entry points invoke the same native tree commands. Synthetic rendered fixture proves inherited checked/mixed states, protected/partial notices, keyboard and dialog cancellation. Native installed application and Windows are separately reported, not inferred from fixture.
