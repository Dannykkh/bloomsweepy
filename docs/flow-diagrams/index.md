# BroomSweepy 흐름 도면

| 기능 | 도면 | 설명 |
|---|---|---|
| 채팅 CLI 읽기 연결 v1 | [cli-control-read-v1.mmd](cli-control-read-v1.mmd) | 현재 구현: 앱 상태·드라이브 상태·기존 파일/문서 목록 검색 |
| 채팅 CLI 검사·검색 v2 | [cli-control-read.mmd](cli-control-read.mmd) | 현재 구현: 별도 검사 허용, 작업 번호, 진행 조회와 정확한 취소 |
| 채팅 CLI 휴지통 검토 v3 | [cli-trash-review.mmd](cli-trash-review.mmd) | 현재 구현: MCP는 제한된 확인 계획만 요청하고 사용자가 앱에서 최종 확인하는 흐름 |
| MCP 설치·연결 | [mcp-registration.mmd](mcp-registration.mmd) | 현재 구현: 설치 파일에 로컬 도구를 포함하고 Codex·Claude Code 공식 명령으로만 명시적 등록·해제 |
| macOS 기능 정합화 | [macos-runtime-parity.mmd](macos-runtime-parity.mmd) | 메뉴 막대·알림 설정, 검토 전용 분기와 정리 후보→최종 확인→항목 동일성 재검사→휴지통 이동 흐름 |
| macOS 빌드 검증 | [macos-build-verification.mmd](macos-build-verification.mmd) | Windows 사전 검사, macOS CI, Swift·Tauri 앱 빌드와 배포 서명 경계 |
| 데스크톱 상주 셸 | [desktop-resident-shell.mmd](desktop-resident-shell.mmd) | Windows 트레이 아이콘과 기존 macOS 메뉴 막대의 열기·숨기기·상태·종료 대응 |
| 단순 용량 관리와 AI 도우미 분리 | [simple-storage-ai-helper-navigation.mmd](simple-storage-ai-helper-navigation.mmd) | 현재 구현: 3단계 용량 관리, 내부 결과 탭, 선택형 AI 도우미 |
| 선택형 Docker 용량 관리 | [docker-cleanup-flow.mmd](docker-cleanup-flow.mmd) | 현재 구현: 기본 꺼짐, 조건부 전용 메뉴, 폴더 없는 Docker 대화, 앱 최종 확인, 고정된 비볼륨 정리 명령 |
| macOS 정합화 검증표 | [macos-parity-verification.md](macos-parity-verification.md) | 세 도면의 구현 근거, 로컬 검증 결과와 실제 macOS에서 남은 확인 |

## 공통 경계

- 채팅 CLI와 공급자는 요약, 판단, 구조화된 명령 선택만 담당한다.
- 실제 파일 검사, 검색, 취소, 안전 재검사, 작업 기록, 휴지통 이동은 BroomSweepy 앱이 담당한다.
- 새 폴더 검사와 정리 검토는 집계와 익명 후보 번호만 보내며 정확한 정리 경로는 앱 안에서만 보여준다. 사용자가 파일·문서 검색을 별도로 허용한 경우에는 검색 결과의 경로와 일치 문맥이 공급자에게 전달될 수 있다.
- `대화`는 CLI 연결 상태와 이번 실행의 파일·문서 검색 허용, 별도 폴더 검사 허용을 보여주고 사용자가 직접 켜거나 끈다.
- Docker는 설정에서 따로 켠 경우에만 전용 메뉴와 상태 조회를 열고 대시보드에는 표시하지 않는다. Docker 대화는 폴더를 고르지 않고 범주별 요약만 전달하며, 앱이 고정된 정리 범주를 최종 확인 뒤 실행한다.
- CLI에는 영구 삭제, 휴지통 비우기, 레지스트리 변경, 임의 셸 실행을 제공하지 않는다.

## 구현 상태

- v2 연결은 앱 상태·드라이브 상태·허용한 기존 목록 검색과, 작업 번호를 쓰는 새 폴더 검사·상태 조회·정확한 취소까지 구현했다.
- v3 정리 검토는 경로 없는 후보 조회, 5분 계획, 앱 전용 최종 승인과 제한된 결과 상태까지 구현했다.
- Windows MSI·NSIS에는 앱과 같은 버전의 MCP 보조 파일이 포함된다. macOS 패키징·서명은 macOS CI 확인이 남아 있다.
