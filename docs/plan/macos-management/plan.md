# Implementation plan — macOS management

## 전략 선택 (Strategy Decision)

| Candidate | Requirements | Domain | Risk | Reversible | Cost | Total |
|---|---|---|---|---|---|---|
| A: incremental allowlist + existing cleanup | 4 | 5 | 5 | 5 | 5 | 24 |
| B: port every upstream tweak/private model operation | 5 | 3 | 1 | 2 | 1 | 12 |
| C: settings links only | 2 | 3 | 5 | 5 | 5 | 20 |

Choose A. Existing dispatcher and trash engine are evidence for reuse. B would mix incompatible OS/profile/service permissions; C cannot supply genuine restore. Borrow C's explicit unsupported/guidance behavior. No new dependency.

## Scope and sequence

1. Finder settings module owns fixed catalog, CFPreferences adapter, durable journal, read/apply/restore and fake-store tests.
2. Developer cleanup adds known roots and a review-only developer-cache kind to existing bounded cleanup. Does not run cleanup automatically.
3. Existing shell registers commands/view; common capability catalog/dispatcher publishes macos.inspect; UI bridges reflect the same DTO and never invent a value for unset/error.
4. Review, compile, full regression suite, isolated native adapter check and actual component browser tests at desktop/narrow widths.

## Authority

Only an explicit local Apply/Restore button can call the native change command. Native/external AI expose no setting mutation. Changing a checkbox stages a choice, not a write. No Finder restart, shell string, profile, launch service or model mutation. Do not use file-trash full_access for OS settings.

## Contract

See api-spec.md. Core values use camelCase JSON. Five stable setting IDs. State and value are separate; null is absent/unknown, never false. Journal status distinguishes prepared/applied/restored/failed and restorePrepared/restoreFailed; conflicts return errors without overwriting history. Storage readback is not visual Finder validation. Retry is an explicit user action and rechecks expected state. External-change protection is detected-value comparison, not a cross-process lock or ABA detection. Trusted main-window origin is checked before local mutation.

## Deferred and rollback

Other 36 ordinary settings and AI14 are future compatibility work. Revert this feature by unregistering the menu/capability/commands and cleanup roots; retain journals for any applied setting. Restore through checked records before removing the module, never overwrite later external changes.

Windows settings are a separate follow-up adapter, not Finder preferences on another platform. See [Windows research and scope](../windows-management/research-and-scope.md). Its fixed read-only catalog/settings-link proposal is not implemented or Windows-runtime verified by this Mac slice.

## Acceptance

All R1–R7 in spec.md must have executed evidence; implementation presence alone is not proof. Host Finder writes/user cleanup are NOT RUN unless separately requested. A successful synthetic and isolated-domain test is labeled as such.
