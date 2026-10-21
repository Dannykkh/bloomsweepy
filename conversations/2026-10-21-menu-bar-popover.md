# Menu bar popover correction

## User request

2026-10-21, Codex. The user reported that the Mac menu-bar summary did not disappear after clicking elsewhere. During the fix, they also asked why its appearance was so strong and where the glassmorphism had gone. This focused record does not include the excluded daily conversation files.

## Scope and evidence

Preserve the existing native popover, three aggregate metrics, 10-second sampler and main-window residency. The original Transient setting alone was insufficient evidence of dismissal. The source lacked app activation/key-window setup and app-deactivation closure. Actual screenshots showed a gray surface and heavy numbers different from the dark main window.

An intermediate local1.8.0 was installed with activation/key-window, DarkAqua and lighter typography. Its private app backup and unchanged databases were checked. The saved window-only capture still looked gray and automated external-app interaction did not establish dismissal, so that automated route was not declared a pass. A44% canvas tint and own-app resign-active observer were built next, but that artifact was never installed.

## User screen feedback

The user then explicitly confirmed “바깥 클릭 닫힘. 그런데 회색진한 판임.” and supplied the actual desktop-composited panel screenshot: [user image](../docs/qa/menu-bar-2026-10-21/03-user-dark-popover.png). Manual outside-click dismissal is therefore verified for the intermediate NSPopover, while the dark-plate visual complaint remains. That feedback rejects the extra44% tint rather than justifying further darkening.

The next source keeps the layout/controls/sampler and replaces the heavy automatic NSPopover material with one clear NSPanel containing one native UnderWindowBackground/BehindWindow/Active effect. No tint, nested blur, whole-window alpha, additional WebView, global input monitor or OS preference change. Local mouse-down monitoring exists only while shown; other-app switching uses our own application notification. Close removes its local token without retaining a RefCell borrow, and callbacks never re-borrow shared UI. New-panel production/native verification is recorded separately in QA; the intermediate manual pass is not treated as proof for the new host. [021](../memory/architecture/021-native-popover.md) is preserved as superseded by [022](../memory/architecture/022-native-status-panel.md).

Evidence: [QA](../docs/qa/menu-bar-2026-10-21/review.md), [existing experience contract](../docs/design-refs/2026-09-07-experience-macos-menu-bar.md).

## Local installation and actual checks

The replacement production build completed in4m30s. Final signed ARM64 main hash6ce058e698bf3d848c926dafe72d261a72efb50ddd0bd76db0118f6f03950b64 matched the staged artifact, signature passed and3closed-app databases were unchanged across the swap. Native Settings-open, Esc, reopening, Open-main, explicit Quit/process exit and relaunch/preference restoration were exercised successfully. All6real-module placement tests and release-library Clippy passed.

The new panel was captured and saved, but a window-only capture excludes actual desktop compositing and still appears gray; that is not declared a glass success. Tab focus did not move under the current OS keyboard preferences, so complete keyboard accessibility is not claimed. The new panel was reopened and the user was asked once to confirm the real desktop backdrop and physical outside-click behavior. That final feedback remains pending here. Public release assets were not changed, and no commit/push was performed.

## New panel rejection and recovery

The user replied “바깥클릭 안닫힘 여전히 회색판”. This is a real failure of both requested criteria, overriding the partial automated/static passes. The native app was normally quit and the known-dismiss-working intermediate NSPopover bundle restored;3closed databases stayed identical and signature verification passed. The failed panel artifact remains recoverable, not installed.

The next candidate changes only the two failed mechanisms: native NSGlassEffectView.Clear on26+ (plain content, no nested effect) and visible-only global mouse-down close-only alongside the local monitor/own-app resignation observer. Global payloads and keyboard events are not read/recorded. Per-opening generations invalidate late callbacks; failed monitor registration closes the panel and returns an open error through the existing async Settings invocation. Official Apple docs and the actual26.5 SDK/current0.3.2bindings support these APIs; real-screen success remains unverified until the new candidate is run.

## Clear legibility feedback

Clear production4m28s was installed with signature/DB preservation checks. An actual coordinate click in another app's blank title bar dismissed the panel; Esc/Open/Quit/relaunch also passed, and no new permission prompt or setting change occurred. The user then answered “너무 투명해서 글자가 잘 안 보임”. Clear is therefore not visually accepted. The next source changes only the glass style to Regular, preserving the working mouse/dismissal implementation and opaque native text/controls. Apple HIG identifies Regular for text-heavy popovers; no fixed dark wash or additional material is introduced.023 is superseded by024, preserving this failed style trial and the successful bounded dismissal check.

## Regular installed, actual acceptance pending

Regular production7m29s completed. Signed ARM64 main79cddfe8ec4318310178cdf1a91e6bf55b4c167e04264a7790c3c0a23be6ba6b matches staging, signatures/helpers/3closed databases were verified. Actual Settings-open succeeds and the saved window capture shows readable text/buttons again, unlike Clear's bare capture. Another external title-bar automation did not establish dismissal, so that route is inconclusive rather than a claimed pass. The Regular panel is left open and a final user check asks readability plus physical outside-click. Actual desktop acceptance is still pending; full native/Windows/a11y claims are not made. No commit/push/release update performed.

## Colored bubble acceptance

The user supplied the actual plain Regular screenshot and requested restoration of the pointed top, attractive gauge colors and more translucency. The implementation keeps the original body controls/sampler but adds a14pt beak, background-only Regular alpha0.82 and full-opacity sibling controls. Native read-only gauges keep their blue/cyan, violet/blue and teal colors independently of activation. A small native preview compiled the actual surface/gauge source, clearly marked its fixed values as preview data, and exposed real progress roles/values. All12source-module tests and release Clippy passed. Asked about the real preview's background/readability, the user answered “이 정도면 괜찮아”. This accepts the visual choice, not an unperformed final install/dismissal test.

During production compilation the user added an English-language audit: after switching to English some text remains Korean, then explicitly approved correcting confirmed omissions. App-owned diagnostics/warnings/evidence are in scope; user file names, existing conversations and macOS-owned UI are not to be rewritten. Two read-only source audits and actual native navigation began; an untranslated Mac preference warning was confirmed on the English screen. The existing four catalog/preference tests passed, demonstrating that catalog key parity alone misses raw backend messages.

#tags: macos-menu-bar, glass-bubble, i18n-audit, lifecycle, arch:024, arch:025, supersedes:#legible-status-glass
