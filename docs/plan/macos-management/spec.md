# Mac management improvement

2026-10-20. Existing implementation: Rust/Tauri desktop, common capability dispatcher, bounded cleanup engine and local review/trash journal. This is an incremental improvement, not a replacement implementation.

## Context Map

Goal: understand Mac configuration and recover storage without opaque shell execution or inflated savings claims.

### Stakeholders and Role Inventory

Local Mac owner is the only authenticated human. NOT APPLICABLE: single role. AI is a caller, not a second administrator; it receives read-only inspection results under existing disclosure permission.

### Ecosystem map

| System | Integration | Section |
|---|---|---|
| CFPreferences | allowlisted Bool preferences, forced-value guard | section-01-finder-settings |
| Local SQLite | write-ahead durable settings journal | section-01-finder-settings |
| Existing cleanup engine | old Xcode DerivedData and device support children | section-02-developer-cleanup |
| Common AI/MCP dispatcher | read-only macos.inspect, ui.view | section-03-app-integration |
| React/Tauri shell | Mac management view, actual state, local apply/restore | section-03-app-integration |

## Problem Statement

| ID | Problem | Impact | Section |
|---|---|---|---|
| P1 | Mac settings are scattered and changes are hard to undo | user cannot confidently manage configuration | section-01-finder-settings, section-03-app-integration |
| P2 | Generic cleanup misses known developer build data | large repeatable storage use remains hidden | section-02-developer-cleanup |
| P3 | AI must use app facts instead of inventing CLI management | inconsistent answers/authority | section-03-app-integration |

## Requirements

- R1: list exactly five settings: show_extensions, show_hidden_files, show_path_bar, show_status_bar, folders_first. Distinguish absent, bool, managed, wrong type, error and unsupported. No automatic OS mutation at startup/read.
- R2: fixed ID + Bool + expected previous value only. Local apply/restore only. Durable prepared journal before write; verify stored preference after write. No automatic Finder restart.
- R3: restore original absence by removing the override. Refuse restore over a later external change. Serialize writes, cap journal history, never replay prepared writes after restart.
- R4: extend existing cleanup with old Xcode DerivedData (7 days) and iOS DeviceSupport (30 days). Review confidence; no simulator, archive, project, Application Support or model deletion. Existing bounded walk/cloud/link/trash safeguards remain.
- R5: Mac management view uses current app tokens, readable controls, pending choice + explicit Apply, restore history, refresh/error/busy/unsupported states. Show clear scope and connect to existing performance/apps/cleanup.
- R6: common macos.inspect query used by chat and external MCP. Existing external inspection disclosure is required. Raw paths/commands/domain/key writes are not part of the protocol. Full-access file permission does not authorize system changes.
- R7: validate using fake preference store, synthetic cleanup fixtures, isolated native test domain and UI mocks. Do not alter real Finder preferences or delete user data during tests.

## Risk Assessment

| Risk | Impact | Mitigation |
|---|---|---|
| OS preference changes/managed profile | high | absent/type/forced distinctions, supported API, allowlist, failure explicit |
| crash between write and result | high | prepared record committed first; pending state never auto-replayed |
| later user changes overwritten | high | expected-value apply and restore comparison |
| logical size mistaken for recovered space | medium | candidate bytes only; existing actual trash result not guaranteed physical savings |
| macOS26.4 AI restrictions deprecated | high | profile/model/service mutations excluded |

## Test Scenarios

Normal: read five values; explicitly apply one; verify readback; restore previous explicit value or absence. Error: forced preference, wrong type, unknown ID, journal failure, native write/readback error, external change conflict. Edge: same desired value, restart with prepared record, repeated apply/restore, Windows unsupported, cloud/link candidate exclusion, busy/stale UI response.

## Explicitly deferred

Remaining RemoveMacAI settings, Apple Intelligence/model removal, launchctl/background services, permanent Time Machine/simulator cleanup, chat-driven OS writes. These are roadmap items, not implemented capabilities.
