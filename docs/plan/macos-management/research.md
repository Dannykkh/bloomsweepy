# Research evidence

## Existing code

Explorer verified capabilities.rs as native/MCP contract; app_tools.rs enforces disclosure/cancellation/result bounds; existing cleanup scanner has age, cloud/link, resource and partial coverage guards; existing trash actions journal exact verified candidates. Existing menu_bar NSUserDefaults is Broom-only, not a cross-app settings engine. Reuse services, not Swift UI or upstream shell scripts.

## External sources (verified 2026-10-20)

- RemoveMacAI snapshot: https://github.com/omlahore/RemoveMacAI/tree/dad085044dd353f226be574d5361cc1ca11d09f8 — 41 ordinary tweaks and 14 feature/model dependencies. Known-location storage scan is not a whole-volume benchmark. No upstream code copied.
- Apple CFPreferences: https://developer.apple.com/documentation/corefoundation/cfpreferencescopyvalue(_:_:_:_:) and https://developer.apple.com/documentation/corefoundation/cfpreferencesappvalueisforced(_:_:) — explicit stored values and managed guard must be separate.
- Apple restrictions schema: https://github.com/apple/device-management/blob/release/mdm/profiles/com.apple.applicationaccess.yaml — AI restrictions have macOS26.4 deprecation annotations. Deprecation is not proof of immediate failure, but not a stable personal-app removal contract.
- Apple Time Machine: https://support.apple.com/en-gb/102154 — automatic local-snapshot reclamation; exclude permanent snapshot removal from default cleanup.

## Conclusion

Five direct Bool preferences + durable restore is implementable without private frameworks. Add old developer roots through the current scanner, not a second recursive background scan. Stored preference verification does not prove Finder visually reloaded. No speed or physical-recovery promise is inferred from logical bytes.
