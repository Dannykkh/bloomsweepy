# 내장 JSON과 외부 MCP의 공통 파일·결과 계약

status: CURRENT
date: 2026-10-06
source: codex
tags: shared-file-tools, mcp-discovery, typed-json, result-feedback, scope-epoch
evidence: conversations/2026-10-06-common-mcp-result-contract.md#현재-요청 (사용자 원본 시각 미제공; 19:53 KST 기록)
alternatives: 내장까지MCP wire 강제 — 같은 프로세스의 앱 엔진 호출에 불필요한 서버/연결 수명 추가; 전송 상한 해제 — 저자원·토큰 상한 계약 위반; 내부 saved session을 외부와 공유 — 선택·계획·현재 폴더 혼선; 외부 직접 execute/자동 승인 — 기존 인간 실행 경계 위반; elapsed-time TTL 추가 — 늦은 확인을 허용하는 무시간 일회용 계약과 충돌.
depends-on: [[004-app-tool-investigation]], [[009-opt-in-permission-lifetime]], [[010-selected-folder-inspection]], [[011-conversational-trash-consent]]
sources: docs/architecture/app-capability-contract.md; docs/flow-diagrams/app-tool-investigation.mmd
files: crates/bloomsweepy-control/src/capabilities.rs; apps/desktop/src-tauri/src/app_tools.rs; apps/desktop/src-tauri/src/assistant_files.rs; apps/desktop/src-tauri/src/assistant_provider.rs; apps/desktop/src-tauri/src/control_server.rs; apps/bloomsweepy-mcp/src/mcp.rs; apps/desktop/src/components/AssistantAppToolCard.tsx
reopen-when: 외부 공급자별 독립 workspace/권한 요구, 파일 작업 종류 추가, 실제 상한 초과나 결과 상태 왕복 실패가 발생하면 재검토한다.
last_verified: 2026-10-06 (새 설치 stdio12도구/24기능/파일10행동·3파일87B·검토/취소; 실제 native GPT-6.1-Sol/중간 결과 분석·같은 결과 지도; 최종 완료 알림 수정 재설치/오분류 제거·설정/대화 복원 실측)

채팅 UI/JSON과 MCP는 각각 입구이고 실제 기능과 결과의 정본은 Rust 앱이다.
기존 내장 files envelope는 typed FileWorkspace 요청의 호환 alias로 같은 dispatcher를
사용한다. 외부는 앱이 선택한 정확한 루트에 갇힌 독립 workspace다. 임의 path나 내부
session 선택은 노출하지 않으며 root/config·정리 허용의 epoch 변경은 예전 조회/계획을
무효화한다. 최종 계획 ID와 전체 경로는 로컬 presentation에만 둔다.

16KiB를 늘리지 않고 모든 기능 ID/작동 예제를 포함하는 discovery index와 ID별 상세를
정본에서 파생한다. 파일 작업은 외부에서 작업 번호로 시작/완료를 구분하고 상태 조회가
실제 목록·검토·취소/부분 실행 결과를 반환한다. 실행은 main 앱의 일회용 확인만 가능하다.

완료 조회는 기존 조사 루프에 피드백하고, 대기/권한/실패는 추가 행동 없는 분석 전용
라운드 한 번으로 설명한다. 모델 분석 오류가 실제 앱 결과나 준비된 검토를 버리거나
작업을 재실행하게 하지 않는다. 4행동·5모델호출·600초 모델 조사 예산·누적48KiB 상한은 유지한다.
새 행동 전에도 deadline을 확인하되 진행 중인 앱 파일 검사는 협력 취소이며 강제600초 종료 보장은 아니다.

내장의 terminal 분석은 새 행동/자동 polling 없이 끝낸다. 외부 running은 반환된 정확한
operationId만 간격을 두고 최대4회 관찰하는 모델 계약이며 완료 후 workspace/status로
실제 목록을 받는다. 서버 rate-limit을 새로 구현한 것은 아니다.

외부 검토창 취소는 현재 정확한 계획만 변경한다. 교체·철회·소비된 창의 유효 형식 ID는
상태 불변 닫기이므로 계획 변경 횟수/시간과 무관하게 UI를 닫되 새 계획과 결과는 보존한다.
별도의 무한 nonce 이력/TTL을 두지 않으며 실행 claim의 현재 권한·신원·일회용 검증은 유지한다.
공유 파일 I/O worker가 완료 lease를 보유하여 요청 future가 사라져도 실제 작업 중복을 막는다.

완료 이벤트는 실제 operation.kind를 보존한다. 파일 workspace·앱/문서 작업의 종료가
storageScan 완료/실패로 해석되지 않아야 한다. 실측에서 발견된 오분류와 회귀·설치 범위는
docs/qa/2026-10-06-common-mcp-results.md에 기록한다.
