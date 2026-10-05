# Experience Contract: Chat header overlap delta

Layout policy superseded on 2026-10-05 by
[Conversation-first workbench](2026-10-05-experience-chat-workbench.md).
The no-overlap requirement remains; the chat now uses bounded, non-overlapping
header/transcript/composer rows instead of a sole main-page scroller.

## Source Mode

- Mode: product-derived; approved local layout correction.
- Evidence: `../qa/2026-09-06-codex-response-quality.md`, `DESIGN.md`, native app screenshot, existing chat-session layout.

## Product Facts

| Claim | Source | Captured at | Freshness/status | Allowed presentation |
|---|---|---|---|---|
| A sticky translucent title overlaps long messages | Native BroomSweepy 1.5.0 and AppShell/App.css | 2026-09-06 | verified | QA finding, not product copy |

## Benchmark Sources

Not applicable: preserve the existing approved product design, no external benchmark introduced.

## Page Goal

Read every part of a long conversation and reach the input without a title obscuring text. Success means the header leaves the viewport with the main content and cannot overlap a message or focused input.

## Audience and Tasks

Existing BroomSweepy users reading storage advice and asking follow-up questions. Start in a saved conversation, scroll through messages, focus the composer. Preserve session selection and data.

## Header and Navigation

Keep the existing title, description, sidebar, compact rail, and mobile navigation. Only the assistant page header changes from sticky to normal document flow; other pages retain their existing behavior.

## Core Message

The conversation remains the primary reading surface. Removing the overlay makes its existing contents legible without adding explanatory copy or UI.

## Content Integrity

| Content item | Classification | Evidence | Presentation rule |
|---|---|---|---|
| Native messages | verified | Existing app-owned session | Preserve exactly |
| Layout fixture messages | prototype | Synthetic fixture | Mark as a test page; do not bundle into production entry |

## Section Order

1. Title and description.
2. Existing session tools, scope and provider state.
3. Transcript, composer and collapsed permissions.

## CTA Strategy

- Primary: existing question send/cancel action.
- Secondary: existing navigation and session controls.
- No new action, label or state transition.

## Trust Strategy

Keep current provider status and transmission-scope text. No real request or filesystem operation in the layout fixture. Native validation preserves stored conversations.

## Asset Provenance

Not applicable: existing project fonts, icons and CSS only; no new external assets.

## Desktop Structure

1280×820 and the existing native window. Main content is the sole vertical scroll owner. Title, scope, messages and composer keep their ordering and widths. No extra sticky layer or nested scroll container.

## Mobile Transformations

| Desktop element | Operation | Mobile result | Reason |
|---|---|---|---|
| Sidebar | compress | Existing compact rail at 760px | Preserve readable transcript width |
| Navigation | replace | Existing overlay at 390px | Preserve access without shrinking text |
| Title | retain | Normal-flow wrapping title | Avoid obscuring messages in short windows |
| Messages and input | retain | Existing wrapping and vertical scroll | Preserve reading and focus order |

## States

| State | Trigger | User sees | Available action | Recovery |
|---|---|---|---|---|
| loading | Existing request | Existing status text | Cancel | Existing cancellation flow |
| empty | Empty transcript | Existing entry guidance | Choose a conversation | Existing selection flow |
| error | Existing request failure | Existing error text | Retry | Existing provider checks |
| success | Short or long reply | Unobscured text | Follow-up question | Scroll freely |

## Performance Budget

No dependencies, effects, new listeners, measurements, or runtime data changes. A page-scoped class and existing CSS flow rule only. Fixture remains outside the production Vite entry.

## Accessibility Contract

Preserve one h1, reading order, labels, native controls, focus indication and reduced-motion rules. Check keyboard access to the composer after scrolling and no horizontal overflow at 1280, 760, and 390px.

## Adopt

Existing single reading surface and 14px minimum text.

## Adapt

Use the existing storage-page normal-flow header policy for chat without renaming storage semantics or changing other views.

## Avoid

Opaque masks, z-index escalation, repeated blur, shrinking text, streaming/backend changes, plugin installation, or deleting test conversations.

## Prompt Contract

GOAL — Remove chat title overlap.
AUDIENCE — Existing users reading long replies.
TASK — Scroll and ask follow-ups.
FLOW — Title → session tools → scope → transcript → composer.
HEADER — Assistant-only normal document flow.
MESSAGE — Existing content unchanged.
FACTS — Native evidence and measured fixture geometry.
CONTENT_INTEGRITY — Preserve real messages; label fixtures.
SECTION_ORDER — Existing order unchanged.
CTA — Existing send/cancel/navigation.
TRUST — Existing provider and scope information.
ASSETS — Project assets only.
LAYOUT — One main scroller, no overlay on transcript.
RESPONSIVE — Preserve existing rail/overlay and text wrapping.
STATES — Preserve loading, empty, error and success.
PERFORMANCE — CSS-only behavior correction; no new dependencies.
ACCESSIBILITY — Visible focus, labels, reading order, reduced motion.
PRESERVE — Glass material, typography, source data, CLI and persistence.
EXCLUDE — Redesign, streaming, unrelated screens and filesystem actions.
SUCCESS — Reproduce baseline overlap; pass geometry, screenshot and installed-app checks.

## Success Checks

- Header scroll displacement matches main scroll displacement.
- No message/title overlap at top, middle or bottom.
- Short/long/loading/empty/error fixtures remain usable at wide and compact sizes.
- Keyboard focus is visible and no horizontal overflow is introduced.
- Actual saved native conversation survives app replacement.

## Delta routing / module coverage

- Interface mode: Agent Workbench. Existing DESIGN.md is the style source; no new recipe.
- Three-direction render exemption: this is a two-selector correction to an approved layout, not a new visual direction.
- Product Design gate: ABSENT (`product-design@openai-curated-remote` 0.1.53 in available list, not installed, no exposed capability). No installation requested or performed; local adapter used under current tool scope.
- Source modules fully read from `/Users/dannysmacair/.codex/.olympus/source-skills/{frontend-design,mermaid-diagrams,ui-ux-auditor,web-design-guidelines}/SKILL.md`. Diagram rendering is N/A for this single-header change; no full-product audit claimed.
- Motion contract: no new animation or scroll handler. Existing reduced-motion behavior retained.
