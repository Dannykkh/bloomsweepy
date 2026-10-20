# Windows 저장값 조회와 로컬 공식 설정 연결

status: CURRENT
date: 2026-10-20
source: codex
tags: windows-management, stored-values, fixed-settings-targets, platform-gating, mac-regression
evidence: conversations/2026-10-20-windows-management.md#사용자-요청 (기록23:42 KST; 원래 사용자 턴 시각 미제공)
alternatives: reuse-finder-menu — Windows의 의미·속성이 다름; registry-toggle-from-dword — 저장값/지원/실효 상태와 정책 부작용을 혼동; upstream-admin-script — 서비스/패키지/보호 변경과 복원 불확실성이 현재 범위 밖. Windows 실기 원복·지원 검증을 별도 설계하면 제한된 쓰기를 재검토한다.
depends-on: [[010-selected-folder-inspection]], [[016-macos-management]], [[017-workspace-navigation]]
sources: docs/plan/windows-management/implementation-contract.md; docs/qa/windows-management/verify-report.md; https://learn.microsoft.com/en-us/windows/apps/develop/launch/launch-settings; https://learn.microsoft.com/en-us/windows/client-management/mdm/policy-csp-windowsai
files: apps/desktop/src-tauri/src/windows_settings.rs, apps/desktop/src-tauri/src/windows_settings/model.rs, apps/desktop/src-tauri/src/windows_settings/native.rs, apps/desktop/src/views/WindowsManagementView.tsx, apps/desktop/src/lib/windowsSettings.ts, apps/desktop/src-tauri/src/app_tools.rs, crates/bloomsweepy-control/src/capabilities.rs
reopen-when: Windows 실기에서 일반 사용자·관리형 PC의 실제 조회/공식 페이지 결과가 확보되거나 Microsoft 정책 경로/지원 조건이 바뀌거나 OS 쓰기를 별도 요청할 때.
last_verified: 2026-10-21 (20일 프론트170/170·type/build·Rust workspace490PASS/6ignored·Mac Finder read1/1·부정IPC15/15·GNU cross-typecheck·합성 화면에 더해, 새 Mac ARM64 설치본의 실제 stdio12도구/27capabilities·Finder5행 bool/null/state·Windows unsupported0행·무변경 PASS, 실제 Codex Finder5행 조회 PASS. Windows runtime/installer는 여전히 NOT RUN).

컴퓨터 그룹 네 번째는 OS별 Mac/Windows 설정이다. Windows는 Explorer4행·AI정책6행의 고정 registry64bit KEY_READ 관찰만 제공한다. DWORD0/부재/거부/타입/미지원값/오류를 유지하며 실제 기능의 ON/OFF·설치·사용자 동의·지원 여부로 합성하지 않는다. Recall HKCU/HKLM을 독립 표시하고 legacy Copilot은 폐기로 표시한다.

로컬 main origin만8개의 고정 공식 설정 target ID로 요청한다. 임의 URI/경로/인수/관리자 상승·registry쓰기는 없다. requested:true/changed:false는 실제 창 표시·변경 완료가 아니다. NonWindows는 reader/launcher를 호출하지 않고 빈행/targets unsupported다.

`windows.inspect`는 동일 native reader→bounded10행·16KiB projection→채팅 카드/화면 연결이다. 외부 inspection 공개권한을 전후 재검증하고 원본 diagnostic/registry경로/URI는 모델에 보내지 않는다. AI/MCP opener·쓰기·복원은 없다. Mac Finder와 durable journal·파일 full_access 경계는 별개다.

UI는 공식 연결8→Explorer4→기본 접힌 정책6→공통 관리 연결이다. single-flight·StrictMode 읽기 재사용·unmount/generation 무효화를 유지한다. 첫 실패는 값이 없다고 알리고, 갱신 실패만 실제 이전 snapshot을 stale로 남긴다. 교차 타입검사/합성 화면과 Windows 실기/설치 검증을 구분한다.

새 Mac 설치와 실제 채팅 검증은 [설치형 검사](../../docs/qa/2026-10-21-installed-chat-audit.md)에 연결한다. 파일 조회·동일 지도·성능/앱 조회·자체 메모리 실행·승인 파일1개 휴지통 이동은 통과했지만, 문서 본문은 별도 root 공개권한으로 차단됐고 성공 경로는 미검증이다. 이전 파일 결과의 턴 출처 소실·문서 공개 동선 P2는 제품 수정 전 상태다. 이 Mac 결과를 Windows 실기 PASS로 승격하지 않는다.
