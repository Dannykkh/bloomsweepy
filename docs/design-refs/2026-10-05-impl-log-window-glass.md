# Window chrome and glass material delta

## Brief / Source Mode

- User: “이 프로그램 타이틀바가 사라졌던데? 글래스모피즘도 약하고?” → “진행하자”.
- Approved local delta of `swift-native-glass-sweep`: restore visible native title/chrome; expose the existing native material. No navigation, content, file action, provider, startup, or security changes.
- Existing contract: [experience](2026-09-04-experience-rust-swift-experience.md), [direction](2026-09-04-direction-rust-swift-experience.md), [layout](2026-09-04-layout-rust-swift-experience.md), root `DESIGN.md`.
- Three candidate renders exempt: approved direction, hierarchy, tasks, responsive structure, and Swift golden master remain unchanged.
- System roles: NOT APPLICABLE — single-user desktop app.

## Evidence / Prompt Contract

- `hiddenTitle: true` hides the already-set app title. Native decorations/AX controls are present; this is not a fully frameless window.
- Body 54–72% dark wash + shell 34% + sidebar 62% compounds opacity and masks native vibrancy.
- Preserve real OS titlebar and native control placement, 224px sidebar/72px rail, storage ring, existing text/action hierarchy, safety confirmations, fonts, motion, and data.
- Change only title visibility and background composition. Use one body wash; no animated or additional blur. Keep browser/Windows canvas opaque, and provide reduced-transparency CSS fallback.
- Desktop native checks: title/version, traffic lights, drag, minimize/restore, close-to-hide/reopen, dashboard/performance/assistant navigation. Browser checks: desktop/compact/narrow render, font loading, focus, fallback, long/error/loading transcript.
- No new IA, routes, assets, signature effect, or motion. Existing sitemap/motion contract retained.

## Adapter / Module Coverage

- Product Design Gate: ABSENT. `codex plugin` help supports all three list commands. Configured marketplaces: openai-primary-runtime, openai-bundled, openai-curated. Available exact selector `product-design@openai-curated-remote` version 0.1.56; installed matches 0; no current session capability. Optional recommendation made once; no install or settings mutation. Adapter: local.
- Adapter comparison: NOT REQUESTED / NOT RUN — no new adapter introduced.
- `frontend-design`: module `/Users/dannysmacair/.codex/.olympus/source-skills/frontend-design/SKILL.md`; Data Instrument playbook and layout anatomy read; existing product-derived recipe preserved.
- `mermaid-diagrams`: module `/Users/dannysmacair/.codex/.olympus/source-skills/mermaid-diagrams/SKILL.md`; existing sitemap retained, no new diagram needed for material-only delta.
- `ui-ux-auditor`: module `/Users/dannysmacair/.codex/.olympus/source-skills/ui-ux-auditor/SKILL.md`; scoped visual/contrast/focus/responsive/performance audit, not a full-product certification.
- `web-design-guidelines`: module `/Users/dannysmacair/.codex/.olympus/source-skills/web-design-guidelines/SKILL.md`; fresh [official guidelines](https://raw.githubusercontent.com/vercel-labs/web-interface-guidelines/main/command.md) fetched 2026-10-05.
- design-system-starter, Stitch, skill-evolve, autoresearch: NOT REQUESTED.
- DESIGN.md interactive lint: NOT RUN — not used as a headless pass signal.

## Implementation / Render Review

- Round 1: restored title visibility, one 24–44% body wash, transparent shell, 28% native sidebar; native sidebar/hero no longer repeat CSS blur. Increased native secondary text brightness; opaque browser/Windows and reduced-transparency fallback remain.
- Native installed round 1: traffic lights now visibly present; title is a native AX text node. Transparent titlebar screenshot has no stable visible title contrast, so round 2 switches to OS `Visible` titlebar rather than adding web chrome. Material remains native behind the content.
- TypeScript check and all 51 frontend tests PASS. After the final titlebar adjustment, both config tests PASS. Production build PASS; existing 872 kB chunk advisory retained. App-only ARM64 bundle/ad-hoc signature verification PASS; this is a local development installation, not an Apple-notarized release.
- Browser fixture: 1280×820 / 760×600 / 390×844 no horizontal overflow; 72px compact rail; narrow drawer Escape restores toggle focus; long message wraps; error remains actionable; test submit yields local-only status; loading input disabled. Pretendard Korean font check true / font status loaded. No browser console errors. Responsive screenshots must be taken after the existing transition settles, not during resize.

## Final Installed Render / Scoped Audit

- Native round 2: `/Applications/BroomSweepy.app` displays the OS title/version and traffic lights with `Visible` chrome, separate from the glass content. Dashboard, CPU/RAM performance, and assistant navigation remain usable. Active and inactive renders were inspected; the final screenshot captures the inactive window, so traffic lights are gray.
- Close-to-hide leaves the app running; reopening preserves the selected assistant view and its existing synthetic conversation. No new external model request, scan, deletion, or provider/settings change was made. Saved data is retained.
- Titlebar drag and native minimize/raise actions were exercised. The native tool does not expose global window displacement or an independent minimized-state flag; those state changes are not claimed as separately measured.
- Native root replaces redundant sidebar/hero CSS blur with existing AppKit material. Current render has readable secondary text and visibly less obscured glass. This is not a contrast certification across arbitrary wallpapers, nor a long-term GPU/RAM measurement.
- Actual installed screenshot: `docs/ui-audit/screenshots/window-glass-native-macos.png`. Browser-only compact fixture: `docs/ui-audit/screenshots/window-glass-compact.jpg`; it is not native product evidence. Screenshots remain ignored by the existing Git policy.
- Installed host SHA-256: `77614c5b67d45935c8804bffe1bc8399166e0b0d5cfba2f9cac9393994d3737c` (matches the final bundle). Version remains 1.7.0. Original rollback app: `/private/tmp/broomsweepy-glass-install-backup-3BovoX/BroomSweepy.app`.
- NOT RUN: native compact-window resize, Windows runtime, OS reduced-transparency/motion toggles, all-wallpaper contrast sampling, long-term resource/FPS soak, full Rust regression rerun. Windows/opaque defaults are covered only by the existing configuration tests and browser fallback render. OS preferences were not mutated.
- UI implementation/installation stage: no version bump, Git staging/commit/push, public release replacement, dependency addition, or unrelated `demo-assets/` changes.
- Follow-up user request “커밋 푸시하자.” authorizes committing/pushing this UI delta and its curated documentation/records. Existing `demo-assets/`, ignored screenshots, build/install artifacts remain excluded; no version/tag/public-release change is authorized.
