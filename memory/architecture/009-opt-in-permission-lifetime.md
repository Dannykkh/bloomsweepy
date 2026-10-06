# 선택적 권한 수명과 안전한 복원

status: CURRENT
date: 2026-10-05
source: codex
tags: remembered-permissions, scoped-consent, durable-revocation, local-grant-store
evidence: conversations/2026-10-05-remembered-permissions.md#현재-요청 (턴 시각·세션 UUID 미제공)
alternatives: always-remember — 최초 동의 없이 공개 기간을 늘리므로 제외; session-only — 재시작마다 같은 허용을 반복하는 사용자 요구와 충돌; frontend-localStorage-authority — Rust/MCP 권한과 범위를 원자적으로 보장하지 못하므로 제외; remembered-approval — 오래된 삭제 확인으로 실행하게 될 위험 때문에 제외.
depends-on: [[004-app-tool-investigation]], [[008-conversation-workbench]]
sources: docs/architecture/app-capability-contract.md; docs/qa/2026-10-05-remembered-permissions.md
files: apps/desktop/src-tauri/src/permission_settings.rs, apps/desktop/src-tauri/src/control_server.rs, apps/desktop/src-tauri/src/lib.rs, apps/desktop/src/App.tsx, apps/desktop/src/components/ControlStatusPanel.tsx
reopen-when: 여러 사용자/프로필 또는 공급자별 동의, 시간 제한 동의, 이동된 폴더의 자동 재승인 요구가 생기면 재검토한다. 동일 사용자 악성 로컬 프로세스까지 위협 모델에 넣으면 파일 보호/서명을 별도 설계한다.
last_verified: 2026-10-05 (Rust desktop218 PASS/3 ignored 재검증, frontend53 PASS, 합성 Settings/popup 저장 실패·반응형 확인; ARM64 최종 설치형 Remember/Session 완전 종료·재실행·기존 설정 보존 확인. 이후 사용자 선택 조회ON/정리 검토ON도 최종본 재시작 복원)

기본 Session/모든 권한 꺼짐을 유지하면서 main UI에서만 Remember를 고른다. App의 같은
컨트롤을 Settings와 대화 팝업이 사용한다. 유지 방식은 허용을 자동 생성하지 않는다.
Rust 전용 bounded SQLite에 네 종류의 공개 허용과 정확한 폴더/검사 설정만 저장한다.
Plan·삭제 승인·nonce·토큰·작업 상태는 별개이며 복원하거나 자동 실행하지 않는다.

2026-10-05 후속 변경 [[010-selected-folder-inspection]]: 폴더 검사의 별도 허용 스위치는
폐기하고 사용자의 앱 폴더 선택으로 읽기 전용 범위를 연결한다. 나머지 공개 권한의 기본
꺼짐, Session/Remember 수명·복원·영속 철회 계약은 유지한다.

복원은 서버 연결 전에 폴더/정규화 대상/설정 상한을 재검증한다. 새 허용은 디스크 저장이
먼저 성공해야 적용하고, 철회 실패는 runtime을 제한한 뒤 전용 DB 폐기를 시도한다.
폐기 실패는 과거 허용이 돌아올 가능성을 명시하고 Remember/경고를 유지해 재시도한다. Session 전환은 미래의 허용만 지우고
현재 실행의 허용은 유지한다. 창 닫기는 상주 상태일 수 있어 완전 종료와 구분한다.

2026-10-05 [[011-conversational-trash-consent]]는 기본 OFF 내장 채팅 확인 생략 선호를
같은 수명/저장소에 추가한다. 새 인간 요청에 대한 권한이지 개별 과거 계획/승인의 복원이
아니다. MCP/모델에는 실행이나 이 권한 변경을 제공하지 않는다.
