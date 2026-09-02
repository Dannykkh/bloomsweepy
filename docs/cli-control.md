# 채팅 CLI와 MCP 연결

BroomSweepy의 제어 도구는 파일을 직접 검사하거나 삭제하는 독립 프로그램이 아닙니다. Codex나 Claude Code의 구조화된 요청을 현재 로그인 사용자가 실행 중인 BroomSweepy 앱으로 전달하고, 앱이 만든 제한된 결과만 돌려줍니다. 앱이 꺼져 있으면 분석 엔진을 대신 실행하지 않고 `app_not_running`을 반환합니다.

## 현재 제공 기능

| MCP 도구 | 실제 처리 주체 | 설명 |
|---|---|---|
| `status` | BroomSweepy 앱 | 앱 연결과 현재 상태 확인 |
| `system_overview` | BroomSweepy 앱 | 드라이브 용량 요약 확인 |
| `search_files` | BroomSweepy 앱 | 앱이 미리 만든 파일 목록 검색 |
| `search_documents` | BroomSweepy 앱 | 앱이 미리 읽어 둔 문서 내용 검색 |
| `start_storage_scan` | BroomSweepy 앱 | `대화`에서 이번 실행에 허용한 폴더 검사 시작 |
| `operation_status` | BroomSweepy 앱 | 작업 번호로 진행 상태와 제한된 결과 요약 확인 |
| `cancel_operation` | BroomSweepy 앱 | 정확히 일치하는 작업 번호의 검사 취소 요청 |
| `cleanup_candidates` | BroomSweepy 앱 | 완료된 보고서에서 경로 없는 정리 후보 요약 확인 |
| `create_cleanup_plan` | BroomSweepy 앱 | 익명 후보 번호로 5분짜리 앱 확인 계획 생성 |
| `cleanup_plan_status` | BroomSweepy 앱 | 계획의 대기·완료·거부·만료 상태 요약 확인 |

MCP에는 승인, 삭제, 휴지통 이동, 휴지통 비우기, 레지스트리 변경, 임의 셸 실행 도구가 없습니다. 정리 계획이 만들어지면 BroomSweepy 앱이 정확한 경로와 이유를 로컬 화면에 표시합니다. 사용자가 그 화면에서 최종 확인한 경우에만 앱이 파일 상태를 다시 검사하고 기존 작업 기록과 운영체제 휴지통 기능으로 이동합니다.

## 세 가지 별도 권한

- `검색 허용`: 앱이 이미 만든 파일·문서 목록을 외부 채팅 도구가 검색할 수 있습니다. 결과에는 경로와 문서 일치 문맥이 포함될 수 있습니다.
- `폴더 검사 허용`: 현재 폴더와 검사 설정을 앱에 고정합니다. 외부 도구에는 경로 인수가 없고 시작 요청만 있습니다.
- `정리 계획 검토 허용`: 완료된 시스템 정리 또는 중복 검사에서 익명 후보 요약과 확인 계획을 만들 수 있습니다. 이 권한만으로 파일을 이동할 수 없습니다.

세 권한은 서로 대신하지 않으며 현재 앱 실행에만 적용됩니다. 폴더나 검사 설정이 바뀌거나 앱을 다시 켜면 필요한 권한을 다시 확인해야 합니다.

## 설치본에서 연결

Windows MSI와 설치 EXE에는 앱과 버전이 같은 `bloomsweepy-mcp.exe`가 포함됩니다. BroomSweepy의 `설정 > 외부 AI에 BroomSweepy 연결`에서 실제 실행 파일 경로와 변경 내용을 확인한 뒤 Codex 또는 Claude Code를 연결합니다.

- 앱은 각 공급자의 공식 CLI 명령을 인수 배열로 실행하며 설정 파일을 직접 편집하지 않습니다.
- 같은 이름의 기존 연결이 앱 소유 기록과 다르면 덮어쓰거나 제거하지 않습니다.
- 연결 해제도 현재 설정이 앱이 등록한 내용과 정확히 같을 때만 수행합니다.
- 개발 빌드에서는 외부 설정 변경을 막고 상태 조회만 제공합니다.
- Claude Code 연결은 Claude Desktop 앱 연결을 의미하지 않습니다. Claude Desktop은 별도 확장 패키지 범위입니다.

연결을 바꾼 뒤 Codex 또는 Claude Code를 다시 시작해야 새 MCP 서버가 보일 수 있습니다.

## 개발 빌드와 직접 확인

저장소에서 앱과 같은 버전의 플랫폼별 보조 파일을 준비합니다.

