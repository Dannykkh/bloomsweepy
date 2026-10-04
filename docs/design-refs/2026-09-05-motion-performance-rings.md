# Performance-ring Motion Delta

- User request: “갱신할때마다 뚝뚝 끊기는데, 스무스하게 그래프보이면 안됨?”
- Source mode: local delta; preserve approved equal-ring glass layout, data sampler and cleanup behavior. Existing local adapter, no plugin or dependency additions. Three new directions are unnecessary for this bounded motion correction.
- Cause: MetricRing updates its SVG dash offset directly, with no transition. The monitor retains the current snapshot while fetching, and stable ring DOM can be reused across samples.

| Scene/component | User purpose | Trigger | Engine/plugin | Timing | Reduced-motion | No-JS fallback | Cleanup/test |
|---|---|---|---|---|---|---|---|
| CPU and RAM meter arcs | Follow the direction and size of a measured change without a visual jump | New runtime sample | CSS stroke-dashoffset transition, no library | 900ms cubic-bezier(0.22, 1, 0.36, 1), no delay | transition:none; latest state immediately | Existing static SVG target when rendered; no separate animation library | DOM teardown cancels transition; inspect intermediate/end frames, rapid retarget and unchanged geometry |

- Keep numbers and accessible meter values exact and current; do not fabricate extra samples or smooth the underlying data.
- First mount shows its actual reading, not an artificial 0→value sweep. A sample arriving during a transition retargets from the current presentation.
- No requestAnimationFrame polling, global animation loop, extra IPC, permanent will-change, or animation of the glass surface.
- Runtime checks use an opt-in deterministic moving fixture; default fixture and production data remain unchanged.

## Render and Interaction Evidence

- 1280×820 fixture, CPU 18→76%: painted dash offset progressed 443.090 → 229.007 → 141.716 → 129.685px, reaching the exact 129.684945px target within rendering precision. RAM 50→78% progressed 270.177 → 166.827 → 124.686 → 118.878px.
- Both meters retained their gradient IDs across refreshes; card size and position stayed fixed after the initial entrance completed. A subsequent refresh during motion retargeted the in-flight arc, not an artificial zero origin.
- 760×600 fixture retained the compact layout and primary action; consecutive manual and automatic samples continued to interpolate. Screenshots were inspected live, not saved as new static assets.
- Reduced-motion is handled explicitly with `transition: none`, reviewed in source. System-wide accessibility preferences were not changed during QA; forced reduced-motion runtime and dropped-frame profiling were not run.
- Existing frontend tests (32), TypeScript, and Experience Contract validation passed. No new dependencies, frame-by-frame React updates, or sampler changes.
- Final ARM64 Tauri build succeeded; strict/deep ad-hoc signature checks passed, and the installed host binary matches the built bundle. `/Applications/BroomSweepy.app` was relaunched and the native Performance screen inspected with actual CPU/RAM values. Previous install is recoverable at `/Users/dannysmacair/.Trash/BroomSweepy-before-smooth-rings-20260905-114430.app`.
