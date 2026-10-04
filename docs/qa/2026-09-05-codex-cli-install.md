# Codex CLI installation and app integration

Date: 2026-09-05, 17:08–17:11 KST. Environment: macOS Apple Silicon, installed BroomSweepy 1.5.0. User explicitly authorized installing the standalone CLI after the [CLI health fixes](2026-09-05-cli-health-fix.md).

## Installation

- Used the standalone installer documented in the [official Codex CLI guide](https://learn.chatgpt.com/docs/codex/cli). Downloaded `https://chatgpt.com/codex/install.sh`, inspected the complete script, then ran it non-interactively.
- Installed `codex-cli 0.153.4`, Mach-O ARM64. The installer verifies release archive SHA-256 digests; local strict code-signature verification also passed.
- Public command: `/Users/dannysmacair/.local/bin/codex`.
- Package: `/Users/dannysmacair/.codex/packages/standalone/releases/0.153.4-aarch64-apple-darwin`; the public command links through `standalone/current`.
- Companion `codex-code-mode-host` installed in the same public bin directory. The installer added its PATH block to `/Users/dannysmacair/.zprofile`; a fresh login shell resolves `codex` to the standalone command.
- Existing broken npm launcher remains untouched. The new standalone command takes precedence. App-private Codex remains 0.153.3 and was not replaced or borrowed.
- No manual credential edits or login/logout. Saved Codex login status succeeded; account output was suppressed.
- Installer script SHA-256: `ba92dd27e5c06f0d3bbc58bfa4b9cfb6599cd2742fbb1f92a2765e6c07dedb5a`.

## Integration results

API Tester workflow applied to Tauri IPC → Rust → CLI → provider response, not an HTTP application server.

| Check | Result | Evidence |
|---|---|---|
| CORS / HTTP proxy | N/A | Native Tauri IPC; no new HTTP endpoint |
| CLI capabilities | Pass | Installed `exec --help` exposes the adapter's required isolation and response-file options |
| Saved authentication | Pass | `login status` exit 0; no raw account output retained |
| App discovery | Pass | Native UI after recheck shows `Codex · CLI 준비됨`, public executable path, version 0.153.4 |
| Actual authentication and reply | Pass | Synthetic no-tools prompt returned `연결 확인` in the installed app |
| Conversation persistence | Pass for save | Existing 8 messages retained; test question/reply raised displayed count to 10 |
| CRUD / file upload | N/A | Installation smoke test; no user file operation requested |
| Exact response latency | Not measured | Completed before follow-up UI inspection; no subsecond performance claim |

The accessibility tree and rendered screenshot were inspected. The app remains on Codex with an empty input and no pending request. Only the existing synthetic fixture summary and conversation context were used. No scan, deletion, or file modification was requested from the provider.

## Limits / remaining work

- Claude Code was not installed or reauthenticated in this action. Its previously reproduced expired OAuth token remains a separate user-login issue; its latest status recheck also encountered the known cold-start timeout.
- No Windows runtime validation. The earlier 220-test suite was not rerun because this action changed no application source.
- No commit or push. Existing unrelated worktree changes preserved.
