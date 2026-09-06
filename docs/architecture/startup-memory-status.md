# BroomSweepy 자동 시작과 시스템 메모리 상태

## 문서 상태와 적용 범위

이 문서는 `v1.6.0` Windows·macOS Tauri 앱의 자동 시작과 성능·메모리 계약을 기록합니다. 기존 Windows 수명주기 검증과 이번 Mac 설치·로컬 검사 검증을 구분합니다. 실제 로그인 재진입, 일반 앱 정상 종료, 최신 Windows 패키지의 전체 실행 검증은 각 운영체제의 격리된 출시 환경에서 별도로 확인해야 합니다.

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

시스템 메모리 표시는 `v1.6.0`에서 별도 성능 화면으로 이동했습니다. 호환성을 위해 유지하는 Tauri 명령 `get_system_memory_status`는 블로킹 작업자에서 `sysinfo` 메모리 값을 새로 읽고 다음 필드를 반환합니다.

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

## 메모리 측정이 하지 않는 일

메모리 측정은 읽기 전용입니다. 아래 동작은 측정 계약에 포함하지 않으며, 성능 화면의 별도 앱 메모리 정리는 다음 절에 한정된 범위만 제공합니다.

- Windows 프로세스의 working set 비우기 또는 축소
- Windows standby 목록이나 시스템 파일 캐시 purge
- macOS에 대량 할당을 만들어 인위적인 메모리 압박 유도
- 다른 앱의 캐시, allocator, private memory 또는 메모리 매핑 해제
- 메모리 정리를 가장한 프로세스 종료, 강제 재시작 또는 swap 비우기
- 메모리 사용량 숫자만 일시적으로 낮추는 최적화 동작

메모리 누수는 소유 프로세스가 여전히 사용 중이라고 표시한 할당입니다. BroomSweepy가 그 메모리를 외부에서 안전하게 해제할 수 없으며, 해당 프로세스의 소유권·수명주기 버그를 수정하거나 프로세스를 재시작해야 합니다. 이 패널의 숫자 변화만으로 누수 여부를 판정하지 않습니다.

## 성능 화면과 정상 종료 계약

`성능` 화면은 지속되는 Rust `sysinfo::System` 샘플러로 전체 CPU·메모리·swap과 상위 프로세스의 CPU·resident memory를 측정합니다. 요청마다 새 샘플러를 만들지 않으며, 프로세스 명령행·환경변수·전체 실행 경로는 프론트엔드에 전달하지 않습니다. 화면이 오래된 스냅샷을 표시할 때는 종료 행동을 비활성화합니다.

첫 출시의 프로세스 행동은 macOS 일반 GUI 앱 하나에 보내는 정상 종료 요청만 지원합니다. 이는 메모리를 직접 정리하는 기능이 아니며, 자동 선택·프로세스 트리 종료·강제 종료 폴백이 없습니다. 다른 플랫폼에서는 프로세스 수치를 읽기 전용으로 표시합니다.

성능 화면의 별도 `메모리 정리` 행동은 macOS에서 `malloc_zone_pressure_relief(NULL, 0)`를 호출해 **현재 BroomSweepy 호스트 프로세스의 malloc 영역**이 반환할 수 있는 페이지를 운영체제에 돌려줍니다. Apple SDK 계약이 반환한 `allocatorReleasedBytes`만 이 행동의 직접 결과이며, 0바이트도 실패가 아니라 이미 반환할 영역이 없다는 완료 상태입니다. 대량 임시 할당으로 압박을 만들지 않고 URL/WebView 캐시, 다른 앱, 시스템 파일 캐시, compressed memory와 swap은 건드리지 않습니다.

