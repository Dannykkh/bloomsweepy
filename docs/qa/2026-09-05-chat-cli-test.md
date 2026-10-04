# Installed macOS chat / CLI integration test

Date: 2026-09-05, approximately 14:10–14:20 KST. Installed BroomSweepy 1.5.0, ARM64 Tauri app.

Follow-up implementation and later runtime findings: [CLI health / authentication fix](2026-09-05-cli-health-fix.md). This document preserves the original pre-fix test results.

Scope: user clarified that “CLI 기능” meant the in-app AI chat. Applied the API-tester workflow to UI → Rust → provider CLI → persisted conversation. HTTP CORS, JWT, uploads and external MCP registration are not applicable to this transport. No application code, CLI installation, authentication settings or external MCP settings were changed. No user files were moved or deleted.

## Runtime results

| Test | Result | Evidence |
|---|---|---|
| New folder conversation | PASS | Selected the agent-created fixture folder; runtime summary was 8,351 bytes and 3 files |
| Codex real response | PASS before restart | Returned 8,351 bytes, 3 files and largest direct item `.DS_Store`, matching local file sizes 8,196 + 85 + 70 |
| Follow-up context | PASS | A subsequent question without repeating the marker returned `파란별-482` |
| In-flight UI guard | PASS | Input, provider selector and new-conversation controls disabled while waiting |
| Cancellation | PASS | Cancel returned `Codex 응답을 취소했습니다`; input unlocked; no matching assistant-workspace Codex process remained |
| Reopen history | PASS | Leaving/re-entering the view restored five stored messages |
| Full application restart | PARTIAL | App process exited and relaunched; five messages and the folder summary were restored, but provider readiness changed |
| Codex discovery after GUI relaunch | FAIL | UI switched from Codex authenticated to Codex login required; Claude became the available default |
| Claude actual request | FAIL | UI reported incompatible CLI options; the same invocation fails with `unknown option '--safe-mode'` |

## Findings

### High: provider launch failures are mislabeled as missing login

`apps/desktop/src-tauri/src/assistant_provider.rs:833` maps every failed status probe to `Required`, including missing executables, spawn failures, timeouts and incompatible versions. It does not distinguish authentication from CLI health.

The shell-resolved `/Applications/ChatGPT.app/Contents/Resources/codex` is ARM64 Codex 0.153.3 and its login status succeeds. The alternate `/Users/dannysmacair/.npm-global/bin/codex` uses x86_64 `/usr/local/bin/node`; both version and login-status fail because its expected `codex-darwin-x64/.../codex` binary is absent. The resolver uses PATH followed by standard macOS directories and accepts the first existing file (`external_program.rs:45`). These observations support a launch-environment-dependent executable-selection problem; the GUI child executable path was not independently captured. No login/logout or npm repair was attempted.

Required follow-up: expose/verify the selected CLI path, distinguish unavailable/broken CLI from unauthenticated status, and make selection stable across launch methods. Do not repair global installations without user approval.

### High: Claude adapter passes an unsupported option

`apps/desktop/src-tauri/src/assistant_provider.rs:518` always supplies `--safe-mode`. Both inspected installations reject it before processing a prompt: `/usr/local/bin/claude` 2.1.19 and `/Users/dannysmacair/.local/bin/claude` 2.1.147. Authentication availability alone therefore does not prove chat readiness.

Required follow-up: use supported, version-checked arguments while preserving the intended no-tools/no-MCP/config-isolation boundaries; add a launch-compatibility regression test. This test request did not authorize implementation, so no flag or safety boundary was changed.

## Automated checks

- `cargo test -p bloomsweepy-desktop assistant_`: 15 passed.
- `node --test --experimental-strip-types tests/assistantText.test.ts tests/dockerIntent.test.ts`: 4 passed.
- These tests cover existing logic, not the installed CLI compatibility failures above.

## Limits and handoff state

- No Windows, Grok, Antigravity or Ollama live chat test. The latter three were shown as not installed.
- Response rendering is completion-based, not token streaming; this is current behavior, not a newly introduced failure.
- An independent read-only SQLite probe returned open error 14; persistence was verified through actual view reload and full app restart instead.
- App is left in the test conversation with the Claude compatibility error visible. The test conversation contains only synthetic questions and a fixture-only folder summary and was retained for inspection.
- No fix or rebuild was performed in this test turn.
