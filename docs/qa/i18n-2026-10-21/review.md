# English-language audit and correction

## Request and boundary

On2026-10-21 the user reported Korean remaining after selecting English and requested a full language audit, then explicitly approved fixing confirmed omissions. Source audit covered production TS/TSX views/components, catalogs, native language propagation and Rust producers of user-facing diagnostics. Test/fixture text, Korean formatter/parser branches, language autonyms and user-owned names/content were classified separately.

The English catalog's1,597values contain no Hangul and production fixed UI strings have no missing translation keys in the audited set. This does **not** establish complete localization: several consumers rendered backend messages verbatim, and completed/pending operations stored already-translated strings in state.

## Actual installed-app baseline

The existing1.8.0 app was already set to English. Native navigation examined Dashboard, Performance, App management, Mac preferences, Space cleanup, File search, AI assistant and Settings, plus all six Settings categories. No scan, cleanup, setting change, chat submission or permission expansion was used to gather these observations.

- Mac preferences: its warning remained Korean; [actual before screenshot](01-before-english-mac-warning.png).
- App management: protected/system/helper-service reasons remained Korean in ordinary list rows.
- Other visited initial screens and all six Settings categories had no confirmed fixed Korean UI text in their exposed initial states.
- Performance contains macOS-localized process names, which are not app translation failures.
- Existing Korean user/AI conversations remain Korean by design. Neither history nor file names/content should be rewritten when display language changes.
- Error, review and result-only branches were inspected statically and through existing component/pure tests, not triggered destructively on user data.

## Corrections

| Boundary | Correction |
|---|---|
| Known catalog notices from a previous language | `LanguageProvider.appText` resolves exact catalog values to their canonical key and renders the selected language |
| Legacy backend text | Pure four-language allowlist:222exact messages,55anchored templates;16KiB input bound, unknown text preserved |
| OS preference warnings/errors | Mac and Windows consumers render app-owned diagnostics through `appText` |
| Applications | Protected reasons, inventory issues, related-data evidence, review warnings, item outcomes; local and parent status use key+params |
| Cleanup/search | Candidate/registry evidence, index issues and app-owned inspection errors; paths/snippets unchanged |
| Chat tools/settings | App-owned result fields, provider/MCP/Docker details, review warnings and current error UI; user/LLM conversation content unchanged |
| Completion/announcement state | Dashboard startup issues, scan announcements, file inspection, app management, performance and file-change review keep canonical keys/parameters |
| Native menu bar | Existing language propagation retained; new bubble/gauges do not hardcode production Korean labels |

No query/listener/polling restart is added just to change language. No automatic translation service, file upload or AI request is introduced. Safety/confirmation/permission policies are unchanged.

## Verification at this snapshot

- Existing full frontend suite:212/212PASS after updating test-only language mocks for the new `appText` context contract. Initial failures were missing mock methods, not masked with production fallbacks.
- Final expanded localization/App/catalog subset:23/23PASS, including17legacy-message tests,4actual-App state/closure tests and2catalog parity tests.
- TypeScript check and production frontend build:PASS. The existing large-chunk warning remains; it is not a build failure.
- Native source component tests:12/12PASS; native release Clippy/-Dwarnings PASS.
- Final full frontend suite after all changes:216/216PASS (serial execution to bound memory). This includes3actual `LanguageProvider` integration tests through all four languages, exact earlier-locale notice lookup, preserved user text and document/format/native language propagation. Final combined native install/runtime recheck remains pending; append its real result below.

## Limits

Final combined production build completed in4m50s. The staged ARM64 app passed deep/strict ad-hoc signature verification; signed main hash`602ff4871e4b7b85b222b2fb12f54689a0385becb6f2a10190b1049053231563`. Both verified helpers are unchanged. Native Quit was blocked because the Mac is locked, so the installed app and all databases remain untouched. The user was asked to unlock; replacement and actual English-screen recheck are not yet reported as completed.

This is a full source/navigation audit of the selected product, not a claim to have physically executed every error branch or Windows runtime. Unknown internal/OS/CLI diagnostics are retained verbatim rather than hidden behind a generic success/error. Legacy app-generated fallback text sharing a field with document snippets or model responses is not blindly transformed; typed provenance/code+params is the follow-up boundary. All four locales have translation coverage tests, but Japanese/Chinese layout and native Windows execution were not physically certified on this Mac.
