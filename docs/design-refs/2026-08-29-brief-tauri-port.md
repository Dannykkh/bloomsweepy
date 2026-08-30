# Design Brief: BroomSweepy Tauri Port

## Scope

- 기존 macOS SwiftUI BroomSweepy를 Windows에서도 동작하는 Rust/Tauri 2 앱으로 이식한다.
- 첫 구현 단위는 저장공간 상태, 폴더 스캔, 큰 파일 탐색, 중복 후보 검증이다.
- 기존 SwiftUI 소스는 비교 기준으로 보존한다.

## Audience

- 저장공간이 갑자기 부족해졌지만 어떤 파일을 지워도 되는지 확신하지 못하는 개인 사용자.
- macOS와 Windows를 오가며 같은 정보 구조와 안전 규칙을 기대하는 사용자.

## Primary Task

사용자가 폴더를 선택하고 스캔을 시작해 큰 파일과 중복 파일의 근거를 확인한 뒤, 복구 가능한 방식으로 정리 대상을 결정한다.

## Trust Risks

- 부분 해시만으로 서로 다른 파일을 중복으로 오인할 가능성.
- 논리 크기와 실제 디스크 점유량을 혼동할 가능성.
- 휴지통이 아닌 영구 삭제가 실행될 가능성.
- 권한 부족 또는 링크 순환 때문에 스캔 결과가 불완전할 가능성.

## Constraints

- Rust 코어는 UI와 분리하고 Windows/macOS의 파일시스템 차이는 어댑터 경계에 둔다.
- Tauri IPC로 전체 파일 목록을 한 번에 전달하지 않는다.
- 기존 글래스모피즘은 보존하지만, 지속 애니메이션과 중첩 블러는 피한다.
- macOS 전용 기능을 Windows에서 작동하는 것처럼 표시하지 않는다.
- 한국어를 우선 구현하고 기존 영어·일본어·중국어 번역 자산으로 확장 가능하게 한다.

## Observable Success

- Windows에서 Tauri 앱이 빌드되고 실제 폴더 스캔 결과를 표시한다.
- 저장공간 링, 어두운 글래스 패널, 사이드바 정보 위계가 기존 영상과 같은 제품으로 인식된다.
- 스캔의 loading, empty, error, success, cancelled 상태를 키보드만으로 확인하고 조작할 수 있다.
- 중복 결과는 전체 콘텐츠 검증을 통과한 파일만 같은 그룹으로 표시한다.

## Source Mode

- Delta port of an existing product.
- Evidence: `BroomSweepy/`, `docs/promo.mp4`, `.claude/handoffs/2026-03-26-handoff.md`, 사용자 요청.
