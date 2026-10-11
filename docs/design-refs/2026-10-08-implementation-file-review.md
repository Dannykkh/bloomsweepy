# Implementation log — 변경 파일 검토

Experience Contract validator 통과. 기존 Settings 방향의 국소 확장 면제에 따라 새 3방향 렌더 탐색은 생략했다.
DESIGN.md token과 기존 label/switch/button/Settings composition을 재사용했다.

- Core: native identity 기반 metadata snapshot/diff, 기존 demand-driven walk에 진입 guard·visited cap 추가.
- Runtime: 기본 OFF 설정 SQLite, 단일 worker/ScanRuntime lease, 세대 취소, 비영속 bounded 기록.
- UI: 실제 설정/상태/20행 목록, 넘김/다시 검토, 비중첩 상태 조회, 영어/일본어/중국어 78개 번역 키.
- AI/MCP: 기존 capability 정본에 files.changes 하나 추가, 이미 만든 목록의 read-only projection과 chat result 표시.
- 보완: linked ancestor와 non-UTF8 경로 reject, folder unknown-size, 저장 실패 알림, 전체 기록 아님 문구.

검증: Rust/MCP 422 통과(기존 opt-in 4 ignored), frontend119 통과, typecheck/build/fmt/diff/contract 통과.
실제 렌더 1280px/390px와 합성 state mutation을 관찰했다. 세부 근거·미실행 범위는 QA 정본과 critique 참조.
설치·릴리스·Git commit/push는 하지 않았다. 개발 서버는 검증 후 종료한다.
