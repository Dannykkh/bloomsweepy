# Brief — 작업 중심 메뉴와 설정

사용자 요청: 전체적으로 부족해 보이며 메뉴/UI 구성이 모두 맞춰졌는지 질문한 뒤 “진행해”. 기존 감사의 실제 설치 화면3개와 현재 소스를 출발점으로 한다. 새 브랜드·시스템 기능 추가가 아니라 전체 정보구조와 주요 화면의 연결을 정비한다.

- 대상: 저장공간이 부족한 개인 Mac/Windows 사용자. 단일 로컬 사용자이며 AI는 별도 권한을 가진 호출자다.
- 과업: 현재 상태 파악 → 적절한 검사/검색/정리 화면 진입 → 근거 검토 → 기존 승인/실행 결과 확인.
- 보존: Rust/Tauri, 데이터·generation·선택·권한·재검증·휴지통 엔진, 기존 드라이브 교대, dark/native glass와 Pretendard/JetBrains Mono.
- 수정: 전역 메뉴 역할/그룹/활성 상태, 시스템 캐시 직접 동선, 설정6분류, 대시보드 높이/빠른 작업, 지도 빈 상태 설명.
- 제외: Windows 설정 구현, Apple Intelligence/서비스 삭제, 자동 정리, 새로운 OS 변경 권한, 채팅 엔진 교체, 설치/커밋/푸시.
- 성공: 1280×820에서 주요 검사 행동과 빠른 작업 노출, 모든 설정 분류에 접근, 분류 전환으로 입력/비동기 상태를 초기화하지 않음,390px 가로 넘침0·44px 조작·키보드 현재 위치.
- 시각 선호는 기존 사용자가 요청한 Swift 계열 glass/시원한 아이콘을 유지한다. 스타일 재질문 대신 구조와 밀도만 비교한다.

## Execution

Product Design Gate READY: exact selector product-design@openai-curated-remote0.1.56, source Plugin_fa77aec24fc08191bc6e57f377126d76. CLI marketplace/list/list--available와 현재 세션 skill 노출 확인. 설치·설정 변경 없음. 기존 코드에 직접 반영하는 native React 어댑터를 선택한다. Product Design은 앞선 실제 화면 감사에 사용했으며 새 이미지/별도 제품 프로토타입은 필요하지 않다. 두 어댑터 비교는 요청하지 않아 NOT REQUESTED.

Module Coverage: frontend-design, mermaid-diagrams, ui-ux-auditor, web-design-guidelines 모두 카탈로그의 `/Users/dannysmacair/.codex/.olympus/source-skills/<name>/SKILL.md`를 메인이 전체 읽음. Experience/layout/render-critique/design-md, interface playbook·anatomy·blacklist를 함께 적용. 나머지 조건부 모듈은 NOT REQUESTED. 디자인 lint는 headless no-op 한계로 NOT RUN이며 렌더/토큰/접근성 관찰로 한정 검증한다.

#tags: 메뉴개편, 작업중심, 설정분류, 보존경계
