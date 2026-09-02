# Direction: 대시보드와 폴더 중심 대화

## Selection

- ID: `A-activity-dashboard + C-scoped-chat`
- AUTO_SELECTION: 사용자가 별도 대시보드와 단순한 채팅 작업면을 명시했으므로 화면별로 A의 데이터 구조와 C의 대화 구조를 하나의 정보 구조로 선택한다.
- Source artifact: `.termsnap/design-candidates/dashboard-chat-workspace.html`
- Screenshots: `.termsnap/design-candidates/dashboard-chat-a.png`, `dashboard-chat-b.png`, `dashboard-chat-c.png`
- Viewport: 1280×820, dark, 2026-09-01

## Direction Contract

- MODE: Dashboard는 Data Instrument, 대화는 Agent Workbench, 검사는 Waiting State.
- PRIMARY: 대시보드에서 위험 드라이브 진입, 대화에서 현재 폴더 질문 전송.
- COMPOSITION: 행 기반 드라이브 → 최근 정리/새 파일 비대칭 분할; 대화는 scope/thread/composer 단일 pane.
- DENSITY: medium, 데이터 행 58~72px, 텍스트 최소 14px.
- STATE: loading, baseline, empty, stale, error, disconnected, success.
- COLOR: 기존 near-black·steel-blue와 국소 amber 경고만 사용.
- MOTION: view transition과 상태 전환만, 장식 효과 0.
- NEGATIVE: 균일 KPI 카드, 큰 온보딩 설명, 가짜 채팅, 첫 색인 전체를 새 파일로 표시.
- SUCCESS: 첫 화면에서 드라이브와 최근 변화를 읽고 한 번의 클릭으로 상세에 진입한다.
