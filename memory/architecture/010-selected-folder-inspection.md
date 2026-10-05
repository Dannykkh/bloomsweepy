# 폴더 선택을 읽기 전용 검사 범위로 사용

status: CURRENT
date: 2026-10-05
source: codex
tags: selected-folder, read-only-inspection, scoped-mcp, no-scan-toggle
evidence: conversations/2026-10-05-selected-folder-inspection.md#현재-요청 (원본 턴 시각 미제공)
alternatives: manual-scan-toggle — 사용자가 선택한 폴더에 중복 허용 관문을 만들고 폴더 미선택 시 설정에서 해결할 수 없어 폐기; arbitrary-external-path — 앱이 정한 범위를 건너뛰고 임의 파일시스템을 노출하므로 제외; hide-toggle-only — 외부 검사 시작 경로가 계속 허용 요청에서 막히므로 제외.
depends-on: [[004-app-tool-investigation]], [[009-opt-in-permission-lifetime]]
supersedes: #manual-scan-toggle (009의 수명·저장 계약은 유지)
sources: docs/architecture/app-capability-contract.md, docs/qa/2026-10-05-selected-folder-inspection.md, DESIGN.md
files: apps/desktop/src/lib/selectedScanScope.ts, apps/desktop/src/App.tsx, apps/desktop/src/components/ControlStatusPanel.tsx, apps/desktop/src-tauri/src/control_server.rs, crates/bloomsweepy-control/src/capabilities.rs
reopen-when: 외부 공급자별 검사 범위나 별도 스캔 대상 프로필 요구가 생기면 재검토한다. 임의 경로 입력이나 자동 디스크 검사를 추가하면 이 계약을 재검증한다.
last_verified: 2026-10-05 (frontend60, desktop219/ignored3, control23, MCP12 PASS; Settings·popup 합성 렌더와 ARM64 교체 설치본 Settings 확인·기존 권한 유지)

사용자가 앱에서 폴더를 선택하면 기존 main-window IPC로 그 root·현재 ScanConfig를
연결한다. 모델/MCP는 범위를 바꿀 수 없다. 기존 범위 철회 실패 시 폴더 변경을
중단하고, 철회 후 새 연결 실패는 표시하되 로컬 읽기 검사를 막지 않는다.
Session/Remember 재검증·저장 계약은 그대로 쓰고 삭제 승인은 생성하거나 복원하지 않는다.

기존 대화를 여는 것만으로 전역 외부 범위를 새로 공개하지 않는다. 내장 대화 검사는
저장된 세션의 선택 root 안에서 이미 가능하다. CLI 자체 도구 차단 여부는 별도 이슈다.
