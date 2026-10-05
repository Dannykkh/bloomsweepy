# Handoff: Opt-in permission duration

## Session Metadata

2026-10-05, BroomSweepy, main. Context-continuity handoff; session UUID and exact user-turn time unavailable.

## Origin

User asked whether permissions survive quitting/relaunching, then agreed to optional remembered
permissions while keeping per-action confirmation. Source: conversations/2026-10-05-remembered-permissions.md.

## Current State Summary

Implementation and permission regressions complete; desktop218/frontend53 pass. ARM64 production
build/bundle/sign/install complete. Native fully-quit/relaunch restored Remember and then Session;
all four permissions stayed OFF during isolated policy checks and existing general preferences stayed intact. Existing chat/settings
and this task remain uncommitted/unpushed. No commit/push authorization in this turn.
Final select44px correction and warning/retry on failed revocation/DB disposal are implemented,
retested (desktop218/frontend53), rebuilt and installed. During the last build the user selected
Remember/system inspectionON/cleanup reviewON; these existing choices were preserved and restored
on the final app restart. File search/scan stayed OFF. No permission was enabled by the agent.

## Implemented Features

| Feature | State |
| --- | --- |
| Session default + explicit Remember in shared Settings/chat popup | Implemented |
| Rust-owned bounded private SQLite grant records, exact approved scopes/config | Implemented |
| Startup revalidation before control listener, durable revocation, fail-closed new grants | Tested |
| No restored plan/approval/token/operation and no MCP policy-setting capability | Tested/inspected |

## Feature/Flow/Decision Snapshot

Settings lifetime is distinct from permission grant and final approval. Rust saves Remember before
new grants become active; Session grants only live in memory. See architecture009.

## Composition Diagram

Shared Settings/popup → App callback → main-window IPC → permissions mutex → SQLite commit →
search scopes/scan plan/control status → event/UI. Startup SQLite read → bounded/schema checks →
scope/config revalidation → publish permissions → start authenticated control listener.

## Files Modified

permission_settings.rs is new; control_server.rs/lib.rs compose restore and commands. Core ScanConfig
adds Serialize/Eq only. App/bridge/types/ControlStatusPanel/CSS/four locales and isolated fixture updated.
README/DESIGN/capability contract and architecture009 document the new lifetime, not new execution power.

## Important Context

Preserve prior dirty chat/settings work and unrelated demo-assets/. CUA is mandatory for UI.
Mac disk1.6GiB: whole core suite67 pass/15 blocked at minimum2GiB indexing guard; no personal deletion.

## Validation

See docs/qa/2026-10-05-remembered-permissions.md. Desktop218 pass/3 ignored; frontend53 pass;
Production component fixture1280/760/390, save failure, popup/Escape verified.
Final native PASS: policy persistence in a new process, explicit Session reset, existing auto-start/
menu-bar memory/Docker/language preservation. App/bundle SHA256:
`6e438be95fbc4193e09f280575db1aeabacf3e22e07d2ccc290cf913de7612a0`.
Rollback: `/private/tmp/broomsweepy-permission-install-backup-qhyZno/BroomSweepy.app`.

## Immediate Next Steps

No required permission implementation remains. Native app is open on Settings with the user's
Remember/inspectionON/cleanup-reviewON selections, search/scanOFF. Screenshot is in QA record.
Fixture9 closed; own Vite90006 stopped. If requested next, commit/push changes or validate Windows.
Full indexing regression needs at least2GiB free; do not bypass disk guards or delete personal files.
Do not enable personal-data permissions through CUA without action-time confirmation. No current
provider transmission or live file deletion was performed. No commit/push without request.
At final check disk633MiB/swap1850MiB; deleted only current self-built static archive92MiB and
unused dylib0.4MiB (regenerable). Preserved app/bundle/rlib/backup and all user files/history.

## Session Memory Review

Mnemo project-storage/handoff-memory/self-improvement/session-learning/project-skill-improvement read.
Root normalized to Git project. Architecture004/008 substantive bodies exist; doctor2026-09-29 <30days,
conditional doctor SKIPPED. New009 linked in both indexes and conversation; raw backlog not distilled
(session ID unavailable). Local skill candidate deferred: no project-specific target/session observation.
No component-map.json: NOT APPLICABLE. Index re-search and validator after final native results.

#tags: permissions, remembered-consent, native-install, arch:009
