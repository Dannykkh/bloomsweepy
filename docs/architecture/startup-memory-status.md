# BroomSweepy 자동 시작과 시스템 메모리 상태

## 문서 상태와 적용 범위

이 문서는 Windows와 macOS용 Tauri 앱의 자동 시작과 시스템 메모리 상태 계약을 기록합니다. 현재 개발 브랜치의 계약이며 `v1.4.0` 릴리스 기능을 설명하지 않습니다. 패키징된 Windows·macOS 앱의 실제 등록, 로그인 뒤 백그라운드 실행, 단일 인스턴스 동작은 출시 전에 각 운영체제에서 다시 검증해야 합니다.

기존 `BroomSweepy/`의 macOS SwiftUI 앱은 별도 제품 경계입니다. 그 앱의 `StartupManager`와 `MemoryManager`는 아래 Tauri 계약의 구현이나 폴백이 아닙니다.

## 자동 시작 계약

자동 시작은 편의 기능이며 설치나 첫 실행만으로 활성화하지 않습니다.

| 상태 또는 동작 | 계약 |
|---|---|
| 초기 상태 | 기본값은 꺼짐입니다. 앱이 사용자의 선택 없이 자동 시작을 등록하지 않습니다. |
| 설정 화면 진입 | 로컬 설정값을 추정하지 않고 운영체제에 등록된 실제 상태를 조회해 토글에 반영합니다. |
| 사용자가 켬 | Windows 또는 macOS의 사용자 범위 자동 시작에 BroomSweepy를 등록합니다. |
| 사용자가 끔 | 같은 등록을 해제합니다. |
| 변경 결과 | 등록 또는 해제 호출 뒤 운영체제 상태를 다시 조회합니다. 토글은 재조회된 상태를 표시하며, 실패를 성공처럼 저장하지 않습니다. |
| 자동 시작 인수 | 등록된 실행에는 정확한 `--background` 인수 하나를 전달합니다. 비슷한 다른 인수는 백그라운드 실행으로 해석하지 않습니다. |
| 일반 실행 | `--background`가 없으면 주 창을 정상적으로 표시합니다. |
| 중복 실행 | Windows와 macOS에서는 한 프로세스만 유지합니다. 이미 실행 중이면 새 프로세스를 상주시킬 수 없으며 기존 인스턴스가 요청을 받습니다. |

Tauri composition root는 macOS에 `LaunchAgent` 실행 방식을 선택하고 Windows와 macOS의 등록·해제를 같은 설정 계약 뒤에 둡니다. 화면은 Tauri autostart 플러그인의 `isEnabled`, `enable`, `disable` 경계를 사용하며, 별도의 앱 설정 파일을 등록 상태의 정본으로 만들지 않습니다.

자동 시작으로 첫 인스턴스가 실행되면 창을 숨기되 앱 수명주기와 로컬 제어 서버는 유지합니다. 이미 실행 중인 인스턴스에 `--background` 요청이 들어오면 창을 갑자기 앞으로 가져오지 않습니다. 반대로 사용자가 아이콘이나 실행 파일을 직접 열어 일반 실행 요청을 보내면 숨겨진 기존 창을 복원하고 포커스를 요청합니다.

## 운영체제별 창 수명주기

| 플랫폼 | 백그라운드 시작 뒤 접근 | 창 닫기와 다시 열기 |
|---|---|---|
| Windows | 알림 영역의 BroomSweepy 트레이 아이콘 | 닫기 요청은 창을 숨기고 프로세스를 유지합니다. 트레이의 `열기` 또는 아이콘 클릭으로 기존 창을 복원하고, `종료`로 프로세스를 끝냅니다. |
| macOS Tauri | Dock의 실행 중인 BroomSweepy 아이콘 | 별도의 Windows식 트레이 계약을 만들지 않습니다. 보이는 창이 없을 때 Dock 아이콘으로 앱을 다시 열면 macOS `Reopen` 이벤트가 기존 주 창을 복원합니다. |

두 플랫폼 모두 일반 실행 요청은 새 앱 상태나 두 번째 제어 서버를 만들지 않고 기존 인스턴스를 재사용합니다. macOS Tauri 앱의 Dock 재열기 계약은 기존 SwiftUI 앱의 메뉴 막대 팝오버와 서로 다릅니다.

## 시스템 메모리 상태 계약

설정 화면의 시스템 메모리 패널은 조회 시점의 운영체제 메모리 스냅샷만 표시합니다. Tauri 명령 `get_system_memory_status`는 블로킹 작업자에서 `sysinfo` 메모리 값을 새로 읽고 다음 필드를 반환합니다.

