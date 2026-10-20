# API contract

Local Tauri IPC, existing single local role. No HTTP server or new authentication.

| Command | Request | Result | Caller |
|---|---|---|---|
| get_mac_settings | none | MacSettingsSnapshot | local Mac view / common inspection |
| apply_mac_setting | request: {settingId:string, enabled:bool, expectedValue:bool|null} | MacSettingsSnapshot | explicit local Apply only |
| restore_mac_setting | request: {changeId:string} | MacSettingsSnapshot | explicit local Restore only |

MacSettingsSnapshot: {supported:boolean, platform:string, settings:MacSetting[], history:MacSettingChange[], warnings:string[]}.
MacSetting: {id,title,description,domain,key:string, value:boolean|null, state:string, managed:boolean, canModify:boolean}.
state: value | unset | managed | typeMismatch | error | unsupported. These are serialized strings, not inferred defaults.
MacSettingChange: {id,settingId:string, previousValue:boolean|null, appliedValue:boolean, status:string, createdAtUnixMs:number, canRestore:boolean}.
Additional backend diagnostic fields are optional; unknown mutation-request fields must be rejected.

AppToolRequest: {kind:"macos_inspect"}; capability ID macos.inspect. Read-only; external inspection permission required, UI presentation view macos_management. LLM projection contains bounded IDs/state/bool/support, never journal path or arbitrary filesystem content. ui.view accepts macos_management. No AI apply/restore command exists.

Concurrency: module serializes its own reads/writes, expectedValue comparison rejects detected stale choices. CFPreferences has no cross-process atomic CAS: a concurrent external write or change-away-and-back (ABA) cannot be ruled out. Native calls run off UI thread. Unsupported platforms return supported=false, no writes. Any failed write/verification is reported as failure, not silently made successful by refreshing.

Native scope: CFPreferences current-user/any-host explicit values; .GlobalPreferences maps to kCFPreferencesAnyApplication. Strict CFBoolean only. Host-specific overrides, forced values, wrong types and failed synchronization block changes; null means confirmed absence, not inherited default. Mutations must originate from the trusted main webview; remote origins, non-main windows and generic AI/MCP invocation routes have no write authority.

Journal policy: prepared is durably committed before applying; restorePrepared before reverting. applied/restored require stored-value verification and result commit. Ambiguous prepared/failed/restorePrepared/restoreFailed outcomes are never replayed. An explicit recovery can restore when current==applied, or complete a pending row without writing if its original value is already present. Active or unresolved restoration evidence is protected from pruning; reaching a safe history cap refuses more changes.
