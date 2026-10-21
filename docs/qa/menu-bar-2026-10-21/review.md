# macOS menu bar: dismissal and glass

Date: 2026-10-21 (KST). Host: macOS26.4, 8GiB Apple Silicon. Scope: the existing status-summary surface, its native host/material and dismissal, not new functionality/layout/WebView. Requests: dismiss on outside interaction; restore consistency with the main window's glass appearance.

## v1.8.1 installed follow-up

The approved patch is now [published](https://github.com/Dannykkh/bloomsweepy/releases/tag/v1.8.1) from exact-source Mac/Windows CI. The Mac was normally quit and replaced with the verified CI DMG app; all3closed conversation/permission/developer-tool DBs stayed identical, and the previous app remains recoverable. [Actual installed bubble](09-installed-1.8.1-bubble.png) shows the14pt beak, blue/cyan memory, violet/blue CPU and teal disk bars, English controls and v1.8.1. Native AX exposes live0–100progress values. Settings-open, reopen, Esc-close and Open-main return PASS; English/memory-title preference remains unchanged.

This is an actual installed-app capture, not the earlier fixed-value preview. Its gray/white window-only backdrop is still not proof of actual desktop-composited glass. Direct status-icon/reclick and physical outside-click acceptance on this final revision remain unverified; earlier Clear/intermediate results are not copied forward. [Exact-source package and installation evidence](../release-1.8.1-verification.md). The timestamped trial history below is retained, including superseded pending/failure states.

## First panel acceptance — rejected, rolled back

The user answered the replacement-panel check: **“바깥클릭 안닫힘 여전히 회색판”**. Therefore the UnderWindowBackground/local-only NSPanel does **not** pass either requested acceptance criterion, regardless of source/Clippy/Esc/Open tests. It was normally quit and recoverably moved to `Rejected-Panel-BroomSweepy.app`. The known-outside-dismissal intermediate NSPopover was restored to /Applications; signatures and all3closed databases were verified unchanged across rollback. The acceptance table below records partial checks of the rejected panel, not completion.

Next bounded candidate: macOS26+ native NSGlassEffectView.Clear with plain NSView content (no nested material), pre26 fallback only, plus visible-lifetime global mouse-down close-only to cover an already inactive app. Global payloads/positions/keys/app identities are never read or stored, no keyboard mask or OS permission change. Local mouse monitoring and own-app resignation observer remain; generation-gated callbacks cannot close a later opening. Monitor registration errors close the panel and are returned to the existing Settings UI, not falsely reported as a successful open. Candidate build/real-screen acceptance is not implied by this description.

## Clear revision — dismissal passed, readability rejected

Production4m28s, signed ARM64 main `aef61c536c62df3f867f115f737b49bac55e29bc696ca152f90b57e82ca2c953`. Signatures passed and3closed databases were identical across the swap. Settings-open completed with both monitor tokens registered and no new OS prompt/permission change. Native AX exposed all metrics/controls. Clicking a blank title-bar area of another app closed the panel; native AX then selected the original Settings main window. The external app's profile data was not selected, edited or saved in QA artifacts. This is an observed coordinate-click route, not every possible physical/system-menu route.

Esc, reopening, Open-main, explicit Quit/host exit and relaunch with the existing memory preference On also passed. The [saved Clear capture](05-clear-glass-panel.png) was reopened/inspected: it contains bars/checkbox but does not reproduce a readable foreground or desktop-composited material, so it is not accepted as visual evidence of legibility. The user then explicitly answered **“너무 투명해서 글자가 잘 안 보임”**. Thus Clear fails visual acceptance even though the new dismissal mechanism works.

The next revision changes **only Clear→Regular**. Apple [Materials](https://developer.apple.com/design/human-interface-guidelines/materials) recommends Regular for text-heavy popovers and distinguishes it from the highly translucent media-oriented Clear variant. No fixed tint, new blur, whole-window alpha, control opacity or dismissal change. Regular source Clippy passes; production/real-screen checks are pending at this snapshot.

## Accepted visual revision — colored bubble (installation pending)

The latest [user screenshot](07-user-flat-gray.png) rejects the plain Regular panel's lost beak, gray bars and bland background. A small real AppKit preview loads the actual new `surface.rs`/`gauge.rs`/`status_panel.rs` components, rather than a browser imitation. It explicitly labels its fixed readings as preview data. [Saved native preview](08-native-bubble-preview.png) shows the rounded14pt beak and persistent blue/cyan, violet/blue and teal gauges. App-scoped captures still cannot prove desktop compositing; the user saw the actual preview and answered **“이 정도면 괜찮아”** to background transparency/readability.

One background-only Regular material uses alpha0.82; foreground controls retain alpha1. Reduce Transparency uses alpha1, checked on each opening, not a new live observer. One rounded bubble mask clips the root; half of its0.75pt edge stroke is intentionally clipped into a thin rim. No nested blur/window alpha/OS preference change. Actual-source geometry/generation/gauge/accessibility-policy tests:12PASS. Release-library Clippy/-Dwarnings:PASS. Production install and final native lifecycle checks are pending; old Regular results below are not copied into this revision's acceptance.

The combined bubble+i18n production build subsequently completed in4m50s, with216frontend tests passing. The staged signed ARM64 app passed deep/strict verification (main`602ff4871e4b7b85b222b2fb12f54689a0385becb6f2a10190b1049053231563`, original helpers unchanged). Native normal Quit was blocked by the locked Mac; no forced termination or swap was used. Current installed app remains the older Regular revision until unlock and DB-preserving replacement.

## Previous installed revision — Regular

Production build finished in7m29s (its initial build-lock wait serialized behind the Clippy run; no two compilations ran concurrently). The app was normally quit, staged from the original verified1.8.0 bundle and ad-hoc signed with the existing entitlements. Installed ARM64 main `79cddfe8ec4318310178cdf1a91e6bf55b4c167e04264a7790c3c0a23be6ba6b` matches the staged signed artifact. Deep/strict signature validation passed; helper hashes stayed unchanged.3closed database files were identical across the swap. The old original, known-dismiss NSPopover, rejected UWB and rejected Clear bundles remain recoverable in the task's private temporary directory (about142MiB before the final swap).

Settings-open returned successfully with all native metrics/controls and the unchanged preference On. [Saved Regular capture](06-regular-glass-panel.png) was reopened/inspected: unlike Clear's window-only capture, text and buttons are visible again. It is still not proof of final desktop-composited glass or contrast on all backgrounds. A repeated title-bar coordinate-click route did **not** establish dismissal this time: AX still selected the status panel. App-scoped automation is inconsistent here, so it is recorded as inconclusive, not a Regular outside-click pass or proof that the unchanged event code regressed. A final user check asks both actual readability and physical outside-click behavior; its result remains pending at this snapshot.

Regular Clippy/-D warnings, formatting and diff checks pass. The eight source-module geometry/generation tests already passed for the unchanged event/position implementation; a material-enum-only change does not substitute for native visual acceptance. Complete keyboard/VoiceOver/contrast certification, pre26 runtime, other Spaces/displays and Windows runtime remain NOT RUN. No new files were scanned/deleted and no OS/accessibility/checkbox preference was changed.

## Current-run evidence

1. [Before](01-before-popover.png): the actual installed 1.8.0 popover. Native labels and aggregate CPU/RAM/disk values are present, but its light/gray surface and heavy heading/numbers differ from the dark main window. The saved capture was opened and inspected; it contains no file paths. While the main window was visible, app-scoped captures selected that window rather than the popover; hiding it exposed the actual popover. An external Chrome control interaction still left the old popover exposed. App-scoped AX actions are not proof of every physical click route.

2. [Theme-only intermediate](02-after-popover.png): actual local intermediate app with activation/key-window, DarkAqua and lighter typography. Its AX tree selected the popover directly after Settings → Open panel. Saved window-only captures looked gray and external Chrome AX interaction did not establish physical outside-click dismissal. Do not call that automated route a pass.

3. [User's actual desktop](03-user-dark-popover.png): user explicitly confirmed “바깥 클릭 닫힘. 그런데 회색진한 판임.” and supplied this desktop-composited screenshot. The provided image was saved unmodified and opened/inspected. It confirms manual dismissal for the intermediate NSPopover but rejects its heavy gray plate. It does not validate the replacement panel.

## Material decision and source correction

- The existing Transient setting did not establish complete dismissal. Activation/key-window and an own-application resignation observer were investigated. The next44% canvas tint+observer production build finished in7m46s, but was never installed: extra darkening contradicts the real user feedback. [021](../../../memory/architecture/021-native-popover.md) preserves that rejected proposal rather than presenting it as the current source.
- The replacement uses one reusable clear/nonopaque NSPanel with one UnderWindowBackground/BehindWindow/Active NSVisualEffectView,16pt rounded clipping and no tint, whole-window alpha or second blur. Existing320×370 geometry, controls, three metrics and sampling order remain. DarkAqua matches the current app theme; heading16pt medium, metrics15pt regular and secondary labels14pt avoid extra weight. No added WebView, AI call, file/process scan or font download.
- Only this panel becomes key, not main. It is positioned8pt below the status-item anchor and clamped to the visible screen. Esc closes it. A local mouse-down monitor exists only while shown, excludes this panel and its status-item window, and always returns the original event. Other-app switching uses own NSApplication did-resign-active; OS-refused activation and nonactivating external menus are not universally covered. Close takes the token and releases its RefCell borrow before removeMonitor. Teardown removes observer → closes panel → removes status item. The sampler and Windows source remain unchanged.
- Native material accessibility fallbacks remain owned by AppKit. A read-only NSWorkspace probe returned reduce_transparency=false and increase_contrast=false on this Mac. No system preference was changed. Enabled accessibility modes and live changes were not exercised and are not reported as verified.

## Build and verification

Source formatting, diff whitespace and release-library Clippy with custom-protocol/-D warnings pass for the new panel source. The actual status_panel.rs module was linked into a small standalone test harness (not a copied/mocked geometry implementation): all6tests pass, covering anchor centering, left/right bounds, reduced padding on a330×390 screen, negative screen origins/vertical bounds, and the physically undersized-screen limit. A screen smaller than320×370 cannot fully contain fixed controls; no unsupported resize guarantee is made. Read-only review found no blocking lifecycle/cycle/reentrancy issue; runtime acceptance remains separate.

The first production attempt compiled Rust, but the x64 Node/Tauri CLI selected x64 sidecar names despite ARM64 output; that bundle was rejected. The theme-only intermediate no-bundle build finished in5m53s. Its signed ARM64 bundle reused the verified1.8.0 helpers; installed executable SHA256 `bfe1f730f0659fa7d1aee129e9897b50fa84e8e2381e087d3a5e9cb18885c53e`, deep/strict signature PASS,3closed-app databases byte-identical across the swap. The original app is recoverably kept in the task's private temporary directory. This intermediate hash is not the replacement panel artifact.

The replacement-panel production no-bundle build completed in4m30s. A new ARM64 app was staged from the original verified1.8.0 bundle, replacing only its main executable, and ad-hoc signed with the existing entitlements. Deep/strict signature validation passed. Installed signed main SHA256: `6ce058e698bf3d848c926dafe72d261a72efb50ddd0bd76db0118f6f03950b64`, equal to the staged signed artifact. Both helper hashes remain the original verified1.8.0 values. Three closed-app databases (conversation, permission, developer-tool settings) were byte-identical across the final swap; contents were not opened. Original and intermediate apps remain recoverable in the task's private temporary directory.

## Installed-panel acceptance

| Check | Result | Evidence / boundary |
|---|---|---|
| Settings → Open panel | PASS | Native AX window `BroomSweepy Status`, all3 live metric rows/version/freshness/checkbox/open/quit exposed |
| [Saved native screenshot](04-native-glass-panel.png) | Inspected; composited glass NOT VERIFIED |640×740 pixels for320×370pt, all controls present, arrow removed, weight reduced. Window-only capture replaces desktop backdrop and appears flat gray; do not call it proof of real translucency. |
| Escape | PASS | Native window dismissed, existing Settings main window returned |
| Repeated opening | PASS, bounded |3opens before explicit quit; one panel each time, next metrics update visible. Not a long-run leak test. |
| Open BroomSweepy action | PASS | Panel closed and existing Settings screen restored, not a new workflow |
| Explicit panel Quit | PASS | Clicked native Quit; exact installed host process absent afterward, no force kill |
| Relaunch / existing preference | PASS | Native1.8.0 app relaunched, Settings and panel still show memory-title preference On, left unchanged |
| Tab navigation | NOT VERIFIED | Tab left window focus unchanged under current OS keyboard preferences; settings were not altered. No complete keyboard accessibility claim. |
| Final direct status-icon/reclick / physical outside-click | FAIL — user physical outside-click | New arrowless panel failed to dismiss; intermediate NSPopover PASS was not a valid substitute. Direct status-icon/reclick still not separately verified. |
| Actual desktop-composited glass | FAIL — user screen report | User still saw a gray plate. Isolated capture cannot override that observation. |

After the negative user check, this panel was rolled back as described above. No user files were scanned, moved or deleted; no OS/accessibility/checkbox setting was changed.

This is a local 1.8.0 fix, not replacement of published v1.8.0 installers. No commit, push, release update, scan or deletion is implied by these source edits.

## Official references

- [NSPopover transient behavior](https://developer.apple.com/documentation/appkit/nspopover/behavior-swift.enum/transient): AppKit owns automatic dismissal, with documented interaction exceptions.
- [NSVisualEffectView](https://developer.apple.com/documentation/appkit/nsvisualeffectview): AppKit automatically supplies popover visual effects.
- [NSApplication activation](https://developer.apple.com/documentation/appkit/nsapplication/activate()): available from macOS 14; activation is not guaranteed synchronously.
- The local Apple SDK NSPopover.h documents the default VibrantLight appearance. DarkAqua aligns this product's existing dark theme; revisit it when the app gains a theme selector.

## Limits

Screenshot inspection does not certify full contrast, VoiceOver or every keyboard/appearance path. Pre-macOS-14 execution, multiple physical displays/Spaces/fullscreen and Windows runtime are outside this check. Direct status-item automation is not exposed by the current app-scoped computer-use surface; do not substitute Settings-open checks or intermediate manual confirmation for final direct-icon/outside-click validation. App/window-only capture is not proof of final desktop-composited translucency.

Product-design audit used real captures first; frontend-design's existing Data Instrument/native-system-font exception preserves the layout and hierarchy rather than adding decoration. Design-system scaffolding and web-only auditor/guideline scripts: NOT RUN — no new web surface or scaffold in this native material repair. Native AX/interaction evidence is reported at its actual scope.

#tags: macos-menu-bar, popover-dismissal, native-glass, bounded-audit, arch:021, arch:022, supersedes:#native-popover
