# Direction: 쉬운 용량 관리와 AI 도우미

## Selection

- ID: `A-guide-first-map`
- AUTO_SELECTION: 사용자 요구인 쉬운 사용 순서, 트리맵 전면 배치, AI/CLI 기본 화면 제거를 가장 직접적으로 만족한다.
- Source artifact: `.termsnap/design-candidates/simple-storage.html#a`
- Desktop screenshot: `.termsnap/design-candidates/simple-storage-a.png`, 1280×820, dark, 2026-09-01

## Direction Contract

- MODE: Data Instrument + Faceted Directory, AI 도우미는 제한된 Agent Workbench 상태 화면.
- COMPOSITION: 5개 전역 메뉴 → 용량 관리 4개 하위 탭 → 3단계 안내 → 전폭 트리맵.
- MESSAGE: 폴더를 고르고 큰 사각형부터 따라가면 용량 원인을 찾는다.
- CTA: 현재 단계의 다음 행동 하나만 solid primary.
- TRUST: 읽기 전용 검사와 실제 경로를 지도 행동 직전에 표시한다.
- RESPONSIVE: 안내 단계를 세로로, 지도 순위를 아래로 이동한다.
- STATE: empty, loading, success, error, disconnected.
- VISUAL SYSTEM: 기존 near-black canvas, 단층 glass, blue interaction, 다색 treemap semantic neutral.
- MOTION: 기존 220ms 진입과 상태 전환만, 신규 장식 모션 없음.
- NEGATIVE: 첫 화면 CLI 권한, 가짜 채팅 입력, 드라이브 14개 빈 행, 이중 포커스 링.
- SUCCESS: 처음 보는 사용자가 5초 안에 폴더 선택과 용량 지도 행동을 찾는다.

## Candidate Render Compliance

핵심 IA 변경과 “복잡하다”는 사용자 평가가 있었으므로 실제 정적 후보 3개를 1280×820으로 렌더했다. 후보는 동일한 카피·프로토타입 데이터를 사용했으며 구성, 안내 위계, 지도 비율, 결과 이동 위치를 달리했다.
