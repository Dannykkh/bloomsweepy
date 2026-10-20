# BroomSweepy Domain Dictionary

> 2026-09-07 · v1 · 앱 관리/공간 정리 기능 컨텍스트에서 기존 타입과 사용자 표현을 정리한 개발용 사전.

| 영문 식별자 | UI 표기 | 정의와 경계 |
|---|---|---|
| InstalledApplication | 설치된 앱 | 운영체제 설치 정보 또는 알려진 앱 폴더에서 확인한 응용프로그램. 전체 드라이브 파일 검색과 다름 |
| applicationBundle | 앱 본체 | macOS `.app` 패키지. 문서·설정·별도 지원 데이터/서비스는 포함하지 않음 |
| applicationTrashPlan | 앱 휴지통 이동 검토 | 하나의 앱 본체를 옮기기 위한 서버 발급 일회용 확인. 관련 데이터는 별도 선택·검토·확인 |
| applicationDataCandidate | 관련 데이터 후보 | 정확한 앱 식별자로 연결된 한정 캐시·환경설정. 사용자 문서·공유 데이터·소유 불명 항목은 제외 |
| inspectFile | 열기 | 허용된 일반 문서/미디어를 기본 앱으로 여는 동작. 프로그램 실행을 의미하지 않음 |
| openDirectory | 폴더 열기 | 일반 폴더를 OS 파일 탐색기에서 여는 동작. 앱 내 재스캔/드릴다운과 다름 |
| revealPath | 위치 표시 | 파일/폴더 자체를 실행하지 않고 OS 파일 탐색기에서 위치를 표시 |
| directoryScan | 하위 폴더 탐색 | 앱 안에서 선택한 폴더를 분석하고 용량 지도로 이동 |
| emptySystemTrash | 운영체제 휴지통 비우기 | 다른 앱의 항목까지 포함하는 별도 영구 삭제. 일반 휴지통 이동/앱 제거와 다름 |
| storedPreference | 저장된 설정값 | 명시적으로 저장된 Bool 또는 미설정. 운영체제 기본값·Finder 화면 반영 여부를 추정하지 않음 |
| macSettingChange | 설정 변경 기록 | 변경 전 값의 부재까지 보존하는 내구성 저널. 파일 휴지통 승인과 별개 |
| developerCache | 개발 캐시 | 알려진 Xcode 출력 위치의 오래된 직계 항목. 삭제 안전·물리 확보량을 보장하지 않는 검토 후보 |
| macosInspect | 맥 설정 조회 | 고정 Finder 설정 5개의 공통 읽기 전용 기능. 시스템 설정 변경 권한을 부여하지 않음 |
| canRestore | 복원 가능 | 지원·수정 가능한 설정의 최신 미복원 기록에 한함. 현재값이 적용값과 일치하면 이전값을 복원하며, 미완료 기록의 원래값이 이미 존재하면 명시적 복원 요청으로 설정 쓰기 없이 기록만 완료할 수 있음. 세부 조건은 [Mac API 계약](plan/macos-management/api-spec.md)을 따름 |
| windowsInspect | Windows 설정 조회 | 고정 탐색기4개·AI 정책6개의 레지스트리 저장값을 읽는 공통 기능. 사용자/컴퓨터 범위를 합성하지 않고 실제 작동·지원 상태를 추정하지 않음 |
| storedWindowsValue | 저장된 Windows 설정값 | REG_DWORD 원시값. 부재·접근 거부·형식 불일치·미지원값·오류는0/false와 다름 |
| windowsSettingsTarget | 공식 설정 화면 연결 | 로컬 main 창에서 고정8개 ID만 열기 요청. requested는 화면 표시·설정 변경·제거 완료를 뜻하지 않으며 AI/MCP 실행 도구가 아님 |

## 금지하는 혼동

- 앱 본체 이동을 `완전 제거` 또는 `관련 데이터까지 삭제`라고 표현하지 않는다.
- 미측정 용량을 `0 B`로 표시하지 않는다.
- `위치 표시`, `열기`, `하위 폴더 탐색`을 하나의 작업으로 표시하지 않는다.
- 앱 본체 휴지통 이동을 `영구 삭제`와 혼동하지 않는다.

## 변경 이력

| 날짜 | 변경 | 근거 |
|---|---|---|
| 2026-09-07 | 기존 타입·UI와 신규 요청의 8개 용어 정리 | WorkPM 앱 목록/파일 열기 조사. 프로젝트 특수 경계이므로 글로벌 사전으로 승격하지 않음 |
| 2026-10-20 | Mac 관리 5개 용어 추가, feature v3 확정 | docs/plan/macos-management/domain-dictionary-delta.md |
| 2026-10-20 | canRestore에 미완료 기록의 무쓰기 완료 경로 반영 | Argos A3, mac_settings.rs snapshot/restore |
| 2026-10-20 | Windows 저장값 조회·공식 설정 연결3개 용어 추가 | [Windows 구현 계약](plan/windows-management/implementation-contract.md) |
