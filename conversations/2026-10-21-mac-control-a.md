# Mac control A selection

2026-10-21 · source: codex · focused decision record, not a copy of the personal daily conversation.

## User request

The user described the current settings page as text-heavy, unlike a Mac app, with hidden actions and buttons that merely navigate to other menus. Three browser-rendered alternatives were shown; A · 시스템 설정형 was recommended for five directly editable Finder settings. The current user replied: “추천대로가자”. Exact original turn time and session UUID were not supplied.

## Decision

Implement A in the existing Mac settings view: two grouped surfaces, adjacent draft Off/On and Apply, visible change history/restore, one main title, no duplicate menu shortcuts. Keep native current-value checks, null semantics, failure locks and OS permission boundary. Verify native writes only through isolated mocks for this design task; installed Mac UI is read-only.

Alternatives B/C and original rendered evidence remain in [direction comparison](../docs/design-refs/2026-10-21-direction-mac-control.md). Completion and test outcomes are recorded separately, not implied by this selection.

#tags: mac-control-a, grouped-settings, direct-actions, user-selection, arch:019
