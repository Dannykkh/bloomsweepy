# Section 02 — Developer cleanup

## Background / Requirements

R4/R7: generic cache cleanup misses old Xcode output. Extend existing scan rather than a second recursive scan.

## Dependencies / Reference Libraries

Existing bloomsweepy-core cleanup engine, std only. Follow project domain dictionary.

## Module Contract

Provides: developer_cleanup_roots(home) returning two fixed CleanupRootSpec entries. Consumes: existing CleanupScanConfig. Owns: desktop mac_cleanup.rs and core cleanup.rs. Composition Point: existing lib.rs::cleanup_scan_config (main integrates).

## Implementation Strategy

Roots ~/Library/Developer/Xcode/DerivedData (7 days), ~/Library/Developer/Xcode/iOS DeviceSupport (30 days). New DeveloperCache kind always Review with rebuild/download/use caution evidence. Root's children only. Preserve bounded counts/age/cloud/link/cancellation. No archives, source, simulators, snapshots or model directories. No actual scan or deletion of host data in development tests.

## Tests / Quality Gate

Synthetic roots verify exact paths/ages; kind confidence/evidence; recent descendants not included, links excluded, no support/user documents roots. Existing core suite regression.

## Risk & Rollback / Acceptance Criteria

Candidates do not guarantee safe deletion or physical space. Existing local review/trash verification remains sole execution route. R4 fixture evidence; remove roots/kind to disable feature after consumers updated.
