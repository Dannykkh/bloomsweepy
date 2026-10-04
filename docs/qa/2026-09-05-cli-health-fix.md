# AI chat CLI health / compatibility fix

Date: 2026-09-05. Scope: BroomSweepy integration and its installed macOS app; no external CLI installation, update, login/logout or credential changes.

Follow-up at 17:11 KST: the user separately authorized standalone Codex installation. [Codex 0.153.4 installation and successful native-app chat verification](2026-09-05-codex-cli-install.md) are documented separately; Claude reauthentication remains pending.

## Implementation

- Readiness now distinguishes missing CLI, launch failure, unverified/incompatible options, positive evidence of missing login, unknown status, Ollama service failure, absent models and ready state.
- An app-private `.app/Contents/...` executable (including a symlink into one) is not treated as a standalone CLI installation. Finding a desktop app is not sufficient evidence of a usable public CLI.
- Enumerate and deduplicate standalone candidates. Failed launch/capability checks can fall back to another installation. A signed-out CLI or an uncertain authentication result does not cause a silent account switch. The selected path and version are visible in the UI.
- Check the options required by the actual Codex/Claude adapters. Missing help entries mean support could not be verified, not proof that a specific version is too old. No unverified minimum-version claim or automatic update is made.
- Claude no longer receives unsupported `--safe-mode`. It receives no-tools, no-persistence, `dontAsk`, empty strict MCP, empty settings sources, disabled hooks/skills/Chrome, and disabled Claude.ai MCP servers. The child process auto-updater is disabled. Organization-managed CLI policy may still apply.
- Status probes retain exit status and bounded stdout/stderr in anonymous temporary files, avoiding pipe-buffer deadlocks. Raw authentication output is not returned to the UI or logged. Probes are time-bounded; chat preflight supports cancellation.
- Chat execution revalidates the same resolver used for status. Frontend errors and placeholders no longer infer missing login from every failure. English/Japanese/Chinese state labels and permission descriptions were updated.

## Automated evidence

- Frontend typecheck passed; all 35 Node tests passed.
- `cargo test --workspace --quiet`: 185 passed, one opt-in installed-CLI diagnostic test ignored by default (220 total with frontend tests).
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo fmt --all --check` and `git diff --check`: passed.
- Fixtures cover missing CLI, app-private binary exclusion, broken launcher, missing required options, healthy fallback, signed-out selection, unknown auth results, bounded stdout/stderr, timeout/cancellation, argument isolation and frontend labels.
- ARM64 Node can run tests/typechecking, but existing project Rollup dependencies are x86_64-only. Production frontend bundling uses the existing x86_64 Node installation; no dependencies were deleted or reinstalled to work around that mismatch.

## Runtime / installation

- Initial new-resolver diagnostic correctly classified the standalone npm Codex launcher as broken with authentication unknown. Its app-bundled executable was not borrowed.
- A slow Claude native installation initially timed out; this remained an unknown status, not a login request. Launch-stage fallback was subsequently added while preserving account selection on auth-stage failure.
- A warm read-only diagnostic completed in 2.50 seconds and found Claude Code 2.1.147 at the standalone native path ready, Codex broken, and the remaining three providers absent.
- First updated ARM64 app was installed with strict/deep signature verification and binary comparison. Previous app preserved at `/Users/dannysmacair/.Trash/BroomSweepy-before-cli-health-20260905-161854.app`.
- The user attempted a real chat from this installed app. Claude failed after CLI startup. Reproducing the exact isolated invocation on macOS returned exit 1 and `API Error: 401 OAuth access token has expired`. This is a server authentication rejection, not inability to execute CLI commands on macOS.
- Follow-up fix classifies bounded stdout as well as stderr (Claude reports this API error on stdout), sends a sanitized structured authentication error, and changes the current provider UI to login required. Raw account data is not returned. Ready now means executable plus saved credentials found, not a guarantee of server-side token validity.
- Two Rust regression tests verify expired-token classification/redaction and actual subprocess stdout error handling; one frontend test verifies structured error interpretation. These are included in the totals above.
- Inspected `/usr/local/bin/claude` 2.1.19 help: it lacks the `auth` command group. Added a guard and regression test to reject such versions before attempting `auth status`, since an unknown command can be treated as a prompt. The native 2.1.147 help advertises `auth` and its status result is structured JSON.
- Added `npm run test:assistant` to both macOS and Windows CI definitions. The script passed locally; remote CI was not triggered.
- Final ARM64 build installed and relaunched, signature and binary equality verified. Intermediate backups also remain recoverable at `/Users/dannysmacair/.Trash/BroomSweepy-before-cli-auth-20260905-163155.app` and `/Users/dannysmacair/.Trash/BroomSweepy-before-cli-guard-20260905-163904.app`.
- At 16:42 KST, actual native UI flow passed: after rechecking, Claude showed `CLI 준비됨` and explained that server validity is checked on request. A synthetic no-tools question received the expired-token error; the structured result changed both provider labels to `로그인 필요`, displayed `claude auth login` guidance, and disabled sending. Screenshot and accessibility tree were inspected. Existing seven messages survived restarts; the test question is retained as the eighth message.
- Some cold/loaded launches exceeded the 10-second status-probe limit; the app correctly reported unknown/check-failed and recovered on explicit recheck. This remains a startup-latency limitation, not a claim that every launch is immediately ready.
- No successful Claude response claimed while OAuth remains expired. Reauthentication is a user action; no login/logout was attempted. Standalone Codex repair also remains a separate user-authorized action.

## Source references and limits

- [OpenAI CLI installation](https://learn.chatgpt.com/docs/codex/cli): standalone CLI installation is documented separately.
- [Claude CLI reference](https://code.claude.com/docs/en/cli-reference): tools, settings sources, persistence, MCP and auth status options.
- [Claude hooks](https://code.claude.com/docs/en/hooks): hook disabling and the managed-policy exception.
- Tested option availability against installed CLI help, not an assumed latest release. No Windows runtime, Grok, Antigravity or Ollama live-chat claim.
- Existing unrelated worktree changes are preserved. No commit or push.
