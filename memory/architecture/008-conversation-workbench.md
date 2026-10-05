# 대화 중심 작업면과 실제 진행 단계

status: CURRENT
date: 2026-10-05
source: codex
tags: chat-workbench, docked-composer, actual-progress, evidence-disclosure, settings-permissions
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
