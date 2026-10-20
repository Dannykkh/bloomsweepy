# Process flows

| Flow | Type | Section | Role |
|---|---|---|---|
| settings.mmd | flowchart | 01/03 | local owner |
| cleanup.mmd | flowchart | 02/03 | local owner |
| inspect.mmd | flowchart | 03 | local owner with restricted AI caller |

Single authenticated role, no RBAC lanes. No automatic recovery/replay. No sequence diagram: native API adapter ordering is already shown explicitly. State diagram follows journal statuses in implementation; prepared and restorePrepared never imply success.

`settings.mmd` includes the Apply same-value no-op, explicit unfinished-record completion without another preference write, the post-prepared expected-value recheck, and final journal-commit failure. A successful native write alone is not reported as success. Diagram branches describe the production paths, not a claim that every path was exercised through installed-app IPC.
