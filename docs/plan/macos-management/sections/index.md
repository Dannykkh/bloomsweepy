<!-- SECTION_MANIFEST
section-01-finder-settings
section-02-developer-cleanup
section-03-app-integration
END_MANIFEST -->

# Sections

| Section | Dependencies | Wave | Ownership |
|---|---|---|---|
| section-01-finder-settings | none | 1 | mac_settings.rs, mac_settings/native.rs |
| section-02-developer-cleanup | none | 1 | mac_cleanup.rs, core cleanup.rs |
| section-03-app-integration | API contract; 01/02 runtime | 1 UI, 2 composition | MacManagementView/bridge/types/fixture; main owns shared registration |

## Harness

Existing desktop lib.rs is the native composition point; App.tsx/AppShell.tsx the existing UI shell; capabilities.rs/app_tools.rs the common AI dispatcher. Each new service is registered once, not a parallel MCP implementation.

## Shared Foundation Rationale

No new shared foundation. Layer-specific assembly in section03 is required by existing Tauri/React architecture; the two feature sections own their domain implementation and tests.

## Flow mapping / ecosystem coverage

Settings flow → 01, cleanup flow → 02, inspect/navigation flow → 03. CFPreferences and local journal covered by01; cleanup engine by02; React, Tauri and MCP by03. No uncovered external system.
