# 내장 채팅 권한 모드와 유한 로컬 볼륨 접근

status: CURRENT
date: 2026-10-11
source: codex
tags: chat-access, native-authority, bounded-volume-search, epoch-revocation
evidence: conversations/2026-10-11-chat-access.md#사용자-요구--턴-시각-미확인; conversations/2026-10-11-chat-access.md#1049-kst--개발-결정과-경계; conversations/2026-10-11-chat-access.md#1122-kst--사용자-설치-요청과-실제-설치-확인
alternatives: frontend-only-mode — direct IPC와 늦은 응답을 막지 못해 제외; old-flag-to-full-access — 기존 확인 생략 동의를 전체 볼륨 조회로 확대하므로 제외; global-MCP-grant — 외부 선택 폴더 계약을 바꾸므로 제외; unlimited-recursive-search — 8GiB/저공간 Mac의 자원 상한과 충돌해 제외.
depends-on: [[009-opt-in-permission-lifetime]], [[010-selected-folder-inspection]], [[011-conversational-trash-consent]], [[013-common-file-mcp-results]]
sources: docs/architecture/app-capability-contract.md; docs/design-refs/2026-10-11-experience-chat-access.md; docs/qa/2026-10-11-chat-access.md; docs/qa/2026-10-11-local-install.md
files: apps/desktop/src-tauri/src/assistant_access.rs, apps/desktop/src-tauri/src/control_server.rs, apps/desktop/src-tauri/src/permission_settings.rs, apps/desktop/src-tauri/src/assistant_provider.rs, apps/desktop/src-tauri/src/assistant_files.rs, apps/desktop/src-tauri/src/trash_actions.rs, crates/bloomsweepy-core/src/local_search.rs, apps/desktop/src/views/AssistantView.tsx, apps/desktop/src/views/AssistantView.css, apps/desktop/src/lib/assistantPermissionMode.ts
reopen-when: 외부 MCP 전체 볼륨 허용, 클라우드/네트워크 검사, 범주 단위 자동 삭제, 여러 사용자/동시 채팅 또는 OS 접근권한 자동 부여를 요구하면 재검토한다.
last_verified: 2026-10-11 — frontend126/TypeScript/production frontend build PASS, desktop286/ignored4·core108·control29 PASS(최후 ID 보강 후 focused2 PASS). 합성4언어·390/1280px·실패/지연·keyboard·전체 검색/open·readonly 확인。이 Mac ARM64 release 설치/서명/현재 confirm·remember/대화 복원·두 popup 실제 표시/Escape PASS。새 mode 변경·실제 CLI 종단・IPC 경쟁상태/실볼륨/Windows 런타임 NOT RUN.

기본 confirm, 명시적 read_only/full_access를 main-window IPC와 Rust 저장소가 관리한다.
009의 Session/Remember를 재사용하고 모델/요청 JSON은 모드를 바꾸지 못한다. 예전 저장소에
필드가 없으면 confirm이며 old flag로 전체 접근을 이관하지 않는다. 새 모드가 있는 host의
confirm은 항상 확인한다.011의 단순 직접 명령/단일 완전 계획/실행 재검증은 유지하고,
그 확인 생략 opt-in의 현재 설정 표현만 full_access로 통합한다.

전체 볼륨 기능은 native 전용4operation. 저장된 폴더 세션은 anchor, 현재 작업 root는
세션별 별도 상태다. opaque revision/ID로만 열고 full scope에서 네트워크·클라우드·중첩
마운트를 건너지 않는다.16볼륨·250k 방문·200결과·경로2MiB·30초의 협력적 상한과24행
결과를 사용한다. partial/동명이인/automaticTrashEligible:false는 자동 이동을 막는다.
010의 외부 선택 폴더 계약은 바꾸지 않는다. 문서 본문 공개·OS 접근·영구 삭제도 별도다.

모드 변경은 native epoch와 취소 토큰으로 늦은 결과/검토/실행을 무효화한다. JS도 revision을
검사하지만 Rust가 정본이다. 공통 계획 ID/preview와 당시 epoch를 결속하고 메모리 검토의
optional epoch도 확인한다. 일반 수동 앱 기능과 외부 grant는 자동 확대/축소하지 않는다.
Product Design0.1.56은 설치·활성 확인했으나 현재 세션 미노출이라 local React/CSS로 구현했다.

이 Mac native smoke에서 AX만으로 popup 표시 성공을 판단하면 놓치는 회귀를 발견했다.
composited transcript 위로 dock/popup을 올리는 명시적 z-index1 후 실제 모델·권한 메뉴 표시가
복원됐다. 현재값을 변경하지 않은 재시작 확인이며 새 full_access opt-in/실행 성공과 구분한다.
