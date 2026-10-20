# Layout blueprint — workspace navigation

## Desktop (1280×820)

- Existing 224px native-glass sidebar; computer / files & storage / tools groups. Settings at bottom, disk status retained.
- Dashboard: short heading/performance/refresh row; compact left drive deck + right measured summary and scan actions; four short quick actions; history and recent files below.
- Settings: 176px category navigation + minmax(0,1fr) content, 24px gap. Only one of six mounted categories visible.
- Main engine, routes, selections and native permission callbacks are composition inputs, not duplicated by navigation.

## Narrow

- Existing icon rail at intermediate widths and overlay at 680px or less; overlay makes main inert and Escape returns focus.
- Dashboard: heading → performance/refresh → selected drive information → ring/drive rail → scan actions. Matches DOM order.
- Settings container ≤760px: wrapped category buttons; ≤500px: vertical form controls. Category targets ≥44px; text ≥14px.
- Scroll lists/drive rail within their own region; no document/main horizontal overflow at tested widths.

## Flow distinction

Mac settings / dashboard / overview system-cleanup entry → set system mode → existing cleanup route. Generic navigation never resets mode, so folder candidate trees remain usable.

Source contract: [experience](2026-10-20-experience-workspace-navigation.md). Actual renders: [verification](../qa/workspace-navigation/verify-report.md).
