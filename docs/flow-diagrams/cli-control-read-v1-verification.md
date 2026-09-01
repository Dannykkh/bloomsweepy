# 흐름 검증: 채팅 CLI 읽기 연결 v1

- 검증일: 2026-09-01
- 도면: `docs/flow-diagrams/cli-control-read-v1.mmd`
- 판정: FULL MATCH

## 노드 매칭

| 도면 노드 | 코드 위치 | 상태 |
|---|---|---|
| Interpret, Allowlist | `apps/bloomsweepy-mcp/src/mcp.rs:22`의 네 도구와 `apps/bloomsweepy-mcp/src/lib.rs:136` 명령 변환 | 일치 |
| Descriptor, Offline | `crates/bloomsweepy-control/src/lib.rs:470`, `crates/bloomsweepy-control/src/lib.rs:530` | 일치 |
| LocalOnly | `crates/bloomsweepy-control/src/lib.rs:60`의 loopback·규격 검사 | 일치 |
| Authenticate | `apps/desktop/src-tauri/src/control_server.rs:475`의 요청 인증과 고정 시간 토큰 비교 | 일치 |
| Route | `apps/desktop/src-tauri/src/control_server.rs:532` | 일치 |
| Scope, ScopeRequired | `apps/desktop/src-tauri/src/control_server.rs:204`, `apps/desktop/src-tauri/src/control_server.rs:238`, `apps/desktop/src-tauri/src/control_server.rs:572` | 일치 |
| Validate, RejectSearch | `crates/bloomsweepy-control/src/lib.rs:224`, `apps/desktop/src-tauri/src/lib.rs:524` | 일치 |
| Busy, Retry | `apps/desktop/src-tauri/src/control_server.rs:251`, `apps/desktop/src-tauri/src/control_server.rs:582` | 일치 |
| ExistingIndex | `apps/desktop/src-tauri/src/control_server.rs:592`, `apps/desktop/src-tauri/src/control_server.rs:636` | 일치 |
| Deadline, CancelQuery | `apps/desktop/src-tauri/src/control_server.rs:26`, `crates/bloomsweepy-core/src/file_catalog.rs:1319`, `crates/bloomsweepy-core/src/document_search.rs:708` | 일치 |
| ScopeCheck, ScopeChanged | `apps/desktop/src-tauri/src/control_server.rs:596`, `apps/desktop/src-tauri/src/control_server.rs:605`, `apps/desktop/src-tauri/src/control_server.rs:730` | 일치 |
| Assistant | `apps/desktop/src/lib/bridge.ts:41`, `apps/desktop/src/App.tsx:449`, `apps/desktop/src/views/AssistantView.tsx:82`, `apps/desktop/src/components/ControlStatusPanel.tsx:130` | 일치 |
| BoundedResult | `crates/bloomsweepy-control/src/lib.rs:568`, `crates/bloomsweepy-control/src/lib.rs:584` | 일치 |
| Summarize | 저장소 밖의 채팅 CLI·공급자 책임이며 MCP는 구조화 결과만 반환 | 경계 일치 |

## 분기 검증

| 분기 | 성공 경로 | 거부·오류 경로 | 상태 |
|---|---|---|---|
| 허용된 기능 | 네 읽기 도구만 등록 | 그 밖의 도구는 목록에 없음 | 완전 |
| 앱 실행 여부 | 연결 후 요청 | `app_not_running` | 완전 |
| 로컬 주소·규격 | loopback과 v1만 허용 | 잘못된 주소·버전 거부 | 완전 |
| 토큰 | 정확히 일치할 때만 라우팅 | 인증 실패 응답 | 완전 |
| 검색 허용 | `AI 도우미`에서 이번 실행의 파일·문서 목록을 명시적으로 허용 | 기본 거부 `scope_required`, 기준 폴더 변경 시 `scope_changed` | 완전 |
| 검색 조건 | 1~250개와 검색어 검사 | `invalid_request` | 완전 |
| 작업 충돌 | 외부 검색 한 개만 실행 | 두 번째 요청은 `busy` | 완전 |
| 제한 시간·종료 | 8초 안에 결과 반환 | 제한 시간 또는 앱 종료 시 SQLite 진행 콜백이 조회 중단 | 완전 |

## 외부 검증

- Rust 테스트: 기본 검색 권한 꺼짐, 검색 한 개 제한, 지연 전송 소켓, 허용 폴더 밖 경로 거부, SQLite 검색 취소, 단일 앱 잠금 통과.
- 실제 Windows release 앱: 허용 전 `scope_required`, `AI 도우미`의 허용 버튼 후 파일 검색 성공, 앱 재시작 후 권한 자동 해제 확인.
- 실제 Windows 중복 실행: 두 번째 앱이 첫 앱의 연결 파일을 덮어쓰지 않고 첫 앱 CLI 연결 유지 확인.
- 독립 재검토: 최초 네 가지 High 지적을 보강한 뒤 남은 Critical·High 없음 판정.

## 범위 밖 목표

`cli-control-read.mmd`의 새 검사·작업 번호·취소·검사 번호 보관과 `cli-trash-review.mmd`의 확인 계획·최종 확인·휴지통 이동은 v1 도구 목록에 없다. 구현되지 않은 기능을 제공하는 것처럼 표시하지 않으며 다음 단계로 유지한다.
