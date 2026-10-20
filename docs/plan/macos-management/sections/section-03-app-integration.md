# Section 03 — App integration

## Background / Requirements

R5–R7: one Mac management screen and read-only common AI inspection, no hidden shell/permission expansion.

## Dependencies / Reference Libraries

Existing React19, Tauri2 bridge, Lucide, DESIGN.md, api-spec.md. Use actual existing visual tokens; no new UI package.

## Module Contract

Provides: MacManagementView, bridge/types, synthetic fixture, menu/navigation, macos.inspect. Consumes: settings snapshot/commands, existing performance/apps/cleanup. Owns: new MacManagementView.tsx/css, lib/macSettings.ts/types, mac-settings fixture; main integrates App/AppShell/types/i18n and capability dispatcher. Composition Point: current App.tsx and app_tools.rs.

## Implementation Strategy

Data Instrument: current values then setting rows, staged choice + explicit Apply, history restore, fast-cleanup entry, scope explanation. unset/error/managed unsupported visibly distinct; no initial false values. Busy prevents duplicate actions; stale async results don't overwrite new view. Refresh never writes. History says stored preference verified, Finder may need relaunch. Windows unsupported, no fake Mac data. New view uses full workspace width, 390px wraps controls, 44px targets, 14px minimum, native glass tokens.

## Test Scenarios / Quality Gate

Fixture simulates success, loading, failure, managed/unset/unsupported, stale/duplicate. Browser desktop/narrow no overflow, actual font, keyboard focus and reduced motion. Common protocol roundtrip/discovery/permission tests; TypeScript/build/locales/full tests. Restore uses one explicit button, not an additional checkbox.

## Risk & Rollback / Acceptance Criteria

No mac mutation AppToolRequest; external inspection grant required. No journal path or full local paths in model result. Navigation from assistant card valid. R5/R6 rendered/integrated evidence required. Remove registrations to roll back.
