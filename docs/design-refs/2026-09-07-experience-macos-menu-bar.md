# Experience Contract: macOS menu bar

## Source Mode
- Mode: benchmark (approved Swift-to-Rust delta)
- Evidence: BroomSweepy/BroomSweepyApp.swift:87,232,456; Views/SettingsView.swift:119. User approved the same resident flow on 2026-09-07.

## Product Facts
| Claim | Source | Captured at | Freshness/status | Allowed presentation |
|---|---|---|---|---|
| RAM and CPU use | Local sysinfo sampler | runtime | current only after sample | Percent, not health or pressure score |
| System disk space | statvfs on / | runtime | current or unavailable | Free / total bytes, no file scan |
| Version | Cargo package version | build | current | Actual build version |

## Benchmark Sources
- Repository-owned Swift MenuBarExtra + 280pt panel, source inspected 2026-09-07. No old app launched; original live render unavailable. Desktop-only OS feature; mobile source unavailable/not applicable.
- Header/version → RAM/CPU/disk gauges → open/quit, 10s timer; showMenuBarPercent controls numbers only. Adopt task order; adapt typography to native AppKit accessibility; avoid fixed v1.0 and unnecessary entrance animations.

## Page Goal
- Read system use without opening the main window, and restore the same working context.
- Success: resident icon survives closing main window; title preference persists; native panel opens/closes; explicit Quit stops sampler and existing app workers.

## Audience and Tasks
- Current 8GiB Mac user with limited free disk; wants Swift-like one-click visibility.
- Start with resident icon; click to inspect; open main window or dismiss. Settings/panel can toggle memory percent. No scan or deletion is triggered.
- Risks: hidden app becomes inaccessible, stale values resemble current data, background sampling grows memory.

## Header and Navigation
- Brand + actual version; RAM → CPU → disk; freshness; memory-title preference; open app; Quit.
- Main desktop navigation is unchanged. macOS controls are separate from Windows tray implementation.
- Mobile: OS menu bar unavailable; existing performance screen remains the fallback.

## Core Message
- System status at a glance; closing a window is not quitting the app.
- Exact values and limited sampling scope are the evidence; no optimization claims.

## Content Integrity
| Content item | Classification | Evidence | Presentation rule |
|---|---|---|---|
| Measurements | verified at runtime | local CPU/RAM counters and root filesystem stats | Missing data is —, never fake zero |
| Native preview | prototype until run | installed build QA | Never claim verified before execution |

## Section Order
1. Identity/version: which app is resident.
2. RAM primary row, CPU and disk rows: current use with bars and text.
3. Freshness and preference: sampling scope and number visibility.
4. Open/quit: return to full workflow or stop residency.

## CTA Strategy
- Primary: BroomSweepy 열기, restore existing main window and its state.
- Secondary: 종료, explicit whole-app exit using existing shutdown.
- Toggle: 메모리 사용량 표시, numbers only; persisted locally and synchronized with settings.
- Outside click/Escape dismisses the panel without quitting. No deletion/cleanup shortcut.

## Trust Strategy
- Display last sample/freshness, use unavailable rather than guessed values.
- Only aggregate machine counters. No file list, process details, cloud access, or external requests.
- Failed native setup leaves ordinary window close behavior and unavailable settings feedback.

## Asset Provenance
| Asset | Source | Local path | License/trademark/attribution | Modification allowed | Status/fallback |
|---|---|---|---|---|---|
| Sparkles symbol/native controls | macOS AppKit system APIs | OS | Apple platform UI | limited | Text fallback if symbol unavailable |
| Settings icon | existing lucide-react | package dependency | existing package license | yes | existing icon system |

## Desktop Structure
- Native clear, nonopaque NSPanel320×384pt: the original320×370pt body plus14pt top beak,16pt margins, no separate WebView. On macOS26+ one NSGlassEffectView.Regular has an empty contentView as the background; existing full-opacity controls are a sibling above it. Only the background uses alpha0.82 (1.0 when Reduce Transparency is enabled, checked each opening). Earlier OS versions use one UnderWindowBackground/BehindWindow/Active effect, not a promise of identical translucency. No added tint, nested blur or whole-window alpha. One rounded bubble mask clips both layers, with a hairline rim. No OS preference changes.
- 2026-10-21 approved visual delta: Clear lost legibility, while plain Regular lost the beak and dimmed all native bars to gray. The user requested the beak, color and more translucency, then accepted the actual native component preview: “이 정도면 괜찮아”. Preserve task order/body geometry with DarkAqua, heading16pt medium, metrics15pt regular and labels14pt. The16pt rounded bubble sits8pt below the icon; its beak follows the actual icon center after screen clamping, bounded away from the rounded corners. Preview acceptance is not installation or full outside-click verification.
- Three measured rows; RAM first, exact value right,6pt persistent-color custom native gauges: RAM blue–cyan, CPU violet–blue, disk teal. Their values/0–100bounds/read-only progress roles remain accessible; unavailable readings hide the bar rather than implying zero. One-level surface, no nested cards, no scroll at ordinary system font sizes.
- Main settings panel reuses existing setting-row styles.
- Settings includes Open panel as a direct, keyboard-reachable way to discover the status panel.

