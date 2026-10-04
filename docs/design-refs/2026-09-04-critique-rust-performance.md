# Render Critique: Rust Performance Instrument

## 2026-09-05 Smooth-value Motion Follow-up

- User feedback identified abrupt sample-to-sample ring jumps. The prior no-value-animation constraint is superseded by the bounded [900ms CSS motion contract](2026-09-05-motion-performance-rings.md).
- Intermediate computed SVG offsets at desktop and compact sizes confirm real interpolation and stable geometry; numeric/ARIA readings remain immediately accurate. No whole-card loop or zero-origin replay was added.

## 2026-09-05 Equal-ring Delta (Current Composition)

- Reviewed the actual source-rendered Vite fixture at 1280×820 (Korean), 760×600 (Korean and English), and the 520×760 stacked transformation. These are development fixtures, not native/release screenshots.
- Desktop CPU and RAM rings both measure 226×226px at the same vertical coordinate; compact rings both measure 184×184px. Their SVG gradient IDs are distinct.
- CPU retains the blue-violet signature; RAM uses blue. One shared heading makes both measures peers. Available and swap values remain directly under RAM.
- Review correction: a short-window rule reduces header/spacing so the complete 63.65px action fits at y=513.5–577.15 in a 760×600 first viewport. The `BroomSweepy 전용` subtitle is never hidden.
- No horizontal overflow at all three widths. Below 560px content width the rings stack without shrinking their labels.
- Pointer and Enter activation both produced the fixture's 38 MB result; zero allocator bytes produced the explicit no-releasable-memory success state. Two-second values remain outside live regions.
- Current card corners now share a 20px radius. The prior PNGs below are historical and do not represent the equal-ring composition.
- Final installed native window at 1154×768 was inspected: both meters aligned, real 8 GB Mac readings were visible, and one current-process cleanup correctly produced the zero-return success state. Native screenshots were observed live, not saved as repository assets.

## Reviewed Renders

- `docs/assets/screenshots/v1.5.0-performance-1280x820.png`
- `docs/assets/screenshots/v1.5.0-performance-760x600.png`
- `docs/assets/screenshots/v1.5.0-performance-termination-dialog-1280x820.png`

## What Works

- The CPU ring is the sole violet signature and establishes the first visual anchor without competing with the navigation.
- The memory rail uses one blue accent and exact used/total/available values, so color supports rather than replaces the numbers.
- Wide layout exposes the first process rows in the opening viewport; compact layout preserves the CPU-first reading order and moves memory below it without horizontal overflow.
- The process surface becomes denser only after the low-density system instrument, matching the task flow from overview to cause.
- The termination dialog names the exact app, PID and last verification time, keeps Cancel as the default focus, and disables the action until the unsaved-work acknowledgement is checked.

## Corrections Made During Review

- Replaced the memory bar's blue-violet gradient with a single blue accent so the CPU ring remains the only signature gradient.
- Replaced div-based process rows with a semantic table while preserving the compact two-line visual transformation.
- Raised refresh, sorting and row actions to at least 44px hit targets and applied tabular numerals to changing metrics.
- Added polite manual-refresh status, modal scroll containment and reduced-motion behavior without making the two-second polling loop noisy to assistive technology.
- Stabilized process-row keys across two-second snapshots and added a disabled/disconnected-trigger fallback so keyboard focus returns to a valid refresh control.
- Retained the exact AppKit application object from snapshot through execution, removing the PID re-lookup race before a normal termination request.

## Residual Limits

- The compact screenshot intentionally shows only the system instrument above the fold; the process table remains available by vertical scrolling.
- Visual regression uses the release CSS fixture because native-window capture is not available in the automated environment. The packaged application was separately built, signature-verified, installed and launch-verified.
- No real user application was terminated during QA. Termination safety is covered by identity-policy unit tests and the non-destructive confirmation flow.

## Final Assessment

The screen now follows the SwiftUI golden master's spacious hierarchy and one-action-per-row UX while keeping Rust/Tauri as the only product runtime. No launch-blocking visual, accessibility or safety issue remains in the reviewed scope.

## 2026-09-05 Memory-clean Delta Review

### Reviewed Live Renders

- 1280×820 release-CSS fixture: default and 38 MB allocator-return success states.
- 760×600 release-CSS fixture: success state with the primary action and result visible in the first viewport.
- Installed `/Applications/BroomSweepy.app` window at 1154×768: actual macOS command completed and displayed the zero-byte completed state.

The live Computer Use captures were inspected during implementation and were not added as static repository assets. The existing PNGs above remain the pre-delta composition baseline.

### What Works

- The full-width solid-blue action is the only hero-level CTA and retains a 63.6px measured hit target with an icon, result label and one-line scope.
- The exact scope copy sits immediately below the button: only unused memory from the BroomSweepy host is returned, and other apps are untouched.
- Positive allocator bytes and the valid zero-byte completion case have distinct success copy. Neither RSS nor system available-memory movement is presented as reclaimed memory.
- The success result stays in the memory pane rather than opening another modal, preserving the Swift one-click rhythm.
- At 760×600 there is no horizontal overflow (`scrollWidth === clientWidth === 760`) and the action remains above the fold.

### Corrections Made During Review

- Split the former 820px responsive rule: process columns still compact at 820px, while the CPU/memory instrument stacks only below 740px. This keeps the new primary action visible at the previously approved 760×600 review size without squeezing the table.
- Replaced the contradictory `읽기 전용 측정` label with `실시간 측정`.
- Added one polite live announcement for user-triggered progress/completion and an alert for failure; automatic two-second metrics remain silent.

### Runtime Evidence

- The installed ARM64 app exposed the button through the accessibility tree and completed one real invocation with `allocatorReleasedBytes = 0`, correctly reporting that no current host memory was available to return.
- No user application was terminated and no memory-pressure allocation was performed during QA.
