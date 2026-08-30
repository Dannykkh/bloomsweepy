# Handoff: Windows NTFS MFT/USN 파일 카탈로그 공급자

## Session Metadata
- Created: 2026-08-30 14:00:46 +09:00
- Project: `<repository-root>`
- Branch: main

## Current State Summary
빠른 파일 찾기의 공용 `portableWalk` 계약 뒤에 Windows 전용 `windowsNtfs` 공급자를 추가했다. 정확한 NTFS 드라이브 루트와 원시 읽기 권한이 있으면 MFT로 전체 카탈로그를 만들고 USN Journal 체크포인트를 저장한다. 다음 수동 업데이트는 파일 변경분만 반영하며 조건이 맞지 않거나 안전하게 증분 처리할 수 없으면 공용 순회 또는 전체 MFT 갱신으로 전환한다.

## Work Completed
- [x] 읽기 전용 NTFS 부트 섹터·MFT 데이터 런·FILE 레코드 판독과 하드링크 경로 복원
- [x] USN Journal 체크포인트 저장, 변경 레코드 중복 제거, 파일 생성·수정·이름 변경·삭제 증분 반영
- [x] 폴더 구조 변경, 저널 유실·교체, 변경량 상한, 비NTFS·권한 실패의 안전한 전체 갱신 또는 `portableWalk` 폴백
- [x] SQLite 스키마 v2 마이그레이션과 공통 공급자 레코드 계약
- [x] 전체 적재 중 보조 인덱스·FTS 트리거 중단 후 일괄 재구축과 취소 롤백 검사
- [x] Tauri 상태와 React UI에 공급자 및 전체·증분 갱신 모드 표시
- [x] MIT 원저작권 고지와 현재 구현·한계 문서화
- [x] 릴리스 실행 파일 기동 확인과 MSI·NSIS 설치본 재생성

### Files Modified
| File | Changes |
|------|---------|
| crates/bloomsweepy-core/src/file_catalog/windows_ntfs.rs | Windows MFT 판독, 경로 복원, USN 체크포인트·변경분 공급자 |
| crates/bloomsweepy-core/src/file_catalog.rs | 공급자 조합, 스키마 v2, 증분 반영, 대량 인덱스 재구축, 테스트 |
| crates/bloomsweepy-core/src/lib.rs | 갱신 모드 공개 타입 내보내기 |
| crates/bloomsweepy-core/Cargo.toml | 필요한 Windows Security·IO 기능 활성화 |
| apps/desktop/src/types.ts | `windowsNtfs`, `refreshMode`, 진행 단계 타입 |
| apps/desktop/src/views/FastFileSearchView.tsx | 실제 공급자·갱신 모드와 폴백 안내 표시 |
| apps/desktop/THIRD_PARTY_NOTICES.md | AllTheThings MIT 고지 보존 |
| README.md | Windows MFT/USN 현재 동작 설명 |
| docs/architecture/fast-file-search.md | 공급자·성능·안전 폴백 정본 갱신 |
| docs/architecture/cross-platform-desktop.md | 플랫폼 경계와 후속 작업 갱신 |
| docs/architecture/scan-runtime-safety.md | MFT 상한과 실제 Windows 검증 추가 |

### Decisions Made
| Decision | Rationale |
|----------|-----------|
| MFT는 정확한 드라이브 루트에서만 선택 | 작은 하위 폴더 때문에 전체 MFT를 두 번 읽는 비용을 피하고 범위 의미를 단순하게 유지 |
| 일반 사용자 권한 실패는 오류가 아니라 공용 순회 폴백 | 기능 자체를 막지 않고 안전한 크로스플랫폼 기준선을 유지 |
| 폴더 USN 변경은 현재 전체 MFT 갱신 | 모든 하위 경로 재작성 누락보다 비용이 들더라도 정확성을 우선 |
| 상시 감시 스레드를 만들지 않고 수동 업데이트에서 USN 재생 | 기존 `ScanRuntime` 수명·동시성 계약을 깨지 않고 누수 위험을 늘리지 않음 |
| 전체 적재 인덱스는 트랜잭션 안에서 일괄 재구축 | 행별 FTS·보조 인덱스 유지 비용을 줄이면서 취소 시 직전 상태를 원자적으로 복원 |

### Verification Evidence
- 워크스페이스 테스트: 코어 38개, Windows 자원 안정성 1개, 데스크톱 19개 통과
- 실제 NTFS: MFT·Journal 조회, 1,000개 제한 검색, 전체 카탈로그 뒤 새 파일의 USN 증분 검색 통과
- 실제 종단간 시간: 최초 단계 65.56초에서 최종 29.51초로 개선
- `cargo fmt --check`, 전체 Clippy `-D warnings`, TypeScript, Vite 빌드 통과
- 릴리스 실행 파일: `Responding=True`, 확인 뒤 해당 프로세스 종료
- MSI SHA-256: `BF6F05AAA9EDC4B936EF1655870F58A5F28844F3E4A8AB574B5ABD235DE6BEFB`
- NSIS SHA-256: `833A02443EA64980F9FB47475DABB5699B63A53C4C6D1DDDDE7FACDAACAA14E8`

## Pending Work
### Immediate Next Steps
1. 일반 사용자 앱에서도 MFT 가속을 쓰도록 읽기 전용 최소 권한 서비스와 검증된 IPC를 설계한다.
2. 폴더 이름 변경·이동을 전체 MFT 재계산 없이 안전하게 반영할 경로 재작성 전략을 구현한다.
3. 실제 패키징된 앱에서 관리자·일반 사용자 각각의 공급자 표시와 폴백 UX를 자동화 검사한다.
4. macOS 공용 공급자 회귀 뒤 FSEvents 증분 어댑터를 별도 구현한다.

### Blockers/Open Questions
- [ ] 원시 볼륨 읽기 서비스의 설치·권한 상승·업데이트·IPC 인증 경계를 제품 설치 흐름과 함께 결정해야 한다.
- [ ] 전체 시스템 볼륨마다 항목 수와 저장장치가 달라 초기 SQLite 적재 시간을 별도 계측해야 한다.

## Context for Resuming
### Important Context
카탈로그 파일명은 호환성을 위해 `file-catalog-v1.sqlite3`를 유지하지만 내부 `PRAGMA user_version`과 메타 스키마는 v2다. `windowsNtfs`와 `portableWalk` 모두 같은 검색 API와 FTS5 결과 계약을 사용한다. 앱 밖 변경은 자동 감시하지 않으며 사용자가 `카탈로그 업데이트`를 실행할 때 USN이 재생된다.

### Potential Gotchas
- 원시 MFT 시험은 관리자 권한이 필요하며 일반 사용자 실행에서는 정상적으로 `portableWalk`가 선택돼야 한다.
- MFT 스캔 시작 전에 저장한 USN 위치를 다음 갱신에서 재생하므로 스캔 중 생긴 변경을 잃지 않지만, 최초 완료 직후에는 다음 업데이트까지 해당 변경이 보이지 않을 수 있다.
- 전체 적재 중 내린 SQLite 인덱스와 트리거는 트랜잭션 안에 있어야 하며, 새 조기 반환 경로를 추가할 때 롤백 검사를 유지해야 한다.
- 실제 종단간 시험은 쓰기 가능한 NTFS 볼륨 루트에 임시 파일을 만들므로 `--ignored`와 단일 테스트 스레드를 유지한다.
