# Windows 관리 — RemoveWindowsAI 조사와 후속 범위

2026-10-20 · Codex · 조사 시점의 제안과 제외 근거를 보존한다. 이후 고정10행 조회·8개 공식 설정 연결을 구현했다. 현재 기능 정본은 [구현 계약](implementation-contract.md), 실제 확인 범위는 [검증 기록](../../qa/windows-management/verify-report.md)을 따른다. Windows 실기 실행·설치 검증은 남아 있다.

## 제품 경계

BroomSweepy의 Rust 엔진·React 화면·typed 앱 도구 계약은 공통이며 native adapter는 OS별로 분리한다. 현재 `맥 관리` 메뉴는 macOS에서만 노출하고 CFPreferences를 사용한다. Windows에서 Finder 키를 흉내 내거나 Mac 검증을 Windows 검증으로 계산하지 않는다. Windows 설치 프로그램·테스트 구성은 `.github/workflows/windows-check.yml`에 있지만 이번 개발본의 해당 CI/실행/설치를 새로 검증하지 않았다.

## 읽기 전용 조사

[RemoveWindowsAI 고정 커밋](https://github.com/zoicware/RemoveWindowsAI/commit/6c30eeb5f48c4d0cc8be16ea32cc108fc55a9c7c), 2026-10-17. README/Documentation/라이선스와 주 스크립트의 실행·백업·복원 관련 함수를 대조했다. 전체 행 보안 감사는 아니다. 스크립트·내장 코드·다운로드 명령은 실행하지 않았다.

| 대상 | 확인한 동작 | BroomSweepy 판단 |
|---|---|---|
| 고정 기능·정책 목록 | registry/패키지/optional feature 분류 | 목록 구조 참고, 실제 조회는 native API로 별도 구현 |
| `DisableRegKeys` | 정책 변경 외 서비스·시스템 파일 제거도 혼재 | 함수 통째 재사용 금지 |
| `Run-Trusted` | TrustedInstaller 실행 경로 변경/SYSTEM task | 제외 |
| Appx/CBS/AI 파일 제거 | 비제거 패키지·시스템 데이터 강제 변경 | 제외 |
| 재설치 방지·자동 재정리 | certificate/custom CAB/SYSTEM 예약 작업 | 제외 |
| `DisableDefenderAI` | 보호 설정·IFEO 변경 | 제외 |
| revert/backup | 일부 registry는 반대 상수, Appx는 이름 목록 후 재다운로드; Recall/CBS 제거는 원복 경로 없음 | 원래값·부재·사용자 데이터 복원을 보장하지 않음. 기존 앱의 checked journal을 대체하지 않음 |

근거는 고정 커밋의 [주 스크립트](https://github.com/zoicware/RemoveWindowsAI/blob/6c30eeb5f48c4d0cc8be16ea32cc108fc55a9c7c/RemoveWindowsAi.ps1) 각 함수다. 문서의 “fully revert”를 제품 보장으로 가져오지 않는다. [MIT 라이선스](https://github.com/zoicware/RemoveWindowsAI/blob/6c30eeb5f48c4d0cc8be16ea32cc108fc55a9c7c/LICENSE) 소스 고지는 복제 시 유지해야 한다. 저장소의 Windows 추출 바이너리 재배포 권한까지 확인한 것은 아니며 코드·바이너리는 복제하지 않았다.

## 공식 지원과의 차이

- 기존 `TurnOffWindowsCopilot`은 deprecated이며 새 Copilot 앱을 끄는 증거가 아니다. [Microsoft 정책 정의](https://learn.microsoft.com/en-us/windows/client-management/mdm/policy-csp-windowsai#turnoffwindowscopilot)
- Recall 가용성/저장 정책은 snapshot 삭제를 수반할 수 있고 component 제거에는 재부팅도 필요하다. 일반 가역 Bool과 분리해야 한다. Home·Insider·관리형 정책을 같은 지원 상태로 취급하지 않는다. `RemoveMicrosoftCopilotApp`의 Pro 표/본문 불일치는 **확인 필요**다. [WindowsAI 정책](https://learn.microsoft.com/en-us/windows/client-management/mdm/policy-csp-windowsai)
- Notepad의 `HKLM\SOFTWARE\Policies\WindowsNotepad\DisableAIFeatures`는 컴퓨터 전체 정책이다. 공식 최소 조건은 Windows11 22H2/Notepad11.2503.16.0이며 일반 사용자 선호값으로 표현하지 않는다. [Notepad 관리](https://learn.microsoft.com/en-us/windows/client-management/manage-notepad?tabs=gpo)

## 최초 제안: 고정 5종 정책 조회 + 설정 연결

아래5종 제안은 이후 Explorer4행과 Recall의 독립 사용자/컴퓨터 범위를 더해10행으로 구체화했다. 현재 명령·화면·`windows.inspect`는 위 구현 계약을 따른다.

| 조회 후보 | 표시·검증 조건 |
|---|---|
| legacy Copilot policy | deprecated, 새 앱 상태 추정 금지 |
| Recall 저장 policy | 저장값과 실제 사용자 opt-in 상태 분리, 삭제 효과 안내 |
| Recall 가용성 policy | 설치/지원/저장값 구별, 제거 효과 안내 |
| Notepad AI policy | machine scope·OS/앱 버전 확인 |
| Paint Image Creator policy | 공식 build/SKU 조건 확인 |

1. 기존 Windows registry 의존성과 OS별 composition을 사용한다. 고정 allowlist만 KEY_READ로 읽고, PowerShell 전체 스크립트·상시 관리자 프로세스·registry 쓰기는 추가하지 않는다.
2. `storedValue`/`valueType`/`scope`/`supportReason`을 구분한다. 부재·접근 거부·타입 오류·미지원·deprecated는 서로 다른 상태다. 저장값을 실제 기능의 effective state로 단정하지 않는다. OS build/UBR/SKU/앱 버전이 미확인이면 지원 확정 대신 미확인을 표시한다.
3. 사용자에게는 고정 ID→허용된 `ms-settings:` URI만 연결한다. privacy/appsfeatures/optionalfeatures 등 [Microsoft 공식 URI 목록](https://learn.microsoft.com/en-us/windows/apps/develop/launch/launch-settings)과 대조한다. 외부 호출자의 임의 URI·registry 경로·명령은 받지 않는다. `privacy-recall`은 이번 공식 목록에서 확인되지 않아 추가하지 않는다.
4. AI/MCP는 같은 앱 조회 결과의 bounded projection만 사용한다. 기존 파일 `full_access`는 OS 변경 권한이 아니다. 일반 파일/휴지통/앱 제거 엔진을 우회하지 않는다.
5. 향후 쓰기는 별도 범위다. 기존값·부재·타입과 충돌·내구성·관리 정책·재부팅/데이터 삭제 효과를 설계하고, Windows에서 실제 원복을 검증한 항목만 제공한다. registry 값을 되돌려도 삭제된 snapshot을 복원한 것은 아니다.

## 완료 기준과 현재 증거

- 고정 입력/schema, 32/64-bit registry view, 미설정/형식/권한/미지원과 출력 상한을 합성 테스트한다.
- Windows CI 컴파일·unit·installer 패키징과 실제 Windows 일반 사용자 조회/URI/권한 실패를 별도로 확인한다. 관리자 요구가 있으면 실패를 권한 상승으로 자동 바꾸지 않는다.
- Mac에서는 Windows 경로가 호출되지 않고 기존 Mac 메뉴·CFPreferences·파일 엔진이 유지되는지 회귀한다.
- **후속 구현:** 고정10행 native 조회·공식8진입·읽기 전용 MCP·OS별 메뉴를 구현하고 Mac 회귀/교차 타입 검사를 수행했다. **NOT RUN:** 실제 Windows 조회·설정 화면 표시·설치. OS 설정 쓰기·효과·복원은 구현 범위 밖이다.

현재 Mac 후속 결과는 [검증 보고서](../macos-management/verify-report.md)에 있다. Windows 단계는 Mac 정식 검증 완료와도 별개다.

#tags: windows관리, removewindowsai, 정책조회, 플랫폼분리, 복원경계
