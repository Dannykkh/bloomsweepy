# Layout Blueprint — Chat access delta

Agent Workbench, 기존 DESIGN.md compact composer 재사용. 단일 역할.
textarea 아래 `[CLI] [모델 · 추론] [권한]`와 우측44pxsend/cancel.
모델/권한의 공유 wrapper는 position:relative; 권한 popup은 wrapper폭 기준으로 위쪽 정렬한다.
390px에서는 모델 이름부터 압축하고 필요시 toolbar행을 분리한다. 초안/진행/전송을 가리지 않는다.

권한 popup: 제목 → 조회만/확인 후 실행/전체 접근 선택지 → 짧은 제외 범위.
설정: 권한 유지 방식 → 채팅 모드 → 외부MCP 별도 권한. 모드 필드가 있는 새 host에서는
중복 확인 생략 체크박스를 숨긴다. confirm은 항상 확인하며 이전 flag로 전체 접근을 이관하지 않는다.
볼륨/전체 이름 검색 결과: 부분 여부 → 이름/용량/볼륨 → 대상 열기. 클릭은 삭제가 아니라 기존 workspace 진입.

#tags: layout, chat-access, compact-composer
