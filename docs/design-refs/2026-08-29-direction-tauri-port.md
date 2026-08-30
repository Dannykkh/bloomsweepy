# Direction: BroomSweepy Tauri Port

## Selection

- ID: existing-dark-glass
- Selection Quote: "지금 ui는 글래스모피즘으로 되어있잖아. 따라가나? 타우리?"
- Source artifact: `docs/promo.mp4`, `BroomSweepy/ContentView.swift`, `BroomSweepy/Views/DashboardView.swift`

## Candidate Render Exemption

이 작업은 브랜드 가이드가 없는 신규 디자인이나 핵심 구도 변경이 아니다. 사용자가 기존 글래스모피즘의 보존을 직접 지정했고 실제 제품 영상과 소스가 있으므로 3방향 후보 렌더를 생략한다. 영향 범위는 동일한 방향을 CSS와 플랫폼 창 재질로 번역하는 데 한정한다.

## Direction Contract

- MODE: Data Instrument, 스캔 중 Waiting State.
- COMPOSITION: 고정 사이드바, 상태 브리핑, 중앙 저장공간 링, 단일 주 행동, 비교 가능한 결과 목록.
- MESSAGE: 용량 증가 원인을 찾고 삭제 전 근거를 검토한다.
- CTA: `스캔 시작`, 보조 `폴더 선택`, 진행 중 `스캔 취소`.
- TRUST: 마지막 스캔 시각, 실제 경로, 전체 콘텐츠 검증, 휴지통 이동.
- RESPONSIVE: 사이드바 → 아이콘 레일 → 오버레이 내비게이션.
- STATE: loading, empty, error, permission, success, cancelled, stale.
- VISUAL SYSTEM: near-black canvas, 단층 dark glass, blue-violet storage sweep, semantic status colors.
- MOTION: CSS transform/opacity, 첫 진입 1회, 결과 변경 시 링 전환, reduced-motion 정적.
- NEGATIVE: macOS 신호등 복제, 중첩 블러, 무한 글로우, 카드만 반복하는 대시보드.
- SUCCESS: Windows WebView2 렌더에서도 기존 BroomSweepy와 동일한 제품으로 인식된다.

## Motion Artifact

| Scene/component | User purpose | Trigger | Engine/plugin | Timing | Reduced-motion | No-JS fallback | Cleanup/test |
|---|---|---|---|---|---|---|---|
| 첫 화면 등장 | 정보 읽기 순서 제시 | 앱 첫 마운트 | CSS opacity/transform | 220ms, 60ms stagger | 즉시 표시 | 즉시 표시 | 재마운트 후 중복 실행 확인 |
| 저장공간 링 | 스캔 결과 변화 인지 | 요약 값 변경 | CSS stroke transition | 360ms | 즉시 최종값 | 정적 링 | 값 0·100 경계 테스트 |
| 내비게이션 전환 | 현재 위치 확인 | 페이지 선택 | View Transition progressive enhancement | 180ms | 즉시 전환 | 즉시 전환 | 포커스와 읽기 순서 확인 |
| 진행 상태 | 현재 단계 확인 | 300ms 이상 스캔 | CSS transform | 단계 갱신 기준 | 정적 진행 바 | 텍스트 상태 | 취소와 완료 전환 테스트 |
