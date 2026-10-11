# Render critique — Chat access

기존 승인된 compact composer의 국소 변경이라3후보를 생략했다. 다크 실제 렌더:
[desktop](../qa/screenshots/2026-10-11-chat-access-desktop-dark.jpg),
[mobile](../qa/screenshots/2026-10-11-chat-access-mobile-popup-dark.jpg),
[English](../qa/screenshots/2026-10-11-chat-access-mobile-en.jpg).

출력 관찰: 질문 입력이 주 행동이고 권한은 모델 묶음 옆 텍스트/Shield로 구분된다. 선택 표시는
색 외에도 체크/라벨로 식별된다.390px에서 toolbar를 두 행으로 나눠44px 전송과 권한을 보존했다.
popup은 화면 안에 있고 긴 영어 설명은 내부 스크롤로 읽는다. 기존 글래스/색/폰트 토큰을
사용하며 새 이미지·블러·장식 모션을 추가하지 않았다. 구조와 메시지를 보존하는 delta로 판정했다.

검토 후 수정: access 목록에서 아직 열지 않은 결과를 잘못된 용량지도로 연결하는 버튼 제거;
새 모드가 있는데도 반복되던 확인 생략 체크박스 숨김; 기존 selected-only/항상 확인 문구를
현재 모드와 실행 검증 설명으로 수정. 디자인 변형을 늘리지 않고 과업과 신뢰 경계를 보정했다.

키보드 End는 포커스만 이동하고 Enter에서만 모드를 적용한다. Escape는 trigger에 복귀했다.
저장 지연 중 이전 라벨/잠금을 유지하며 실패는 composer 옆 alert로 표시한다.
라이트 테마는 앱 미지원 N/A. reduced-motion CSS 분기 확인, 실제 OS reduce-motion 설정 변경은
NOT RUN. 네이티브 glass 합성은 브라우저 QA로 검증했다고 주장하지 않는다.

#tags: render-critique, chat-access, responsive, accessibility
