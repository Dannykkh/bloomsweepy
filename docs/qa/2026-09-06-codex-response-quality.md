# Codex response quality and conversation restoration

Date: 2026-09-06, 08:26–08:30 KST. Environment: installed `/Applications/BroomSweepy.app` 1.5.0, macOS Apple Silicon, standalone Codex CLI 0.153.4. This follows the [installation smoke test](2026-09-05-codex-cli-install.md).

## Scope and method

API Tester workflow applied to the native UI → Tauri IPC → Rust → Codex CLI → provider response. HTTP CORS, proxy configuration, JWT issuance, CRUD endpoints, and file uploads are not applicable to this flow. Existing authenticated Codex was used without changing login, models, or settings.

Used the existing synthetic folder conversation, initially containing 10 messages. Three real questions tested multi-part analysis, a follow-up referring to that answer, and a follow-up after normal app quit/relaunch. The prompts instructed the provider not to use tools or modify files. No new user-folder scan or cleanup was requested.

## Ground truth and analysis result

Read-only filesystem checks and independent arithmetic established the expected values before the question. The provider received the app's existing folder summary, not this answer table.

| Item | Expected | Actual answer | Result |
|---|---|---|---|
| Total logical bytes | 8,351 | 8,351 | Pass |
| Files | 3 | 3 | Pass |
| Largest direct child | .DS_Store, 8,196 bytes, 98.1% | Exact match | Pass |
| Second direct child | keep-this-fixture.txt, 85 bytes, 1.0% | Exact match | Pass |
| Smallest direct child | Nested, 70 bytes, 0.8% | Exact match; explained directory aggregate | Pass |
| Remaining without largest item | 155 bytes | 155 bytes | Pass |
| Scope limitation | Cannot infer why the whole Mac is full | Explicitly said broader scan is needed and distinguished logical size from disk usage | Pass |
| Cleanup advice | Review only; no deletion/safety claims | Three review steps, with file safety and contents left unverified | Pass |
| Long response completion | Six sections, final synthetic marker | Full sixth section `은빛솔-731` visible in screenshot | Pass |

The response also explained why rounded percentages need not sum to exactly 100%. Text was readable with line breaks and without raw Markdown decoration. Native accessibility text truncates a long individual node in tool output; the rendered screenshot confirmed the answer itself was complete.

## Follow-up and restart evidence

- Follow-up asked for the marker from section 6, remaining size from section 3, and smallest item from section 2, without repeating their values. Actual reply was three lines: `은빛솔-731`, `155바이트`, `Nested`.
- After that reply, the saved conversation showed 14 messages. Confirmed no pending request and an empty draft before normal Cmd-Q and relaunch.
- On reopening AI chat, the same 14 messages and detailed answer were restored. Codex became ready without changing selection or logging in again.
- Post-relaunch question again omitted the expected values and asked not to confuse the new marker with the earlier test marker. Actual reply: `은빛솔-731, 155바이트`.
- Final conversation contains 16 messages. App remains on Codex, input empty, no pending request. No existing messages were deleted.

## Timing

These are observation bounds, not exact response durations or a performance benchmark:

- Detailed analysis was still pending at 17.6 seconds and completed by the 35.8-second inspection.
- First follow-up completed by the 25.0-second inspection.
- Post-relaunch follow-up completed by the 28.2-second inspection.

Current adapter displays a completed response after CLI exit, not token streaming. The loading indicator and cancellation control were present while the first request ran. Live cancellation was not repeated in this turn.

## Automated verification

- `npm run test:assistant`: 5 passed.
- `cargo test -p bloomsweepy-desktop assistant_provider::tests -- --nocapture`: 20 passed; 1 opt-in installed-CLI diagnostic ignored as designed.
- `cargo test -p bloomsweepy-desktop assistant_sessions::tests -- --nocapture`: 5 passed, using temporary test databases, not the installed app's database.
- Total: 30 passed, 0 failed, 1 ignored. `git diff --check` passed.
- All three fixture file SHA-256 hashes were unchanged after the live tests. This checks fixture integrity, not a general audit of every possible provider read.

| Fixture file | SHA-256 before and after |
|---|---|
| .DS_Store | e3736942d229698a4226c922d189f2517ee6f6e82dce1f671d3fa5d199f65385 |
| keep-this-fixture.txt | eca3553bffe93d41433a052f0929aadec3ed84cf03150dd6c8c0f095efa82f4e |
| Nested/inside.txt | f67ed3acfaa515c1bd22fd21ce6b58696fd2c03f87d929f188f960f98f4bd678 |

## Remaining findings and limits

1. Medium UI issue: when the main pane scrolls through a long conversation, the sticky title overlaps transcript text. Reproduced in screenshots before and after restart. The `.utility-header` sticky positioning and translucent-to-transparent background in `apps/desktop/src/App.css:440` are a likely contributor. This turn verified responses and did not change layout or rebuild the app. Follow-up: resolved and installed in the [chat-header layout correction](2026-09-06-chat-header-layout.md); this original observation is preserved as history.
2. Reply waiting UX: completed-answer delivery works, but text does not appear incrementally. Streaming would require a separate adapter/event-flow change, not just a display animation.
3. This is evidence for the tested short conversation, not unlimited memory. Frontend sends at most 20 recent messages, 2,000 characters per message, and 24,000 total characters (`apps/desktop/src/views/AssistantView.tsx:1103`).
4. No large-folder, adversarial, Windows, Docker live-chat, or Claude success claim. Claude reauthentication remains separate. No commit/push or application source changes.
