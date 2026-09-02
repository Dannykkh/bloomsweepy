# Direction: 조건부 Docker 용량 작업면

## Selection

- ID: `conditional-docker-workspace`
- Selection Quote: “도커사용하면 도커용량 메뉴가 따로 나와서 거기서 처리해야하는거 아님? 대시보드엔 안나오더라도?”
- Supersedes: `existing-data-instrument-delta`. 설정 안에 상세를 펼치는 방식은 기능을 다시 찾기 어렵다는 사용자 관찰로 폐기한다.
- Candidate render exemption: 기존 Data Instrument 토큰과 행 anatomy를 보존하면서 조건부 내비게이션과 전용 작업면으로 재배치하는 승인된 정보 구조 delta다.

## Direction Contract

- MODE: 전용 화면은 Data Instrument, 실행은 Waiting State, Docker 대화는 Agent Workbench.
- PRIMARY: Docker 사용량 확인 후 정리 검토.
- COMPOSITION: 설정 opt-in → 조건부 nav → 상태·총량 → divide-y 사용량 → 대화/검토 행동 → 한 겹 dialog.
- DENSITY: medium, 행 60~76px, 텍스트 최소 14px.
- STATE: disabled, loading, unavailable, ready, preview, running, partial, success.
- COLOR: 기존 near-black·steel-blue, 위험 확인에 기존 warning/danger만 국소 사용.
- MOTION: 진행 상태 전환만, 장식 효과 0.
- NEGATIVE: 비활성 사용자에게 Docker 메뉴 노출, 대시보드 Docker 위젯, Docker 카드 벽, 볼륨 자동 정리, 임의 셸, 가상 디스크 직접 조작.
- SUCCESS: 일반 사용자에게 비용이 0이고 Docker 사용자는 전용 작업면과 폴더 없는 Docker 대화에서 안전한 앱 실행으로 이어진다.
