# 앱 기능 정본과 LLM의 사용자 대행 조사

status: CURRENT
date: 2026-10-04
source: codex
tags: app-capabilities, app-owned-results, llm-investigation, consent, bounded-loop
evidence: conversations/2026-10-04-team-daedalus-app-tools.md#현재-요청 (사용자 턴 시각·세션 UUID 미확인)
alternatives: 프롬프트 설명만 보강 — 실제 기능 연결과 후속 분석을 해결하지 못해 제외; CLI 자체 파일/셸 조사 — 앱 소유 목록·scope·승인 규칙과 어긋나 제외; 전체 UI payload 전송 — 경로/승인 ID 불필요 공개 때문에 제외.
depends-on: [[001-conversational-file-workspace]], [[003-conversational-storage-map]]
sources: docs/architecture/app-capability-contract.md; docs/plan/app-tool-integration/plan.md; docs/qa/2026-10-04-app-tool-integration.md
files: crates/bloomsweepy-control/src/capabilities.rs; apps/desktop/src-tauri/src/app_tools.rs; apps/desktop/src-tauri/src/assistant_provider.rs; apps/desktop/src-tauri/src/app_tools_system.rs; apps/desktop/src-tauri/src/app_tools_search.rs; apps/bloomsweepy-mcp/src/mcp.rs
reopen-when: 새로운 앱 기능/권한을 추가하거나 실제 조사에서4행동·결과 상한 때문에 과업 완료가 자주 막힐 때. CLI가 app protocol 밖에서 자체 조사한 실측이 나오면 provider 격리를 재검토한다.
last_verified: 2026-10-04 (control23/MCP12/desktop191·frontend43 검사 및 frontend 빌드, 설치 조사 흐름은 미검증)

사용자 개념은 LLM이 사람 대신 앱 조회를 지시하고 실제 결과를 검토·분석·추가 조회하는 것이다. 앱은 데이터를 수집·검색·측정하고 실제 실행을 검증한다. 이는 단일 intent parser나 static 요약기와 다르다.

typed capabilities catalog를 native/MCP가 직접 소비하고 같은 dispatcher의 기존 서비스들을 호출한다. model-safe data와 local-only presentation을 분리한다. 문서 본문 일부 공개는 별도 문서 동의, 외부 시스템 조회는 closed-default 동의, 전역 시스템정리는 native/external 모두 기존 cleanup 허용을 유지한다. 최종 삭제/앱 제거/정상 종료/영구 정리는 모델 요청과 분리한다.

기존 파일·빈 폴더·용량지도 계약을 유지하면서 최대4행동·48KiB 누적 실제 결과를 모델 다음 입력에 전달한다. 승인/실행 중/권한/미지원은 중단 상태다. 미측정은0이 아니며 부분 목록은 전체 증거가 아니다. 추가 모델 라운드는 토큰을 소비하고 절감률은 미측정이다.

2026-10-06 보완 [[013-common-file-mcp-results]]: 중단은 추가 행동을 멈춘다는 뜻이다.
실제 대기/실패 상태 뒤에는 전체 상한 안에서 한 번의 분석 전용 응답을 허용한다.
내장 파일 alias/외부 MCP도 공통 파일 요청을 사용하고 기능 발견은 index/상세로 나눈다.
기존 final-execution/범위/공개 경계는 유지하며 새 검증 결과는013과 해당 QA를 따른다.

공통 dispatcher·4행동 조사 루프·결과/준비된 검토 UI는 개발본에 연결했다. Native/External 및 세션별 작업 소유권, 동의 철회 후 결과 비공개, 변경 요청128개 재전송 보호를 둔다. workspace lib307·frontend43 검사와 frontend 빌드는 통과했다. 디스크 최소 여유 guard로 첫 회귀가 실패했으나 자체 단일 빌드 산출물 정리 후 lib307과 합성 반복 자원 검사1개가 통과했다. 전체 integration/설치된 앱의 실제 Codex 흐름/Windows/장시간 메모리 검증은 별도다. QA의 최신 결과를 따르고, 설치본까지 완료했다고 읽지 않는다.
