# Direction: Rust Native Glass Sweep

## Selection

- ID: `swift-native-glass-sweep`
- Selection Quote: “swift ui가 확실히 훨씬 깔끔한데. 먼저 이 ui부터 따라가야할듯?”
- Reinforcement Quote: “글래스모피즘 디자인이라서 더 다르지”
- Source artifact: `demo-assets/01_dashboard.png`, Swift `ContentView.swift`, `DashboardView.swift`, `SmartCleanView.swift`.
- Reference screenshot: `demo-assets/01_dashboard.png`, 2880×1800, dark, 재검토 2026-09-04.
- Current screenshot: `docs/assets/screenshots/v1.2.0-dashboard.png`, 1280×820, dark, 재검토 2026-09-04.

## Candidate Render Exemption

사용자가 프로젝트 소유 Swift 구현을 명시적으로 선택했고, 이번 범위는 해당 첫 화면의 delta recreation이다. 서로 다른 3개 방향을 탐색하면 선택된 기준을 희석하므로 후보 렌더를 생략한다. 구현 후 실제 Tauri 렌더를 reference와 나란히 비평하고 최대 두 차례 수정한다.

## Adapter Gate

- Product Design: `ABSENT` — 설치 가능하지만 현재 미설치.
- 실행 경로: local React/Tauri.
- 비교 결과: `NOT RUN`; 한 경로만 렌더하므로 adapter 우열을 주장하지 않는다.

## Direction Contract

- MODE: `Data Instrument`; 검사 중 `Waiting State`.
- PRIMARY: 크게 표시한 드라이브 상태를 보고 `이 드라이브 검사`.
- COMPOSITION: color-coded glass sidebar / compact briefing / 2:1 hero with one 240px active ring and a compact drive-card rail / four quick actions / evidence and recent activity.
- DENSITY: 첫 뷰포트 low, 결과 영역 controlled-medium.
- MESSAGE: 저장공간 상태를 한눈에 보고, 한 번 눌러 원인을 찾는다.
- CTA: primary `전체 검사 시작`; secondary `정리 후보 찾기`; 진행 중 해당 취소.
- TRUST: 읽기 전용 검사, 검증된 중복, 안전·검토 후보 분리, 최종 휴지통 확인.
- RESPONSIVE: 220px sidebar → 72px rail → overlay; hero 2:1 → single column; tiles 4 → 2×2 → 1.
- STATE: loading, scanning, cleanup-scanning, empty, error, success, stale.
- VISUAL SYSTEM: native vibrancy, near-black translucent fallback, neutral reflection edge, blue-violet ring signature, category colors only on icons.
- MOTION: interruptible FLIP transform for drive-card role swaps, CSS feedback, and SVG stroke; one entrance; no continuous animation.
- NEGATIVE: fake score, automatic deletion, nested blur, full-card gradients, equal-weight dashboard rows, macOS chrome imitation on Windows.
- SUCCESS: reference의 큰 여백·중앙 초점·원클릭 인상을 유지하면서 모든 수치와 행동이 Rust runtime에 연결된다.

## Motion Artifact

| Scene/component | User purpose | Trigger | Engine/plugin | Timing | Reduced-motion | No-JS fallback | Cleanup/test |
|---|---|---|---|---|---|---|---|
| 대시보드 첫 진입 | 브리핑→링→행동 읽기 순서 | 첫 마운트 | CSS opacity/transform | 220ms, 60ms 간격 | 즉시 표시 | 정적 DOM | 재마운트와 포커스 확인 |
| 저장공간 링 | 디스크 수치 변경 인지 | volume 변경 | SVG stroke transition | 360ms | 즉시 최종값 | 정적 링 | 0·100·데이터 없음 |
| 드라이브 카드 교대 | 여러 디스크를 한 초점으로 비교 | 작은 링 카드 선택 | FLIP + Web Animations API | 460ms, emphasized ease-out | 즉시 역할 교대 | 즉시 역할 교대 | 0·1·다수, 재클릭 중단, 제거, 키보드 포커스 |
| 빠른 실행 hover | 클릭 가능성과 대상 확인 | hover/focus | CSS transform/background | 160ms | transform 제거 | 정적 버튼 | keyboard focus 확인 |
| 검사 상태 | 기다리는 이유와 취소 확인 | 300ms 이상 작업 | CSS progress/state | 실제 단계 갱신 | 정적 상태와 텍스트 | 텍스트 상태 | 취소·완료·오류 전환 |