명령은 입력을 받지 않으며 `AtomicBool` lease로 한 번에 하나만 실행합니다. 실행 전후의 현재 호스트 resident memory와 시스템 available memory는 진단 관찰값일 뿐입니다. WebKit 보조 프로세스를 합친 앱 전체 메모리가 아니며, 다른 프로세스 활동에 따라 오를 수도 있으므로 UI는 이 차이를 `확보량`으로 표시하지 않습니다. macOS 외 플랫폼은 같은 IPC 계약에서 `unsupported`를 반환하고 실행 버튼을 노출하지 않습니다.

프론트엔드는 raw PID를 종료 명령에 넘기지 않습니다. 백엔드가 만든 snapshot target과 termination preview는 난수 불투명 ID, 짧은 만료 시간과 단일 사용 제한을 가지며, 실행 직전에 PID·시작 시각·실행 파일·사용자·bundle ID·AppKit 앱 신원을 다시 확인합니다. snapshot에서 확보한 동일한 `NSRunningApplication` 객체를 preview와 실행까지 보관하므로 검증 뒤 PID로 새 앱 객체를 다시 찾지 않습니다. BroomSweepy 자신과 Finder·Dock·SystemUIServer·loginwindow·WindowManager 같은 명시적 시스템 대상은 거부합니다. 검증한 객체에만 `terminate()`를 호출하고 실제 종료 여부를 제한 시간 동안 관찰합니다.

## 기존 SwiftUI 구현과의 경계

`BroomSweepy/Services/StartupManager.swift`는 사용자와 시스템의 `LaunchAgents/*.plist`를 찾아 각 파일의 `Disabled` 값을 읽고 쓰는 기존 macOS 관리 기능입니다. Tauri 앱 자신을 Windows와 macOS 자동 시작에 등록하는 새 설정 계약이 아니며, 신규 토글에서 재사용하지 않습니다.

`BroomSweepy/Services/MemoryManager.swift`의 `purgeMemory`는 URL 캐시를 비우고, 메모리를 반복 할당·해제해 압박을 유도한 뒤, 현재 SwiftUI 프로세스에 `malloc_zone_pressure_relief`를 요청합니다. Tauri에는 이 압박 할당과 URL 캐시 초기화를 이식하지 않습니다. 공통점은 마지막 current-process allocator relief뿐이며, 새 동작도 다른 앱의 누수 메모리를 해제하지 않습니다.

## 출시 전 검증

- Windows 새 설치에서 자동 시작이 꺼져 있고, 켜기·앱 재시작·로그인 재진입·끄기 뒤 실제 OS 등록 상태가 토글과 일치하는지 확인합니다.
- macOS 패키지에서 같은 상태 전환과 `LaunchAgent` 등록·해제를 확인합니다.
- 두 플랫폼에서 자동 시작 프로세스가 `--background`로 창을 띄우지 않고 제어 서버를 준비하는지 확인합니다.
- 숨겨진 인스턴스가 실행 중일 때 일반 실행은 기존 창을 복원하고, 백그라운드 중복 요청은 창을 앞세우지 않는지 확인합니다.
- Windows 트레이의 열기·종료와 macOS Dock `Reopen` 동작을 패키징된 앱에서 확인합니다.
- 메모리 응답의 byte 단위, `usedBytes` 계산, 플랫폼별 swap 표시와 조회 실패 UI를 Windows와 macOS에서 확인합니다. Windows에서는 commit 기반 추정치를 실제 페이지 파일 사용량으로 표시하지 않는지도 확인합니다.
- 설정 화면과 명령 목록에 시스템 전체 purge, pressure-allocation, working-set trim, 강제 종료 또는 프로세스 트리 종료 경로가 추가되지 않았는지 회귀 확인합니다.
- 성능 화면의 메모리 정리가 동시 실행을 거부하고, allocator 반환량 0과 양수를 구분하며, RSS·시스템 available 관찰값을 확보량으로 표시하지 않는지 확인합니다.
- 성능 sampler의 두 번째 CPU 표본, bounded CPU/RAM union, 오래된 snapshot 차단, 종료 preview 만료·단일 사용·PID 재사용 거부와 자기/시스템 앱 보호를 확인합니다.
