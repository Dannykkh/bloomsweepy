# Direction: Rust Performance Glass Instrument

## Selection

- ID: `swift-performance-glass-instrument`
- Selection quote: “swift 에 보면 메모리 정리하기 기능도 있지 않나? cpu정리나 사용량 파악도??”
- Source artifact: Swift `PerformanceView`, 기존 Rust/Tauri dashboard와 `DESIGN.md`.

## Candidate Render Exemption

사용자가 이미 SwiftUI의 UI·UX를 Rust 기본 프로젝트에 합치는 방향을 선택했다. 이번 작업은 그 방향의 성능 화면 delta이므로 서로 다른 세 방향 탐색을 생략하고 설치 앱 렌더 비평에 예산을 사용한다.

## Adapter Gate

- Product Design: `ABSENT`.
- 실행 경로: local React/Tauri.
- 비교 결과: `NOT RUN`; 외부 adapter 결과보다 프로젝트 소유 golden master를 우선한다.

## Direction Contract

- MODE: `Data Instrument`.
- PRIMARY: CPU·RAM 상태를 실제 수치로 확인하고 BroomSweepy 자체 메모리를 한 번에 반환한다.
- COMPOSITION: compact briefing / shared heading + equal-sized CPU and RAM rings / bounded process panel / confirmation dialog.
- DENSITY: first viewport low-to-controlled-medium.
- MESSAGE: 느려지는 원인을 실제 수치로 확인합니다.
- CTA: hero-level `앱 메모리 정리` (BroomSweepy only); row-level `종료 요청`; stale/error utility `지금 새로 고침`.
- TRUST: timestamp, allocator-reported current-process 반환량, no pressure allocation, no other-app purge, backend identity revalidation, normal termination only.
- RESPONSIVE: 1:1 instrument → stacked below 560px content width; table → two-line rows; dialog → viewport-width sheet.
- STATE: measuring, loading, success, stale, error, empty, cleaning-memory, memory-clean-result, terminating, result.
- VISUAL SYSTEM: existing native vibrancy and blue-violet ring signature; category color localized to the performance icon.
- MOTION: 180ms opacity/transform action feedback; CPU/RAM arcs ease to new samples over 900ms with CSS stroke-dashoffset. Numeric/ARIA values update immediately; polling never triggers entrance or live announcements. Reduced motion snaps to the final arc.
- NEGATIVE: system-wide reclaim claims, pressure allocation, hardcoded app severity, auto/force termination, nested blur, rainbow metric cards.
- SUCCESS: Swift의 넓고 원클릭인 인상을 유지하면서 모든 값과 종료 판단이 Rust runtime 근거를 가진다.
