# QA — Conversation-first workbench

## Scope

User-directed chat redesign, visible progress, bottom composer, settings-owned
permissions. Production component in synthetic fixture; no personal files,
conversation contents, provider requests, permission changes or real deletion.

## Executed

- `npm run check`: PASS.
- `npm run test:all`: 53/53 PASS; final event-payload guard tests 2/2 PASS.
- `cargo test -p bloomsweepy-desktop --lib assistant_provider::tests -- --test-threads=1`
  with one build job/nonincremental: 25 PASS, 2 explicitly ignored live tests.
- Experience Contract validator: PASS.
- Production frontend build: PASS; existing >500kB main chunk warning remains.
- Long production transcript: main 720px/scrollHeight720px, internal transcript
  scrollHeight4606px. Composer bottom678px in720px viewport; no horizontal overflow.
- Evidence collapsed: app-result row DOM0; expanded:24 rows, composer remains visible.
  Partial-result warning visible even collapsed. Memory review auto-expanded;
  mock executions0 without clicking final review/execute.
- 760×600: composer490–558px, transcript227px, no main overflow. 390×700:
  composer bottom637px, no horizontal overflow; readable Korean15px message font.
- Synthetic slow request: actual mock bridge stage `앱 결과 분석 중`, elapsed13s,
  stop → cancellation error → enabled input/send state, no working indicator.
- Synthetic provider error: visible dock error, send/input recover.
- Connections dialog: Escape closes, focus returns to target-adjacent button;
  no toggles changed. Settings/native checks recorded below when complete.
- With stable source/no HMR reload, next-question draft survived slow reply;
  previous transcript position stayed off-bottom and latest-message button stayed
  visible. Settings contains the shared inspection/search/scan/cleanup permissions.
- Korean font: computed Pretendard Variable; `document.fonts.check` PASS.

## Native installation

ARM64 app-only bundle and ad-hoc deep/strict signature verification PASS. Version
remains1.7.0 development build. Previous installed app preserved at
`/private/tmp/broomsweepy-chat-install-backup-eNk2bn/BroomSweepy.app`.
First replacement restored7 saved messages, kept native title/version/traffic lights,
displayed docked input and Codex ready state; target popup and Settings shared
permission controls verified without toggling or transmitting a question.

WKWebView returned keyboard focus to a container after Escape in the first native
check. Added explicit opener-focus restoration; final installation verified
Escape → connection button30, and click → enabled composer52. Codex ready and
native title/version/chrome remain visible. Installed signed host SHA-256 matches
the final bundle: `0028aaf951859ab4e72595feede536b757870efd13638b2dd9a71b5dbb3cc874`.
Permissions scoped to one execution reset on restart by the existing policy;
no defaults or persistent preferences were expanded.

## Remaining / boundaries

Final native UI/build/signature checks PASS. Real provider/private
conversation transmission, Windows runtime, OS reduced-transparency/motion
preference changes and long-duration memory soak: NOT RUN in this scope.
This local development UI is not a new public release or notarized build.

Own Vite server stopped, synthetic tab closed and temporary viewport reset.
Native app is left open on the chat with an empty, focused composer. Private native
captures and synthetic QA pixels are local ignored files, not pushed assets.

## Follow-up: Settings width / non-overlay title

- `App.css:5405`: connection permissions span both settings columns; settings
  occupy the available main-pane width instead of retaining the 1100px cap.
- `AppShell.tsx:368`: Settings uses a non-sticky utility title. Keyboard scrolling
  moves the title out of the pane, not over the selected control.
- Actual production Settings component, synthetic IPC adapter: at 1280px the
  connection panel is 986px wide (old first-column allocation about485px).
  At1440px: 1146px; at760px: 634px/one column; at390px: 342px/one column.
  No main/panel horizontal overflow. One h1, Pretendard loaded, scan/search/review
  controls44px high. Keyboard focus ring measured as the existing two-shadow ring;
  focused select top552px while the scrolled title bottom is-417px.
- Frontend53/53 tests PASS and production TypeScript/Vite build PASS. Existing
  approximately879kB JS chunk warning remains. No Rust source changed in this
  follow-up; Rust unit tests NOT RUN again because this delta is CSS/header only.
- Fixture intentionally rejects autostart queries; its startup error is synthetic,
  not an installed OS registration failure. No permission, startup or Docker
  switches changed; no real AI request or file action was executed.

### Final Settings installation

ARM64 release build PASS (2m30s); app-only bundling and ad-hoc deep/strict signature
verification PASS. Replaced `/Applications/BroomSweepy.app` and opened its Settings
page. Full-width connection permissions and a scrolled page without the old
overlay title verified in actual WKWebView. Automatic startup and menu-bar memory
display remain on; Docker and all transient inspection permissions remain off.
The app is left open at the top of Settings. Own fixture tab closed, viewport
override reset and Vite server stopped.

Installed host SHA-256:
`c7aafd133284d2ff1a937800b3b2eec08aa56275ce5dedb4f7a5e66f98223e86`.
Rollback copy: `/private/tmp/broomsweepy-settings-install-backup-vYNU0r/BroomSweepy.app`.
Proof: local ignored `docs/ui-audit/screenshots/2026-10-05-settings-wide-native.png`.
Version remains1.7.0 local development; no public release, commit or push.

Scoped visual audit (not an app-wide release score): dark appearance8/10,
responsive9/10, keyboard/semantics8/10, loading/performance8/10, form layout8/10,
navigation9/10, typography/spacing8/10, motion9/10, unnecessary decoration9/10.
Weighted8.40/10 (B). Existing main-chunk size warning remains. Full contrast
measurement, light theme (app is dark-only), OS accessibility setting mutation,
Windows runtime and provider execution are outside this repair.

Web Interface Guidelines scoped findings: App.css:5405 first-column connection
allocation fixed; AppShell.tsx Settings title overlay fixed. No new controls,
uncaptioned media, focus overrides, effects or permission mutations introduced.
