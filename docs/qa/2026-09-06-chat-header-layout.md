# Chat header layout verification

Date: 2026-09-06. Scope: assistant-page header overlap only. Existing glass styling, transcript, provider execution, and session persistence remain unchanged.

## Cause and correction

The common `.utility-header` was sticky inside the main scroll container. Its translucent background allowed messages to show beneath the title. AppShell now adds `utility-header--assistant` only to chat; that class shares the existing storage header's normal-flow rule (`position: relative; z-index: auto`). No scroll handler, extra container, dependency, or animation was added.

Design basis: [local Experience Contract](../design-refs/2026-09-06-experience-chat-header.md) and existing DESIGN.md. This is a small approved correction; new visual directions and a full-product accessibility audit are out of scope. The focused-content/overlay check follows the [Web Interface Guidelines](https://raw.githubusercontent.com/vercel-labs/web-interface-guidelines/main/command.md).

## Browser evidence

`apps/desktop/chat-layout-fixture.html` imports real AppShell and production CSS, with synthetic representative transcript markup. It is a layout fixture, not an end-to-end AssistantView/provider test. Its labels explicitly say no real AI requests or file operations occur.

- Baseline at 1280×820 and main scrollTop 820: header `sticky`, top 0, one overlapping message. Screenshot inspected.
- Same position after correction: header `relative`, top -820, bottom -718, zero overlaps. Screenshot inspected.
- Read-only DOM measurements after UI scrolling checked header displacement, message intersections, one h1, and horizontal overflow. Accessibility snapshots were refreshed after scrolling to avoid stale measurements.

| Viewport | Short | Long | Empty | Loading | Error |
|---|---|---|---|---|---|
| 1280×820 | Pass | Pass | Pass | Pass | Pass |
| 760×600 | Pass | Pass | Pass | Pass | Pass |
| 390×844 | Pass | Pass | Pass | Pass | Pass |

All 15 combinations: relative header, zero header/message overlaps, no horizontal overflow, exactly one h1. Long states were inspected after nonzero main scrolling. 390px is browser stress coverage, not a claim of native mobile support; the native minimum is 760×600.

- Screenshots inspected at 1280, 760, and 390px; long unbroken sample text wraps within the transcript.
- At 760×600 and 1280×820, keyboard Tab reaches the send button fully within the viewport; Return submits only the local fixture and announces `테스트 입력 확인 · 외부 전송 없음`.
- Control views: settings header remains `sticky`; overview/storage remains `relative`.
- Browser error log for the fixture was empty at final inspection.
- No new motion. Runtime OS reduced-motion toggling, native screen-reader narration, and Windows execution were not tested.

## Build and installed-app evidence

- `npm run check`: passed.
- `npm run test:assistant`: 5 passed, 0 failed.
- `TAURI_ENV_TARGET_TRIPLE=aarch64-apple-darwin npm run tauri -- build --target aarch64-apple-darwin --bundles app`: passed. Existing >500kB chunk-size advisory remains; it is not a build failure.
- Production `dist` contains no fixture identifier or fixture banner.
- Built binary is Mach-O arm64. Initial bundle retained a linker-only ad-hoc signature and failed strict resource verification. Applied local ad-hoc signing with `codesign --force --deep --sign -` to the generated app, then strict/deep verification passed. This is local installation signing, not Developer ID signing or notarization.
- Verified no active request and an empty draft, quit normally, then replaced `/Applications/BroomSweepy.app` using the newly built app. Previous app is recoverable at `/Users/dannysmacair/.Trash/BroomSweepy-before-chat-header-20260906-085530.app`.
- Installed app passes `codesign --verify --deep --strict`; its main binary matches the built binary with `cmp`.
- Reopened native app: existing conversation still contains 16 messages, including the prior final answer. Codex reaches `CLI 준비됨`.
- Native screenshots at the middle and bottom of the saved long conversation show the title scrolled away and no text overlap. Typed a temporary draft and tabbed to the visible send control without submitting; cleared the draft afterward. No new provider request, message deletion, or file cleanup occurred.
- Fixture viewport override reset, owned fixture tab closed, owned Vite process stopped. Native app left open on the saved conversation with an empty draft.

## Limits and remaining work

The earlier [response-quality report](2026-09-06-codex-response-quality.md) remains the evidence for actual Codex answers. Its header-overlap finding is resolved by this change. Incremental response streaming and Claude reauthentication are separate outstanding topics; neither changed here. Other dirty worktree changes were preserved. No commit or push.
