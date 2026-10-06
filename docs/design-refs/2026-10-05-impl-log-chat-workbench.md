# Implementation / critique — Conversation-first workbench

## Brief and direction

User chose familiar chat interaction: conversation first, bottom input, visible
progress, permissions moved out of the transcript. Preserve existing branding,
native material/chrome, app-owned investigation and separate final approval.
One directed layout; no unrelated brand candidates. Experience Contract validator PASS.

## Product Design Gate

ABSENT: configured marketplaces and `codex plugin list --json/--available --json`
checked. Exact selector product-design@openai-curated-remote, available0.1.56,
remote source Plugin_fa77aec24fc08191bc6e57f377126d76, installed matches0.
Optional installation recommendation made once; no reply/consent at time of work.
Local adapter used. No plugin, marketplace, account, hook or permission mutation.
Adapter comparison NOT RUN — no introduction/comparison requested.

## Implementation and critique loop

- Assistant-only bounded main pane; compact session/target controls, centered800px
  reading measure, right user bubble, plain assistant text, non-overlay bottom dock.
- Actual native stages, bounded request/session nonce filtering, elapsed time and
  cancellation; no fake token stream or completion percentage.
- Shared App composition point for Settings/Dialog permissions. Browser native
  dialog provides modal focus/Escape; explicit opener focus restores WKWebView
  behavior found in native QA. Values and approval authority are unchanged.
- Read evidence lazy-mounts on disclosure; partial status is visible when closed;
  review/permission/error states default open. Local final safety cards retained.
- Critique repair: visually hidden user label initially contributed to outer-page
  scroll height. Anchored it to its message; main720/scrollHeight720 verified.
- Critique repair: restore focus explicitly after native modal close. Native result
  recorded in QA after final build rather than inferred from browser behavior.
- Old sole-main-scroll contract is explicitly superseded; no sticky masking/z-index
  workaround. Other views retain old scroll behavior and native material007.

## Motion / cost / accessibility

CSS loader only, transform rotation, existing reduced-motion stop; no extra blur,
animation dependencies, frame measurements or per-frame React state. One1Hz timer
only during a request, stage alone live-announced. Elapsed time is not live text.
14px controls/15–16px messages, labelled textarea, Enter/Shift+Enter/IME guard,
44px actions. Korean Pretendard loaded in rendered UI; four locale catalogs PASS.
Actual FPS/long soak and OS accessibility preference changes NOT RUN.

## Validation and remaining limitations

Use docs/qa/2026-10-05-chat-workbench.md for measured checks, synthetic/real boundaries,
installed SHA and rollback path. Synthetic screenshot and private native screenshot
are local ignored files under docs/ui-audit/screenshots, not release marketing assets.
No external provider request using private conversation, no real delete/permission
toggle, no Windows runtime or public release/commit/push in this task.
Existing approximately879kB main JS chunk warning remains; no new dependencies.

## Follow-up: Settings width and header overlap

The installed Settings view exposed the connection panel in only the first column
of the old two-column grid. The next full-span language panel forced the second
column to remain empty. Make connection permissions full-span, let the settings
grid use available main-pane width, and leave short preference panels in two
columns on wide windows. Settings alone joins the non-sticky header contract;
other views' header behavior and the chat dock are unchanged.

Frontend-design bounded repair: existing tokens/material, no new assets, effects,
dependencies or permission authority. UI audit and Web Interface Guidelines
review are scoped to these two layout defects, not a new app-wide release audit.
Fresh rules: https://raw.githubusercontent.com/vercel-labs/web-interface-guidelines/main/command.md.

## Follow-up: One conversational trash decision

User requested fewer steps and no timeout when returning late. Native inspection
confirmed app-body review required opening a modal, two acknowledgements and a
two-minute deadline. Local component repair, not a new visual direction: preserve
approved glass, composer, evidence hierarchy; three candidate renders exempt under
Render and Critique Loop §1. Main agent owns implementation and rendered critique.
Existing Product Design absence is unchanged; local adapter, no plugin mutation.
Contract validated before source implementation. Execution must remain one-shot,
main-WebView-only, exact-target-bound and freshly revalidated. No real app removal
or personal-data provider request is part of this UX test.

Follow-up user choice: native, default-OFF “skip additional confirmation” grant
for original exact-named human removal commands. Session/Remember applies;
individual plans remain runtime-only. Rust checks opt-in again, models cannot
grant it. Advice/conditional/ambiguous/multi-plan responses and related app data,
Docker, process termination, permanent Trash emptying are excluded. Inline human
yes/no uses no provider round. Native chat plans have no clock deadline but keep
single-use consumption and fresh target/selection/inventory/kind checks.
Rendered390px critique: composer remains visible, no horizontal overflow,
affirmative text wraps. Keyboard focus ring verified; repair file/empty question
buttons from42px to44px using the existing shared question-action class.
Product has a fixed dark color scheme, no light-theme switch. Reduced-motion
rules retained; OS preference toggle NOT RUN. Detailed actual/synthetic boundaries
are recorded in docs/qa/2026-10-05-conversational-trash-consent.md.

## Follow-up: Shared CLI model selection

User chose the familiar chat-composer model-selection interaction. Add one
labelled native select below the message field and reuse it in full-span Settings;
provider-specific local preferences are shared, not copied. Existing tokens,
glass and dock stay unchanged (Agent Workbench, effect budget0). No new brand
direction, effects, dependencies or permission authority.

Rendered critique found WKWebView draws an auto-appearance select too compactly;
reuse the existing lifetime-select pattern with appearance:none,44px height and
small CSS chevron. Focus/label/name/options and390px overflow were checked.
Review also repaired busy Settings refresh lock. Installed QA exposed Codex's
actual visibility:list metadata, repaired separately from UI styling.
Scoped UI audit8.85/B and all observed versus NOT RUN boundaries are in
docs/qa/2026-10-06-cli-model-selection.md. Provider metadata is not account-access
proof. Model selection does not mutate CLI global configuration or permissions.

## Follow-up: Model-specific reasoning effort

Extend the same Picker with a second44px native select and provider/model-scoped
optional version1 preference. Only explicit Codex model catalog levels appear;
verified defaults label the null override path. Stale saved levels remain visible
with an alert/reset and block sending. Unsupported providers/default models do
not receive invented mappings. No effects/assets/dependencies or new authority.

Synthetic CUA checks cover exact request effort, model restoration, shared Settings,
default null, catalog failure/support shrink/old host/busy recovery,390px reflow,
keyboard focus and4locale labels/glyph rendering. Scoped audit8.85/B and native
versus synthetic evidence/NOT RUN limits are recorded separately in
docs/qa/2026-10-06-cli-reasoning-selection.md.
