# Implementation log — Mac control A

2026-10-21 · source: codex · selected A from “추천대로가자”.

## Contract / ownership

Experience Contract validation PASS before production edits. Existing Mac5 native setting IDs, nullable expected value, durable history and scope are unchanged. Root owns AppShell/App.css/App.tsx/i18n/docs; UI worker owns MacManagementView.tsx/css; test worker owns mac-settings-fixture.tsx/test-mac-settings-ui.mjs. No new dependency. Existing dirty work is preserved.

## Implementation

- Shell retains the sole large h1 and removes the repeated Mac eyebrow/description. Mac-only header is nonsticky and has no background wash.
- Mac view uses selected A's two grouped surfaces, visible draft choices/Apply and direct history access. Supporting explanations only are collapsible; native warnings are now visible.
- Duplicate performance/apps/cleanup links are removed from this screen, not from global navigation.
- Mock fixture only for writes/restores/failures. Installed app verification is read-only; no actual Finder preference changes authorized.

## Verification

Typecheck exit0, frontend184/184 PASS (new real-View/fake-hook Mac14 included), Cua component+real-App composition desktop/narrow/state/error/recovery/history/keyboard checks PASS. Evidence and exact limits: [QA](../qa/mac-design-review-2026-10-21/review.md).

One semantic correction: unsupported state heading h3→h2 with matching CSS to avoid skipping the Shell h1. Initial build had already read the previous source into dist; the owned Cargo child was intentionally interrupted (not a product build failure), and a fresh complete build was started after the correction. The final build assets contain h2/matching CSS and were verified before installation. Final ARM64 app build exit0, signing/deep-strict/cmp/data-preservation checks PASS. Actual installed native-glass Mac page and history navigation verified read-only; [installed evidence](../qa/mac-design-review-2026-10-21/20-a-installed-native.png). No OS write/restore. Version remains1.7.0 local development build, no commit/push/release.

## Limits

No Windows runtime/design change, new OS feature, security permission expansion, full screen-reader audit, or actual Finder write/restore test. Prototype A/B/C remain comparison artifacts, not installation evidence.

#tags: mac-design, selected-a, implementation, verification-boundary
