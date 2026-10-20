# Workspace navigation UI verification

Date: 2026-10-20, Codex. Scope: menu grouping, dashboard density, settings categories and explicit cleanup entry points. This is source/browser verification, not an installed-app or Windows-native certification.

## Reproduce

From `apps/desktop`: `npm run dev -- --host 127.0.0.1 --port 1431`.
Open `/workspace-navigation-fixture.html`; optional `?platform=windows` or `?drives=3`.
The fixture imports the real App and intercepts every IPC command. Unknown commands and writes fail closed; no real file inspection, native settings change, model discovery or AI transmission occurs. Values are labeled synthetic (256GB disk /24GB free; CPU16% /RAM50%). It is not a production entry point.

## Automated

- `npm run check`: PASS.
- `npm run test:all`: 154/154 PASS, including new settings composition5 and global navigation8 tests.
- `npm run build`: PASS. Existing single JS bundle remains ~1.06MB /304KB gzip and emits the >500KB chunk warning; this task adds no dependency and does not claim bundle optimization.
- Experience contract validator: PASS.
- `git diff --check`: PASS.

## IAB runtime checks

| Check | Actual observation | Result |
|---|---|---|
| Desktop first view | 1280×820, four quick actions end≈590px; main scroll/client both1046px | PASS |
| Six settings categories | General default; exactly one visible, others remain mounted | PASS |
| Value retention | Large-file threshold500MB retained after AI/monitor/developer/about transitions | PASS |
| CLI discovery stability | Model query count remains1 across category switches, rejected native commands0 | PASS |
| Neutral disconnected status | No clients → `is-waiting`, “연결 없음”, not green connected | PASS |
| Form identity | Duplicate IDs0; hidden category select boxes0 | PASS |
| Mac→cleanup | System candidates pressed=true; tree can still be selected | PASS |
| Dashboard→cleanup | System candidates pressed=true, global parent aria-current=page | PASS |
| Multiple drives | QA Archive becomes main216px ring, previous drive shrinks; narrow/desktop no main overflow | PASS |
| Narrow settings | 390×844, document390px/main380px, categories44px high | PASS |
| Intermediate settings | 768: main686/686;1024:790/790 scroll/client | PASS |
| Mobile navigation | Main inert attribute present while open; Escape restores opener focus | PASS |
| Keyboard | General Tab→Scanning; visible focus ring, not hidden content | PASS |
| Icon rail tooltip | At768px, keyboard focus shows “앱 관리” at x82 outside the nav scroller; aria-describedby links it; Escape removes it | PASS |
| Windows UI adapter | Mac settings menu0, categories6, Mac menu bar0, native command rejections0 | PASS |
| Font | Computed Pretendard Variable; Korean font check true | PASS |
| Fresh final page console | No error/warning after initial load and settings navigation | PASS |

During development fixture HMR caused duplicate createRoot warnings; the fixture now unmounts on hot dispose. Fresh final tab had no warnings. All screenshots are real frontend renders with synthetic data, not real user files or native-glass screenshots.

## Screenshots

- [Dashboard1280×820](dashboard-desktop.jpg), [390×844](dashboard-narrow.jpg), [multiple drives](dashboard-multiple-drives.jpg).
- [Settings1280×820](settings-desktop.jpg), [390×844](settings-narrow.jpg).
- [Icon rail tooltip768px](navigation-rail-tooltip.jpg).
- Candidate A/B/C renders are indexed in [direction](../../design-refs/2026-10-20-direction-workspace-navigation.md).

## NOT RUN / boundaries

- Installed Tauri rebuild/replacement and native glass, native settings writes, real scan/trash/LLM flow: not needed for this frontend navigation change and not performed.
- Windows runtime: unavailable on this Mac; only platform-specific UI composition and existing compatibility tests.
- Forced reduced-motion runtime: emulation not exposed by selected browser interface. Existing FLIP/route reduced-motion branches and new CSS fallback retained; static review only.
- Full Rust workspace: no Rust production change in this UI task; prior Mac engine checks are separately documented.
