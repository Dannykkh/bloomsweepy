# Section 01 — Finder settings

## Background / Requirements

R1–R3/R7: owner needs genuine current values and safe rollback, not guessed switches. Five IDs: show_extensions (.GlobalPreferences/AppleShowAllExtensions), show_hidden_files (com.apple.finder/AppleShowAllFiles), show_path_bar (ShowPathbar), show_status_bar (ShowStatusBar), folders_first (_FXSortFoldersFirst).

## Dependencies / Reference Libraries

Existing serde/rusqlite/tauri and native CoreFoundation; no new dependency. Follow docs/domain-dictionary.md and api-spec.md.

## Module Contract

Provides: MacSettingsState + get_mac_settings/apply_mac_setting/restore_mac_setting commands and inspection helper. Consumes: AppHandle app-data location. Owns: apps/desktop/src-tauri/src/mac_settings.rs, optional mac_settings/native.rs only. Composition Point: existing lib.rs (main registers).

## Implementation Strategy

Typed Bool catalog, original absent distinct, forced-value/type guards; expected-value apply; persistent bounded journal prepared commit before native write; stored-value readback; serialize access. Undo only current==applied. No replay pending writes, no Finder restart, no commands built from input. Never convert failure to success. Schema documented api-spec.md.

## Test Scenarios / Quality Gate

Fake store tests normal, absent restore, managed, type mismatch, invalid IDs/fields, journal unavailable, write/readback failure, external conflict, prepared restart, duplicate restore. Isolated native domain only; real Finder read-only. Full cargo tests performed by main after module registration.

## Risk & Rollback / Acceptance Criteria

Fail closed on journal failure; preserve pending evidence; no automatic external effects. R1–R3 evidence required. Remove command registrations to disable; retain original-value journal.
