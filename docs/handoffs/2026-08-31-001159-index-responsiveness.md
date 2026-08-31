# Handoff: 색인 중 응답성과 확정 단계 취소

## Session Metadata
- Created: 2026-08-31 00:11:59 +09:00
- Project: `<repository-root>`
- Branch: main

## Current State Summary
FastFind 벤치마크에서 재현된 본문 검색 서버 정지를 기준으로 BroomSweepy의 장기 작업 제어와 SQLite 색인 읽기 경로를 회귀했다. 기존 `spawn_blocking`과 `ScanRuntime` 제어는 즉시 응답했지만, 문서·파일 색인 쓰기 트랜잭션 동안 상태 조회가 2초 뒤 `database is locked`로 실패하는 문제를 발견했다. 읽기 연결에서 WAL 재설정과 현재 스키마 DDL을 제거하고, 확정 단계 취소를 커밋 직전까지 재확인하도록 수정했다.

## Work Completed
- [x] 블로킹 작업 중 상태 확인·중복 작업 거부·취소 플래그 응답 상한 테스트
- [x] 문서·파일 색인 쓰기 중 마지막 완료 스냅샷 상태·검색 회귀 테스트
- [x] 읽기 연결에서 `PRAGMA journal_mode=WAL` 재실행과 현재 스키마 DDL 제거
- [x] 문서 전체 색인, 파일 전체 카탈로그, USN 증분 카탈로그의 확정 단계 취소 재확인
- [x] 취소된 세대가 검색 결과와 상태에 노출되지 않는 롤백 검증
- [x] 포맷·Clippy·전체 Rust 테스트·TypeScript·Vite 빌드 검증

### Files Modified
| File | Changes |
|------|---------|
| crates/bloomsweepy-core/src/document_search.rs | 현재 스키마 읽기 연결 분리, 확정 전 취소 재확인 |
| crates/bloomsweepy-core/src/file_catalog.rs | 현재 스키마 읽기 연결 분리, 전체·USN 확정 전 취소 재확인 |
| crates/bloomsweepy-core/tests/index_responsiveness.rs | 활성 writer 중 스냅샷 읽기·응답 상한·취소 롤백 회귀 |
| apps/desktop/src-tauri/src/lib.rs | 블로킹 작업 중 제어 경로 응답성 테스트 |
| docs/architecture/scan-runtime-safety.md | 잠금 원인, 새 계약, 실측값과 테스트 위치 |
| README.md | 색인 중 완료 스냅샷과 확정 단계 롤백 경계 |

### Decisions Made
| Decision | Rationale |
|----------|-----------|
| writer 연결만 WAL 모드를 설정 | reader가 활성 writer의 데이터베이스 모드를 다시 설정하며 잠기는 일을 막기 위해 |
| 현재 스키마 reader는 DDL을 실행하지 않음 | WAL의 직전 커밋 스냅샷을 잠금 없이 읽기 위해 |
| 스키마 0 또는 구버전만 기존 초기화·마이그레이션 사용 | 기존 v1 카탈로그 마이그레이션 호환성을 유지하기 위해 |
| `Finalizing` 직후와 commit 직전에 취소 재확인 | 진행 알림 뒤 들어온 취소가 새 세대를 확정하지 않게 하기 위해 |
| 장기 작업은 계속 하나만 허용 | 디스크·CPU 경합을 늘리지 않으면서 상태와 취소 제어만 독립적으로 유지하기 위해 |

## Verification Evidence
- `cargo fmt --all -- --check`: 통과
- `cargo clippy --workspace --all-targets -- -D warnings`: 통과
- Rust: 코어 단위 38개, 응답성 2개, Windows 자원 1개, Tauri 20개 통과; OS 조작·관리자 권한 테스트 5개 의도적 제외
- `npm run check`, `npm run build`: 통과
- 활성 writer 중 문서 상태 1.052ms, 문서 검색 1.4294ms
- 활성 writer 중 파일 카탈로그 상태 1.2693ms, 파일 검색 2.3156ms
- 런타임 상태·중복 거부·취소 제어 합계 6.2µs
- 20회 반복 뒤 일반 스캔·문서 색인·파일 카탈로그 모두 핸들 `158→158`, 스레드 `3→3`; private bytes 증가는 허용 상한 16MiB 이내

## Pending Work
### Immediate Next Steps
1. 실제 패키징된 WebView2 앱에서 대형 문서 색인 중 화면 전환·상태 조회·취소 버튼 응답을 자동화한다.
2. 느린 UNC·외장 드라이브의 단일 커널 I/O 지연을 별도 시간 제한 worker 또는 프로세스 경계로 격리할지 설계한다.
3. P1 빠른 파일 찾기의 구조화 질의 파서와 필터 UI를 구현한다.

### Blockers/Open Questions
- [ ] 현재 UI는 장기 작업 중 검색 입력을 비활성화한다. 코어는 직전 완료 스냅샷을 읽을 수 있지만 이를 사용자 기능으로 노출할지는 별도 UX 결정이 필요하다.
- [ ] 이미 진입한 운영체제 `read`, `read_dir`, `metadata` 호출은 협력적 취소 플래그로 강제 중단할 수 없다.
- [ ] macOS APFS에서 동일한 WAL 읽기·취소·자원 회귀를 실행할 러너가 필요하다.

## Context for Resuming
### Important Context
`open_index`는 writer 전용으로 WAL·동기화 설정을 유지한다. 상태와 검색은 `open_existing_index`와 `ensure_existing_schema`를 사용하며 현재 버전이면 DDL을 실행하지 않는다. 파일 카탈로그 v1은 읽기 진입 시 기존 마이그레이션 경로를 계속 사용한다. 테스트는 writer를 `Finalizing` 콜백에서 정지시켜 실제 트랜잭션 잠금과 취소 타이밍을 재현한다.

### Potential Gotchas
- reader 경로에 `PRAGMA journal_mode=WAL`이나 무조건적 `CREATE TABLE IF NOT EXISTS`를 다시 넣으면 활성 writer 중 2초 대기 뒤 `database is locked`가 재발한다.
- `rebuild_bulk_indexes` 같은 단일 SQLite 호출 자체는 중간 취소할 수 없으므로 호출 직후 플래그를 확인해 commit을 막는다.
- 응답 시간 수치는 작은 로컬 fixture의 단일 Windows 실행값이며 실제 시스템 드라이브 성능을 대표하지 않는다.