## Mobile Transformations
| Desktop element | Operation | Mobile result | Reason |
|---|---|---|---|
| OS status icon/panel | remove | Not supported on mobile/non-Mac | OS-specific entry point |
| Main status detail | retain | Existing performance view | Main UI remains available |
| Settings row | compress | Existing responsive setting rows | No new layout system |

## States
| State | Trigger | User sees | Available action | Recovery |
|---|---|---|---|---|
| loading | First sample | — and measurement pending | Open / quit | Next sample |
| empty | Counter not available | — | Open app | Next sample |
| error | Native setup failure | Settings unavailable | Continue ordinary app | Restart |
| success | Valid sample | CPU/RAM/disk and sampling info | Toggle/open/quit | Automatic 10s refresh |
| stale | Older than 30s/sleep | Old measurement marker | Open app | Next successful sample |

## Performance Budget
- One reusable sysinfo System with CPU+RAM only; one worker; one latest snapshot, no history.
- Root statvfs only; no process enumeration, no per-file scan, no AI calls.
- No extra WebView, font download, image generation, or JS timer in the panel. Native controls/window are allocated once and reused. Local/global mouse-down monitors exist only while visible. The global callback ignores its event payload entirely and only dismisses; no keys/positions/app identities/history are read or retained.
- Stop channel interrupts the 10s wait on exit. UI work stays on main thread; sampling stays off it.

## Accessibility Contract
- Native labels/buttons/check states in logical reading order; exact text supplements bars.
- Escape dismisses the key panel. While shown, local left/right/other mouse-down outside the panel closes it, excluding the status button's window so its toggle cannot close then reopen. Original clicks are always delivered. Global mouse-down close-only covers clicks while our app is already inactive (the local+resign-only candidate physically failed). Own-app did-resign-active remains for app switching. No global key monitoring. Monitoring failure closes the panel and returns an error to Settings-open rather than a false success; callbacks are generation-gated to prevent a stale callback dismissing a reopened panel.
- Showing requests app activation and makes only this panel key without explicitly restoring the hidden main window. The own-app observer is removed before teardown, and its callback uses the retained panel without re-borrowing shared UI. Close invalidates the generation, takes both monitor tokens and releases all RefCell borrows before native unregistering. Native controls retain keyboard/AX semantics; actual keyboard and outside-click checks stay separate from source inspection.
- System semantic colors/fonts and native Glass accessibility behavior follow the OS. No custom entrance animation/timer or repeated live announcements. Actual reduced-motion/transparency/contrast modes were not changed or certified during this check.

## Adopt
- Swift resident icon, optional RAM title, 10s aggregate update, popup order, open/quit separation.

## Adapt
- Rust AppKit native controls replace SwiftUI, keeping one small reusable panel and avoiding a second browser process.
- Main window hides on close only after status icon successfully exists. Windows unchanged.

## Avoid
- Hardcoded version, background full process scans, fake health scores, permanent deletion controls, icon-hidden/no-reentry state.

## Prompt Contract
GOAL — Resident Mac status without losing main state.
AUDIENCE — Current low-resource Mac user.
TASK — Inspect aggregate status, toggle RAM title, restore or quit.
FLOW — Icon → panel → dismiss/open/quit.
HEADER — BroomSweepy and Cargo version.
MESSAGE — System measurements, not optimization promises.
FACTS — Local aggregate counters only.
CONTENT_INTEGRITY — Missing/stale values explicit.
SECTION_ORDER — Identity, RAM/CPU/disk, freshness/preference, actions.
CTA — Open main window; separate Quit.
TRUST — No files/process list/AI transmission.
ASSETS — Existing icon system and OS-native controls.
LAYOUT — 320pt native panel, fixed three rows, one-level glass.
RESPONSIVE — OS-only panel; existing responsive settings retained.
STATES — Pending, unavailable, stale, current.
PERFORMANCE — One 10s sampler, one snapshot, no extra WebView.
ACCESSIBILITY — Native controls, text+bars, Escape, no motion.
PRESERVE — Main views, Windows tray, safety and shutdown flows.
EXCLUDE — Swift target, automated cleanup, notification permissions.
SUCCESS — Native UI/lifecycle/preferences tests and resource observation pass.

## Success Checks
- Native icon/panel show correct content and reachable actions.
- Close → inspect → reopen preserves main UI, explicit Quit terminates.
- Settings and panel toggle agree after restart; number hidden leaves icon.
- Repeated panel opens do not spawn new WebViews or workers.
- Windows cfg isolation and existing tests remain intact.