```powershell
cd apps\desktop
npm run prepare:sidecar
```

Windows에서는 생성된 `src-tauri\binaries\bloomsweepy-mcp-x86_64-pc-windows-msvc.exe`를 직접 실행할 수 있습니다. 아래에서는 짧게 `<helper>`로 표시합니다.

```powershell
<helper> status
<helper> system-overview
<helper> search-files "보고서" --max-results 20
<helper> search-documents "계약 기간" --max-results 20
<helper> start-scan
<helper> operation-status <작업번호>
<helper> cancel-operation <작업번호>
<helper> cleanup_candidates --source system_cleanup --max-results 20
<helper> create_cleanup_plan --source system_cleanup --source-generation <검사번호> --candidate-id <후보번호>
<helper> cleanup_plan_status <계획번호>
```

개발 환경에서 수동으로 등록하려면 절대 경로를 사용합니다. 설치본에서는 앱 설정 화면을 우선합니다.

```powershell
codex mcp add bloomsweepy -- "<helper-absolute-path>" mcp
claude mcp add --scope user bloomsweepy -- "<helper-absolute-path>" mcp
```

## 정리 검토 흐름

1. 앱에서 시스템 정리 또는 중복 검사를 끝냅니다.
2. `대화`에서 이번 실행의 정리 계획 검토를 허용합니다.
3. `cleanup_candidates`가 기본 20개, 최대 50개의 종류·신뢰도·논리 용량·익명 후보 번호를 반환합니다. 파일 이름과 경로는 반환하지 않습니다.
4. 채팅 도구가 후보 번호를 골라 `create_cleanup_plan`을 호출합니다. 같은 후보 집합의 재전송은 기존 계획 번호를 돌려주고 다른 계획은 대기 중 계획이 끝날 때까지 거부합니다.
5. 앱의 검토 알림을 열어 정확한 경로, 개수, 용량과 이유를 확인합니다. 계획은 5분 뒤 만료되고 앱 재시작 뒤 복원되지 않습니다.
6. 사용자가 앱에서 거부하면 아무 파일도 바뀌지 않습니다. 승인하면 앱이 검사 번호와 항목을 다시 확인한 뒤 운영체제 휴지통으로 이동합니다.
7. `cleanup_plan_status`에는 이동·실패·건너뜀 개수와 논리 용량만 반환합니다. 정확한 경로와 작업 기록은 앱 안에 남습니다.

## 안전과 개인정보

- 연결 통로는 `127.0.0.1`에만 열리고 앱을 시작할 때마다 새 토큰을 발급합니다.
- 같은 사용자 계정에서 BroomSweepy를 두 번 실행해도 첫 앱의 연결 정보는 덮어쓰지 않습니다.
- 요청과 응답에는 크기 상한과 프레임 전체 10초 읽기·쓰기 기한이 있습니다. 외부 검색은 한 번에 하나만 실행하고 제한 시간이 지나거나 앱이 종료되면 SQLite 조회를 중단합니다.
- 제어 도구에는 분석 코어와 휴지통 이동 구현이 없습니다. 모든 파일 I/O와 실행은 앱 프로세스가 담당합니다.
- 파일·문서 검색 결과에는 경로와 일치 문맥이 포함될 수 있으므로 필요한 목록만 허용하고 결과 수를 작게 정합니다.
- 폴더 검사는 경로를 외부 입력으로 받지 않으며, 완료 뒤에도 전체 파일 목록 대신 제한된 집계만 반환합니다.
- 정리 후보 응답은 최대 50개이며 이름·경로·해시 대신 앱 실행마다 바뀌는 32자리 익명 후보 번호를 사용합니다.
- 후보 번호는 검사 번호에 묶입니다. 새 검사로 결과가 바뀌거나 파일 상태가 달라지면 이전 계획을 실행하지 않습니다.
- 중복 정리는 전체 내용 검증을 마친 그룹만 사용하고 반드시 한 복사본을 남깁니다. 서로 다른 폴더의 복사본도 앱에서 각각 보여 줍니다.
- 실제 여유 공간은 운영체제 휴지통을 비운 뒤에 늘어납니다. 복원 가능성은 운영체제와 저장장치 상태에 따라 보장할 수 없습니다.

자세한 검사 계약은 [채팅 CLI 검사 제어 v2](architecture/cli-control-v2.md), 정리 계약은 [MCP 정리 검토 v3](architecture/mcp-cleanup-v3.md), 실제 이동 경계는 [안전한 휴지통 작업](architecture/safe-trash-actions.md)을 참고하세요.
