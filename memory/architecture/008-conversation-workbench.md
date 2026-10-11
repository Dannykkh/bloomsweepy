# 대화 중심 작업면과 실제 진행 단계

status: CURRENT
date: 2026-10-05
source: codex
tags: chat-workbench, docked-composer, actual-progress, evidence-timeline, explicit-retry
evidence: conversations/2026-10-05-chat-workbench.md#현재-요청 (턴 시각·세션 UUID 미제공)
alternatives: sole-main-scroller — 긴 결과가 입력창을 화면 밖으로 밀어 새 요구와 충돌; sticky overlay — 과거 제목 겹침 문제를 재현하므로 제외; 가짜 스트리밍/완료율 — 현재 CLI는 최종 응답을 반환하므로 사실이 아닌 표시; 조회 목록 전체 기본 펼침 — 대화 읽기를 막고 불필요 DOM을 늘리므로 제외.
depends-on: [[004-app-tool-investigation]], [[007-native-window-material]]
sources: DESIGN.md; docs/design-refs/2026-10-05-experience-chat-workbench.md; docs/qa/2026-10-05-chat-workbench.md
files: apps/desktop/src/components/AppShell.tsx; apps/desktop/src/views/AssistantView.tsx; apps/desktop/src/views/AssistantView.css; apps/desktop/src/views/SettingsView.tsx; apps/desktop/src-tauri/src/assistant_provider.rs
supersedes: docs/design-refs/2026-09-06-experience-chat-header.md의 sole-main-scroller 정책만 대체; native chrome과 앱 승인 정책은 유지
reopen-when: Provider가 실제 streaming을 제공하거나 백그라운드 대화 지속을 설계할 때. 작은 창에서 입력/검토가 숨는 실측이 나오거나 권한이 추가되면 공유 Settings/Dialog 계약을 재점검한다.
last_verified: 2026-10-05 (frontend53, Rust provider25/2 ignored, production build, component 합성 render, ARM64 설치형 복원/입력/Settings/dialog/Escape focus 검증; 실제 provider 새 요청·Windows·장시간 검증은 별도)

AppShell은 assistant만 높이를 제한하고 header/scope/transcript/dock을 겹치지 않는 행으로
만든다. 내용·provider identity는 보존하고 긴 읽기 결과는 lazy disclosure, 최종 검토와
실패는 기본 펼침이다. Settings와 대상 옆 dialog는 App의 같은 ControlStatusPanel props를
소비하며 허용과 최종 실행의 권한은 바꾸지 않는다. macOS native glass/chrome은007을 유지한다.

`assistant-progress`는 실제 경계에서 preparing/analyzing/querying을 emit하고 진행 nonce와
session ID로 stale/null/잘못된 payload를 제외한다. 질문·path·본문은 포함하지 않는다.
UI는 경과 시간만 계측하며 token/완료율을 추측하지 않는다. 종료/오류/unmount에서 listen을
해제한다. 사용자가 끝에서 벗어나 읽고 있으면 위치를 유지하고 최신 버튼으로 돌아올 수 있다.

2026-10-11 보완: 조회 근거는 안정적인 assistant turn key에 연결한다. 완료·취소한 검토는
조작 UI를 제거하고 실제 처리 요약만 남긴다. 과거 결과는 읽기 전용이며 새 질문은 이전
요청의 승인·취소가 아니다. 현재 native workspace/권한과 화면의 역사적 근거를 분리한다.
화면160턴·상세 근거16개·live 결과16개 상한을 두며 오래된 상세는 제목/상태/작은 요약으로
축소한다. 전체 카드 payload나 계획 토큰을 메시지 DB에 저장하지 않는다.
저장된 질문의 실패도 BroomSweepy 메시지로 남기고 원래 질문·동일 세션·권한/workspace revision의
명시적 재시도만 제공한다. 수동 검사·검색·드라이브 탐색은 이전 재시도를 무효화한다.
질문 중복 저장·취소 후 자동 재시도·과거 승인 재활성은 없다.
근거: conversations/2026-10-11-codex.md의 응답 불가/카드 수명 사용자 요청(원본 시각·UUID 미확인),
docs/qa/2026-10-11-chat-response-timeline.md. 대안: send 시 모든 카드를 삭제하면 미확인 검토도
사라지고, bottom 별도 렌더는 새 질문 뒤에 과거 결과가 남으므로 제외했다. 세션 재시작 후
카드 전체 복원은 권한·메모리 설계를 별도로 확정할 때만 재검토한다.

보완 검증: 2026-10-11 frontend132 PASS, 실제 component fixture의 중복 없는 retry·수동 검사 후
retry 비활성·turn별 카드·이전 검토 비실행·취소 완료·1280/390px 입력창 노출 확인.
