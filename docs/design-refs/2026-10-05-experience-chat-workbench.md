# Experience Contract: Conversation-first workbench

## Source Mode

product-derived. User explicitly requests a familiar ChatGPT-style conversation,
bottom composer, visible progress, and settings-owned permissions. One directed
layout is used; unrelated brand/style candidates are excluded.

## Product Facts

| Claim | Source | Captured at | Freshness/status | Allowed presentation |
|---|---|---|---|---|
| CLI returns a complete response, not a token stream | assistant_provider.rs | 2026-10-05 | verified | Real phase + elapsed time, no invented completion percentage |
| App owns queries, results and final execution approval | app_tools.rs, AssistantView.tsx | 2026-10-05 | verified | Collapsible evidence; review and failures remain visible |

## Benchmark Sources

No new external benchmark. The user selected the familiar chat interaction model;
retain the project's typography, glass material and native window chrome.

## Page Goal

Ask a follow-up without hunting for the input, and distinguish working from stalled.

## Audience and Tasks

Single-user local file management on a small, memory-constrained Mac. Read old
messages, ask questions, inspect evidence, explicitly approve sensitive actions.

## Header and Navigation

Existing sidebar; one compact chat header with session tools and one h1. Target,
provider and a connection-settings button remain available above the transcript.

## Core Message

Conversation first. App evidence supports the answer without dominating it.

## Content Integrity

| Content item | Classification | Evidence | Presentation rule |
|---|---|---|---|
| Messages | verified | Local session store | Preserve content and provider identity |
| Phase | verified | Native request boundaries | Correlate request/session; no fake streaming |
| Fixture | prototype | Synthetic production-component harness | Clearly labelled; no real provider or filesystem actions |

## Section Order

Compact header → target/provider → bounded transcript → status/error → composer.
Permissions and diagnostics live in Settings and a target-adjacent modal.

## CTA Strategy

Primary: send/stop in the bottom composer. Secondary: new session, history,
evidence disclosure. Existing final approval controls remain distinct from sending.

## Trust Strategy

Show partial-result and permission/failure signals even when evidence is collapsed.
Never change permissions merely by opening settings; never treat chat as approval.

## Asset Provenance

Existing Pretendard, Lucide and project CSS tokens only. Product Design plugin
product-design@openai-curated-remote 0.1.56 is available but not installed;
local adapter used, no plugin/configuration changes.

## Desktop Structure

1280×820 and 760×600: bounded main pane, fixed layout rows (not position overlays),
one scrollable transcript, centered 800px reading width, docked bottom composer.
Supersedes the sole-main-scroller policy of the 2026-09-06 header delta; its no-overlap
requirement still applies. Other screens keep their existing scroll ownership.

## Mobile Transformations

| Desktop element | Operation | Mobile result | Reason |
|---|---|---|---|
| Header controls | compress | Icon actions and compact history | Retain reading space |
| Target/provider | reorder | Two compact rows | Preserve readable labels |
| Evidence | collapse | Summary then explicit disclosure | Avoid giant result walls |
| Composer | retain | Full-width bottom row | Always reachable |

## States

| State | Trigger | User sees | Available action | Recovery |
|---|---|---|---|---|
| loading | Session load | Labelled loading state | Wait | Error retains data |
| empty | No messages | Brief starter guidance | Choose target, ask | Cancel keeps state |
| working | Request | Actual phase + elapsed time | Stop | Draft preserved on save failure |
| error | Query/provider failure | Visible error above input | Retry/check connection | No false success |
| success | Response | Reply plus compact evidence | Follow up/review | Read older messages freely |

## Performance Budget

No new dependencies or blur layers. Collapsed evidence is lazy rendered. One timer
only while a request is active. Event listeners disposed on completion/unmount.

## Accessibility Contract

One h1, semantic controls and labelled textarea, polite phase updates (timer not
live-announced), native modal/Escape/focus return, 44px controls, IME Enter guard,
reduced-motion loader fallback and no horizontal overflow at 390px.

## Adopt

Centered transcript, right user bubble, plain assistant response, bottom composer.

## Adapt

Use app-owned bounded evidence and safety reviews rather than chat-only execution.

## Avoid

Giant diagnostics/result cards by default, overlay headers, fake progress, private
chat screenshots in tracked docs, copying a provider's logo or changing authority.

## Prompt Contract

GOAL — Conversation-first local file management.
TASK — Read, ask, inspect evidence, approve separately.
FACTS — Local session persistence and app-owned queries; CLI is not token-streamed.
CONTENT_INTEGRITY — Preserve messages, incomplete results, failure and review state.
ASSETS — Existing fonts/icons/tokens only.
RESPONSIVE — Bounded rows, one transcript scroller, compact controls.
STATES — Loading, empty, working, error, success.
SUCCESS — Input and status stay visible; modal usable; permission values unchanged.

## Success Checks

- Long transcript and expanded evidence do not move the composer out of view.
- Readback scroll is not yanked by provider progress.
- Actual native phase events reject stale request/session identifiers.
- Narrow layout, keyboard, cancellation, error and modal focus work in rendered UI.
