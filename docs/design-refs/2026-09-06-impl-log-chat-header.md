# Implementation log: Chat header overlap delta

- Contract: [Experience Contract](2026-09-06-experience-chat-header.md), validator passed without warnings.
- Preservation: existing glass, fonts, sidebar, input behavior, provider adapter, and saved sessions.
- Production delta: AppShell assistant-specific class; App.css reuses the existing storage-page normal-flow rule. DESIGN.md records the scrolling policy.
- Regression support: standalone, clearly labeled layout fixture uses real AppShell/CSS with representative synthetic messages; excluded from production entry.
- Render critique round 1: reproduced sticky title overlap; one scoped correction removed it. Three visual directions are exempt for this approved local fix. No second revision needed.
- Verification: 15 viewport/state combinations, keyboard focus at two widths, unchanged control-page header policies, native installed long conversation, typecheck, 5 assistant tests, ARM64 release build, and strict local signature verification passed.
- Module coverage and plugin gate are recorded in the contract. No design plugin installed; local adapter used. No new assets, effects, dependencies, or runtime measurements.
- Full evidence and limits: [QA report](../qa/2026-09-06-chat-header-layout.md).
