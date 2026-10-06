# 공통 파일·MCP·실제 결과 왕복 보완

## Session Metadata

- Created: 2026-10-06 20:01:23 KST
- Project: BroomSweepy
- Branch: main, 기준 HEAD 9e9d775
- Session duration: 시작 시각 미확인; 진행 중 컨텍스트 압축에 따른 자동 인계. 실제 설치 검증 후 갱신.
- Continues from: none — 이번 보완 작업의 첫 핸드오프.

## Origin

| 항목 | 내용 |
|---|---|
| 요구 | 사용자 “보완하자”: 내장 채팅과 외부 MCP의 기능·실제 결과 연결 보완 |
| 출처 | session 01a06a3f-f2f6-72c0-9c0c-78a13e23b651; [이번 대화](../../conversations/2026-10-06-common-mcp-result-contract.md), 사용자 원본 턴 시각 미제공 |
| 해결할 문제 | 기능 목록 전송 누락, 내장 전용 일반 파일 작업, 대기·실패 결과 분석의 단절 |

## Current State Summary

공통 Rust 엔진을 유지한 구현·독립 리뷰·이 맥 개발 설치 완료.
공통 계약/MCP43·native246·frontend96 합계385 PASS(3opt-in ignored).
실제 새 stdio MCP24기능/파일10행동·3파일87B·검토/취소, 실제 native GPT-6.1-Sol/중간
앱 결과 분석 및 같은 결과 용량지도 PASS. 마지막 이벤트 kind 수정도 재설치해 잘못된
검사 실패 알림이 없는 것을 확인했다. 사용자/테스트 파일의 실제 이동은 하지 않았다.
Git 커밋/푸시/공개 릴리스는 이번 요청에 포함되지 않는다.

## Session Memory Review

- Architecture preflight: SKIPPED — 실제 기억 본문 있음, 닥터 차트 2026-09-29로 7일 경과(30일 미도래).
- Anchor index: RAN — 생성기와 파일별 역색인으로 의존 기억 확인. 기존 결정 전체를 대체하지 않음.
- Memory/index updates: [013](../../memory/architecture/013-common-file-mcp-results.md), 004 보완, architecture/index, MEMORY.md.
- Retrieval verification: mcp 인덱스 → 004/013 → 연결 대화와 실제 소스. 본문·링크 있음.
- Observations: 이번 대화와 해당 파일·테스트 범위만 사용. 런타임에서 session ID 확인; 기존 관찰 백로그 정제/offset 변경 없음.
- Skill improvement candidate: defer — 프로젝트 전용 스킬로 확인된 개선 대상이 없어 자동 생성·전역 변경하지 않음.
- Component map: N/A — codemap/component-map.json 없음. 기존 codemap은 보조 탐색 자료.
- Handoff validator: READY92/100, TODO/잠재 민감값0. 현재 session 관찰 로그가 없어 날짜 전체로 검사한 기존012 태그 경고는 별도 작업 기록이므로 이번 작업에서 수정하지 않았다.

## Feature/Flow/Decision Snapshot

### Implemented Features

| Feature/Change | Entry Point | Implementation Anchors | Verification |
|---|---|---|---|
| 전체 discovery + ID별 상세 | app_capabilities/app_action | control capabilities, app_tools, mcp | 계약/MCP43 PASS; 새 stdio24ID/10행동,15707B/11012B |
| 일반 파일 10행동 공통화 | 내장 JSON / 외부 MCP file_workspace | assistant_files, control_server | native246 PASS; 실제 검사3파일87B·검토/취소 |
| 실제 terminal 결과 분석 전용 왕복 | ask_assistant | assistant_provider, assistant_tools | scripted terminal5상태 PASS; 실제 Codex 조회/분석 PASS |
| 외부 준비 파일 검토 카드 | app-tool-review / main IPC | AssistantAppToolCard, externalFileReview, bridge | 프런트96 PASS; 렌더/최소760×600/실제 아니오/오래된 모달 취소 PASS |
| 완료 이벤트 작업 구분 | control-scan-completed | control_server | 6종류×3상태 회귀 및 최종 설치 MCP 실패 알림0 |

### Feature Boundary

같은 typed 요청·공유 dispatcher/worker 코어를 사용하되 전송 규약은 분리한다.
내장은 JSON envelope/Tauri, 외부는 MCP stdio → 로컬 control bridge.
외부는 앱이 승인한 단일 루트·현재 권한 epoch에 묶이고 native 세션과 별도 작업 공간을 사용한다.
모델/MCP가 경로·권한·승인·실행을 직접 지정하지 못하며 외부 최종 이동은 main 앱의 일회용 확인만 가능하다.
준비·시작을 완료/삭제로 표현하지 않는다. 원본 파일 내용·로컬 plan ID·경로 presentation은 모델 결과에서 제외한다.

### Menu / Screen Map

기존 AI 도우미/외부 검토 팝업을 재사용한다. 새 화면·색상·레이아웃 재디자인 없음.

### Composition Diagram

```mermaid
flowchart LR
    Native[내장 채팅 JSON] --> Dispatcher[공통 app_tools]
    MCP[외부 MCP stdio] --> Control[control bridge]
    Control --> Dispatcher
    Dispatcher --> Engine[공유 파일 worker]
    Engine --> Model[상한 있는 실제 결과]
    Engine --> UI[로컬 전용 검토 presentation]
```

### Flow Diagram

[정본 흐름도](../flow-diagrams/app-tool-investigation.mmd): 입력 → typed 검증/권한 → 앱 엔진 → bounded 실제 결과 → 분석 또는 상태 설명.
외부 비동기 작업은 operationId로 이어지고 준비된 검토만 사용자 확인으로 연결된다.