| 필드 | 의미 |
|---|---|
| `totalBytes` | 시스템 전체 물리 메모리 |
| `availableBytes` | 운영체제가 현재 사용 가능하다고 보고한 물리 메모리 |
| `usedBytes` | `totalBytes - availableBytes`이며 값이 뒤집힌 경우 0으로 제한 |
| `totalSwapBytes` | `sysinfo::System::total_swap()`이 보고한 플랫폼별 swap 지표. Windows `sysinfo` 0.39.6에서는 `(CommitLimit - PhysicalTotal) × PageSize`를 0 아래로 내려가지 않게 계산한 값 |
| `usedSwapBytes` | `sysinfo::System::used_swap()`이 보고한 플랫폼별 swap 지표. Windows `sysinfo` 0.39.6에서는 `(CommitTotal - PhysicalTotal) × PageSize`를 0 아래로 내려가지 않게 계산한 값 |
| `capturedAtUnixMs` | 스냅샷을 읽은 시각 |
| `platform` | Rust 실행 대상 운영체제 식별자 |

`availableBytes`는 단순한 미사용 RAM만을 뜻하지 않습니다. 운영체제가 회수 가능하다고 판단한 메모리를 포함할 수 있으므로, 패널은 운영체제의 보고값을 그대로 설명하고 자체적인 “정리 가능 용량”을 계산하지 않습니다. `usedBytes`도 프로세스별 합계나 누수 판정값이 아니라 전체와 사용 가능 값의 차이입니다.

Windows의 두 swap 필드는 페이지 파일별 현재 사용량을 직접 조회하지 않는 commit 기반 추정치입니다. 특히 `usedSwapBytes`는 실제 pagefile residency나 `CurrentUsage`가 아니며, 어느 페이지 파일에 데이터가 기록됐는지도 나타내지 않습니다. 공용 응답의 필드명은 플랫폼 간 계약을 유지하기 위한 것이므로 UI와 문서는 Windows 값을 “현재 페이지 파일 사용량”으로 단정하지 않습니다.

## 하지 않는 일

메모리 패널은 읽기 전용입니다. 다음 동작을 수행하는 명령이나 버튼은 이 계약에 포함하지 않습니다.

- Windows 프로세스의 working set 비우기 또는 축소
- Windows standby 목록이나 시스템 파일 캐시 purge
- macOS에 대량 할당을 만들어 인위적인 메모리 압박 유도
- 다른 앱의 캐시, allocator, private memory 또는 메모리 매핑 해제
- 프로세스 종료, 강제 재시작 또는 swap 비우기
- 메모리 사용량 숫자만 일시적으로 낮추는 최적화 동작

메모리 누수는 소유 프로세스가 여전히 사용 중이라고 표시한 할당입니다. BroomSweepy가 그 메모리를 외부에서 안전하게 해제할 수 없으며, 해당 프로세스의 소유권·수명주기 버그를 수정하거나 프로세스를 재시작해야 합니다. 이 패널의 숫자 변화만으로 누수 여부를 판정하지 않습니다.

## 기존 SwiftUI 구현과의 경계

`BroomSweepy/Services/StartupManager.swift`는 사용자와 시스템의 `LaunchAgents/*.plist`를 찾아 각 파일의 `Disabled` 값을 읽고 쓰는 기존 macOS 관리 기능입니다. Tauri 앱 자신을 Windows와 macOS 자동 시작에 등록하는 새 설정 계약이 아니며, 신규 토글에서 재사용하지 않습니다.

`BroomSweepy/Services/MemoryManager.swift`의 `purgeMemory`는 URL 캐시를 비우고, 메모리를 반복 할당·해제해 압박을 유도한 뒤, 현재 SwiftUI 프로세스에 `malloc_zone_pressure_relief`를 요청합니다. 이 동작은 누수 메모리를 해제하지 않으며 Tauri 시스템 메모리 패널로 이식하지 않습니다.

## 출시 전 검증

- Windows 새 설치에서 자동 시작이 꺼져 있고, 켜기·앱 재시작·로그인 재진입·끄기 뒤 실제 OS 등록 상태가 토글과 일치하는지 확인합니다.
- macOS 패키지에서 같은 상태 전환과 `LaunchAgent` 등록·해제를 확인합니다.
- 두 플랫폼에서 자동 시작 프로세스가 `--background`로 창을 띄우지 않고 제어 서버를 준비하는지 확인합니다.
- 숨겨진 인스턴스가 실행 중일 때 일반 실행은 기존 창을 복원하고, 백그라운드 중복 요청은 창을 앞세우지 않는지 확인합니다.
- Windows 트레이의 열기·종료와 macOS Dock `Reopen` 동작을 패키징된 앱에서 확인합니다.
- 메모리 응답의 byte 단위, `usedBytes` 계산, 플랫폼별 swap 표시와 조회 실패 UI를 Windows와 macOS에서 확인합니다. Windows에서는 commit 기반 추정치를 실제 페이지 파일 사용량으로 표시하지 않는지도 확인합니다.
- 설정 화면과 명령 목록에 메모리 purge, working-set trim, 다른 프로세스 조작 경로가 추가되지 않았는지 회귀 확인합니다.
