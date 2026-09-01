# 채팅 CLI 연결

BroomSweepy의 제어 도구는 파일을 직접 검사하거나 삭제하는 독립 프로그램이 아닙니다. Codex, Claude Code, Gemini CLI의 요청을 현재 로그인 사용자가 실행 중인 BroomSweepy 앱으로 전달하고, 앱이 만든 결과만 돌려줍니다.

## 현재 제공 기능

| 도구 | 실제 처리 주체 | 설명 |
|---|---|---|
| `status` | BroomSweepy 앱 | 앱 연결과 현재 상태 확인 |
| `system_overview` | BroomSweepy 앱 | 드라이브 용량 요약 확인 |
| `search_files` | BroomSweepy 앱 | 앱이 미리 만든 파일 목록 검색 |
| `search_documents` | BroomSweepy 앱 | 앱이 미리 읽어 둔 문서 내용 검색 |
| `start_storage_scan` | BroomSweepy 앱 | `AI 도우미`에서 이번 실행에 허용한 폴더 검사 시작 |
| `operation_status` | BroomSweepy 앱 | 작업 번호로 진행 상태와 제한된 결과 요약 확인 |
| `cancel_operation` | BroomSweepy 앱 | 정확히 일치하는 작업 번호의 검사 취소 요청 |

정리 후보 확인과 휴지통 이동은 아직 채팅 CLI에 공개하지 않았습니다. 앱이 꺼져 있으면 제어 도구가 분석 엔진을 대신 실행하지 않고 `app_not_running`을 반환합니다.

새 검사는 파일·문서 검색 허용과 별개입니다. 먼저 `용량 관리`에서 검사할 폴더를 선택한 뒤 `AI 도우미`에서 `이번 실행에서 폴더 검사 허용`을 눌러야 합니다. CLI의 시작 명령에는 경로나 설정 인수가 없으며, 앱이 보관한 허용 폴더와 당시 설정만 사용합니다. 폴더나 설정을 바꾸거나 앱을 다시 켜면 검사 허용은 자동으로 꺼집니다.

## 빌드와 직접 확인

현재 개발 버전은 설치 프로그램에 제어 도구를 포함하지 않습니다. 저장소 루트에서 먼저 빌드합니다.

```powershell
cargo build --release -p bloomsweepy-mcp
```

BroomSweepy 앱을 실행한 뒤 다음처럼 직접 확인할 수 있습니다.

```powershell
target\release\bloomsweepy-mcp.exe status
target\release\bloomsweepy-mcp.exe system-overview
target\release\bloomsweepy-mcp.exe search-files "보고서" --max-results 20
target\release\bloomsweepy-mcp.exe search-documents "계약 기간" --max-results 20
target\release\bloomsweepy-mcp.exe start-scan
target\release\bloomsweepy-mcp.exe operation-status <작업번호>
target\release\bloomsweepy-mcp.exe cancel-operation <작업번호>
```

파일과 문서 검색은 앱 화면에서 해당 검색 목록을 만든 뒤, `AI 도우미`의 `이번 실행에서 검색 허용`을 눌러야 사용할 수 있습니다. 허용은 현재 앱 실행에만 적용되며 앱을 다시 켜면 자동으로 꺼집니다. 파일 목록과 문서 목록은 각각 만들어진 경우에만 허용됩니다.

## MCP 등록

아래 `<repo>`는 이 저장소의 절대 경로로 바꿉니다. Windows 예시는 빌드한 실행 파일을 직접 등록합니다.

```powershell
codex mcp add bloomsweepy -- "<repo>\target\release\bloomsweepy-mcp.exe" mcp
claude mcp add --scope user bloomsweepy -- "<repo>\target\release\bloomsweepy-mcp.exe" mcp
gemini mcp add --scope user bloomsweepy "<repo>\target\release\bloomsweepy-mcp.exe" mcp
```

연결 뒤 공급자에게 “BroomSweepy 상태를 확인해 줘”, “앱에서 허용한 폴더 검사를 시작하고 끝날 때까지 확인해 줘”, “이미 만든 파일 목록에서 보고서를 찾아 줘”처럼 요청할 수 있습니다.

검사 시작은 완료를 기다리지 않고 작업 번호를 바로 돌려줍니다. 공급자는 그 번호로 상태를 확인하며, 완료되면 전체 파일 목록 대신 파일 수·전체 용량·큰 파일 수·중복 그룹 수·중복 낭비 용량·읽지 못한 항목 수와 한도 도달 여부만 받습니다. 전체 결과와 삭제 판단 근거는 앱 안에 남습니다.

## 안전과 개인정보

- 연결 통로는 `127.0.0.1`에만 열리고 앱을 시작할 때마다 새 토큰을 발급합니다.
- 같은 사용자 계정에서 BroomSweepy를 두 번 실행해도 첫 앱의 연결 정보는 덮어쓰지 않습니다.
- 요청과 응답에는 크기 상한과 프레임 전체 10초 읽기·쓰기 기한이 있습니다. 파일·문서 검색어는 최대 256자, 결과는 최대 250개이며 외부 검색은 한 번에 하나만 실행하고 8초가 지나거나 앱이 종료되면 SQLite 조회를 중단합니다.
- 제어 도구에는 분석 코어, 휴지통 이동, 셸 실행 기능이 없습니다.
- 외부 검색은 `AI 도우미`에서 이번 실행에 허용한 현재 파일 목록·문서 목록만 조회합니다. 목록의 기준 폴더가 바뀌면 다시 허용해야 합니다.
- 외부 검사는 검색 허용을 재사용하지 않습니다. 별도 검사 허용에 저장한 폴더를 시작 직전에 다시 확인하며, CLI가 임의 경로를 넣을 수 없습니다.
- 검사·색인·검색은 하나의 작업 잠금을 사용합니다. 겹친 디스크 작업은 줄을 세우지 않고 `busy`로 거부합니다.
- 취소는 작업 번호가 현재 검사와 정확히 일치할 때만 원자 취소 신호를 설정합니다. 앱이 실제 작업을 멈춘 뒤에만 상태가 `cancelled`로 확정됩니다.
- 완료 결과에는 성공한 검사에만 증가하는 검사 번호가 붙고, 최근 작업 상태는 현재 앱 실행 동안 16개까지만 보관합니다.
- 검색 결과에는 파일 이름·경로가 포함되고 문서 검색에는 일치 문맥이 포함될 수 있습니다. 채팅 CLI를 외부 공급자에 연결하면 그 결과가 해당 공급자에게 전달될 수 있으므로 필요한 목록만 허용하고 결과 수를 작게 정합니다.
- 현재 v2는 읽기 전용 검색과 읽기 전용 검사 시작·상태·취소까지만 제공합니다. 이후 휴지통 이동을 연결하더라도 CLI는 확인 계획만 만들고, 정확한 목록을 앱에서 보여 준 뒤 사용자가 최종 확인해야 실행됩니다.