### Decision Records

013은 004의 공통 기능 정본과 실제 결과 조사 모델을 구체화한다. 전체 MCP wire를 내장 UI에 강제하거나
전송 상한을 제거하거나 외부에 execute를 여는 대안은 채택하지 않았다. 대체 대상 none; 기존 004 CURRENT 유지.
지도 스냅샷은 기존003의 저메모리 단일 generation 계약을 유지한다. 다른 내장·외부 측정
검사가 이전 지도 generation을 교체하면 재검사가 필요하며 선택·계획을 공유하는 것은 아니다.

## Codebase Understanding

정본은 docs/architecture/app-capability-contract.md. 주요 조합점 app_tools::execute,
공유 파일 코어 assistant_files::apply_workspace_action, 외부 권한 control_server::FileWorkspaceBinding.
결과 data는 모델용, presentation은 로컬 UI용이다.

## Work Completed

소스 계약/엔진/UI/문서 및 회귀·실제 설치 왕복을 완료했다. 파일 소유권은 control/MCP, 파일backend, UI 세 작업자로 분리했다.
주요 변경: capabilities.rs/lib.rs, mcp.rs, app_tools.rs, assistant_provider.rs, assistant_tools.rs,
assistant_files.rs/control_server.rs, desktop lib.rs, AssistantAppToolCard.tsx/bridge.ts,
externalFileReview.ts와 테스트/fixture, docs 계약·흐름도·QA, 기억013·004·인덱스·현재 대화.
추가 설치형 stdio harness는 apps/bloomsweepy-mcp/tests/installed-common-flow.mjs다.
Clippy는 기존 items_after_test_module 경고만 제외한 -D warnings PASS, fmt·diff check PASS.

## Pending Work

### Immediate Next Steps

1. 현재 보완 요청은 완료. 기존 외부 클라이언트는 새 MCP 연결로 다시 시작해야 새 DTO/기능을 사용할 수 있다.
2. 사용자가 요청하면 이번 변경만 구분해 커밋/푸시/릴리스한다. 기존 unrelated dirty 파일을 섞지 않는다.
3. Windows·장시간 메모리·실제 파일 이동은 각각 별도 환경/승인과 검증 범위로 진행한다.

### Blockers/Open Questions

현재 구현·설치 검증의 차단 없음. 새 범위의 삭제 승인은 없다; 실제 삭제 없이 연결/검토/취소까지 검증했다.

### Deferred Items

개인 자료 삭제, 휴지통 비우기, 앱 제거, Windows, 장시간 메모리 검증은 이번 통합 검증 범위 밖.

## Context for Resuming Agent

### Important Context

- 기존 dirty QA·기억011/012·설치 기록과 untracked demo-assets/를 보존한다. 이번 요청은 commit/push/release가 아님.
- fixture3파일87B는 보존. direct-auto.txt29B 실제 이동 승인은 아직 없다; 보완 요청을 삭제 승인으로 해석하지 않는다.
- 설치 앱은 이번 보완 개발본(표시1.7.0); 공개 GitHub1.7.0과 같지 않다. 기존 MCP sidecar는 새 DTO를 모를 수 있어 새 stdio로 검증했고 기존 사용자 연결은 강제 종료하지 않았다.
- 원래 앱은 /private/tmp/broomsweepy-common-mcp-backup-TsW1zC/BroomSweepy.app에 보존했다. 최종 host SHA는 QA에 기록했다. 사용자 데이터 DB/설정/대화는 보존했다.
- 8GiB ARM Mac, 여유 디스크 약2.8GiB. Cargo 단일 job/incremental off, 네이티브 빌드 중복 실행 금지.

### Assumptions Made

내장/외부 기능 일치와 응답 연속성을 보완하되, 프로토콜 전체 교체·새 UI 디자인·외부 execute는 요구 범위가 아니다.

### Potential Gotchas

- 외부 reserved key는 external-files.workspace; root/config/승인 epoch와 cleanup epoch 분리. status.revision은 권한 epoch가 아니다.
- 최종 상태 설명은 action:null만 허용, 추가 dispatch 금지. 모델 실패 후 실제 근거·검토를 버리거나 자동 재실행하지 않는다.
- 기존 local CLI fake tests는 실제 LLM 성공으로 보고하지 않는다.

## Environment State

### Tools/Services Used

Cargo native lib246/계약·MCP43/frontend96 PASS. ARM .app 및 마지막 host 빌드·설치·strict서명 PASS. 독립 읽기 전용 리뷰 완료.

### Active Processes

최종 /Applications/BroomSweepy.app 실행 중, Codex 준비됨·GPT-6.1-Sol/중간·저장 대화 복원.
기존 MCP 사용자 연결 프로세스는 보존한다. 이번 Vite 서버(session61234)는 정상 중단했고
임시 browser fixture(tab18)는 닫았다. 원래 설치 백업과 테스트 파일3개는 복원/검증 근거로 보존한다.

### Environment Variables

이름만 기록: CARGO_BUILD_JOBS, CARGO_INCREMENTAL. 민감값·credential receipt는 출력하지 않는다.

## Related Resources

- [공통 기능 계약](../architecture/app-capability-contract.md)
- [QA 진행 기록](../qa/2026-10-06-common-mcp-results.md)
- [기억013](../../memory/architecture/013-common-file-mcp-results.md)
- [데스크톱 사용 설명](../../apps/desktop/README.md)

#tags: 공통계약, mcp, 파일관리, 결과왕복, 핸드오프, arch:003, arch:004, arch:013
