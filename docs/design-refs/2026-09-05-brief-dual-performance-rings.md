# Delta: Equal CPU and Memory Rings

## User decision

“메모리도 원형이 같이 나와야 하지 않을까?” → “그래? 그럼 다음을 진행해봐”

## Layout (supersedes the 2026-09-04 asymmetric layout blueprint)

- One shared system-performance heading; two equal-width panes with identical ring diameter and vertical alignment.
- CPU: blue-violet ring, logical-core count, sample window, explanation that reducing running work reduces load. No CPU-clean action.
- Memory: blue ring, used / total bytes, available and swap values, one-click `앱 메모리 정리` with BroomSweepy-only scope.
- At less than 560px content width, stack CPU then memory without shrinking text or touch targets.
- Current usage is not labeled as memory pressure, and no fabricated health grade is added.

## Validation

- Existing TypeScript build and frontend tests; desktop and compact rendered checks for equal rings, overflow, accessible meter names, and cleanup feedback.
- Rebuild and inspect the installed macOS application using actual Rust measurements.
- Preserve current cleanup and normal-termination backend behavior; no new permissions or system-wide purge.
