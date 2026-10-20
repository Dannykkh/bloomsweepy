# System candidates — screen correction

Date: 2026-10-21 (KST)
Scope: Storage → Cleanup candidates → System candidates. The folder candidate tree is not the requested screen. Release v1.8.0 is confirmed but publication is deferred until this correction is verified.

## Current-run audit evidence

1. [Installed empty screen](01-before-empty.png): the native app was already on System candidates. Strength: the mode is explicitly selected and scanning is read-only. Defect: the status surface shrinks to a narrow strip, breaking Korean text into a vertical column. Two primary scan buttons repeat the same action; the second empty-state surface starts below the viewport.
2. [Installed completed scan](02-before-results.png): the existing read-only scan completed, but the status surface is still a narrow vertical column and pushes the summary, filters and candidate list far below the viewport. The accessible tree contains results; this is a layout failure, not a missing-result claim. No candidate was selected and no file was moved or deleted.

## Correction contract

- Keep the existing dark glass, type and spacing tokens. This is a repair of the existing code target, not a new visual direction.
- Give both nested cleanup workspace and size-query view an explicit full-width, min-width-zero, single-column track; do not change global view-stack sizing for unrelated screens.
- Keep one initial scan action. Keep scan/cancel, type filters, evidence, sizes, file actions and selection review readable across actual container sizes.
- Use a consistent mode switch and a heading description valid for both folder and system candidates. Remove the repeated system-mode introduction.
- Separate Mac copy from Windows-only installation-record inspection. Keep developer-cache rebuild warnings, review acknowledgements, final confirmation, backend revalidation and Trash execution unchanged.

## Verification status

The existing-view correction is implemented. TypeScript checking and all192 frontend regressions passed, including8 new tests executing the real CleanupView with fake hooks and fail-closed IPC dependencies. No Rust execution/selection/acknowledgement policy was changed. Before/after browser fixtures use synthetic paths and mock all operating-system calls; these are distinct from the installed native audit above. No destructive native verification is authorized for this task.

3. [Empty](03-after-empty-desktop.png) and [scanning](04-after-scanning-desktop.png): full-width status and exactly one start/cancel action. Synthetic start reached results; the fixed scanning scenario was captured separately rather than labeling a completed screenshot as progress.
4. [Results at1280](05-after-results-desktop.png), [1100](06-after-results-medium.png), [760](07-after-results-760.png), [390](08-after-results-mobile-top.png): explicit row areas, wrapped paths/evidence and no horizontal overflow. The760 view intentionally requires vertical scrolling for row actions; warnings are not hidden to force everything into the first viewport.
5. [Selection](09-after-mobile-selected.png) and [developer-cache review](10-after-developer-review-dialog.png): one640MiB synthetic selection, rebuild/download acknowledgement, disabled final action until acknowledgement, initial Cancel focus, Tab wrap to Close and Escape dismissal. The final destructive action was not submitted. After dismissal, focus went to the page rather than the invoking review button: this is a pre-existing shared-dialog accessibility gap, not a verified focus-return pass.
6. [Windows/English](11-after-windows-english.png): no Mac-only developer-cache filter; literal Windows installation-record inspection remains read-only. This is a browser fixture, not a native Windows runtime test.
7. [Partial result](12-after-partial-warning.png), [scan failure](13-after-scan-error.png), [zero candidates](14-after-zero.png): retry/empty state distinguish failures from a successful zero report, and the partial warning appears before the summary/filter/selection surfaces. Unreadable or issue-bearing reports also receive the existing partial-result warning.
8. [Japanese](15-after-japanese-760.png) and [Simplified Chinese](16-after-chinese-760.png): current localized copy and wrapped controls. The item count stays on one line and the Korean confidence badge no longer breaks its last word into isolated characters.

The recorded [DOM measurements](layout-measurements.json) contain12 observed states across1280/1100/760/390 viewports. All keep view width equal to workspace width, no root/view horizontal overflow, visible type at least14px, button/selection targets at least44px and candidate children inside the row bounds. Pretendard reports loaded in the observed states. Rejected transient entrance-animation screenshots were replaced with settled captures. Temporary viewport overrides were reset.

Screenshot inspection alone is not a full accessibility audit. Keyboard/focus, overflow, type, control sizing, fonts and state-specific behavior require their own checks below before completion. Windows interactive runtime and full VoiceOver verification remain separate from the Mac/browser layout task.

Not verified here: light theme/full composite contrast certification, forced reduced-motion runtime, every assistive-technology path or native deletion. Existing reduced-motion CSS remains unchanged. The installed native build is still the pre-correction1.7.0 audit build at this snapshot; fresh packaging/native verification follows the release step.

## Next action

Proceed with the already approved v1.8.0 release packaging. Keep native read-only verification and release-asset validation separate from these fixture results, and do not claim that known chat/OS/Windows/long-memory gaps are closed by this layout patch.

#tags: system-candidates, layout-regression, bounded-audit, no-deletion, release-gate
