# Design Brief: Rust One-click Memory Clean Delta

## Decision

- Rust/Tauri `성능` 화면의 메모리 pane에 큰 `메모리 정리` 원클릭 행동을 추가한다.
- 동작 범위는 macOS에서 **현재 BroomSweepy 프로세스의 malloc 영역**이 반환할 수 있는 페이지를 운영체제에 돌려주는 것으로 제한한다.
- Swift `MemoryManager`의 대량 임시 할당, 인위적 memory pressure, 다른 앱 종료 연계는 이식하지 않는다.
- 결과는 allocator가 실제로 반환했다고 보고한 바이트를 정본으로 삼고, 시스템 전체 여유 메모리 변화는 확보량으로 주장하지 않는다.

## User Evidence

- “메모리 클린 기능이 있었잖아?”
- “공간 정리 및 메모리 정리도 다 버튼 한번이면 했던거 같은데?”
- “진행하자.”

## Primary Task

사용자가 현재 CPU·RAM 상태를 확인한 뒤 확인창 없이 `메모리 정리`를 한 번 눌러 BroomSweepy 자체의 반환 가능한 allocator 메모리를 macOS에 돌려주고, 정확한 완료 결과를 바로 확인한다.

## Observable Success

- 버튼은 44px 이상이며 아이콘, 결과형 이름, 한 줄 범위 설명을 함께 제공한다.
- 실행 중에는 중복 클릭이 막히고 `정리 중…` 상태가 보조기술에도 한 번 알려진다.
- 성공 시 실제 allocator 반환량 또는 `이미 반환할 메모리가 없음`을 구분한다.
- 실패는 기존 수치를 유지한 채 다시 시도할 수 있고, 완료 뒤 성능 snapshot을 갱신한다.
- macOS 외 플랫폼에서는 실행 가능한 척하지 않고 지원 범위를 설명한다.

## Non-goals

- 다른 앱의 메모리, 시스템 파일 캐시, swap 또는 누수 메모리를 해제하지 않는다.
- 메모리 확보를 위해 큰 버퍼를 할당하거나 프로세스를 자동 종료하지 않는다.
- 전체 시스템 `available memory`의 순간 변화량을 BroomSweepy의 정리량으로 표시하지 않는다.

## Adapter Status

- Product Design plugin: `ABSENT`.
- Adapter: local React/Tauri implementation.
- 기존 SwiftUI의 시원한 원클릭 위계를 golden master로 사용하되 실행 의미는 Rust/macOS의 안전한 current-process 범위로 좁힌다.
