# Handoff: 스캔 프리징·잠금·자원 누수 안전화

## Session Metadata
- Created: 2026-08-29 18:28:59 +09:00
- Project: `<repository-root>`
- Branch: main

## Current State Summary
파일 검색이 UI를 멈추거나 다른 프로그램의 파일 사용을 막는 경로를 감사하고, Rust 스캔 코어와 Tauri 작업 경계에 메모리·스레드·취소·파일 변경 안전 계약을 구현했다. Windows 잠금 회귀와 20회 반복 스캔 자원 계측, 전체 Rust·TypeScript 검증, 릴리스 MSI/NSIS 빌드 및 실행 파일 기동까지 통과했다.

## Work Completed
- [x] Windows 공유 읽기 핸들 및 독점 잠금 파일 건너뛰기 구현·테스트
- [x] 큰 파일 해시·바이트 비교의 256KiB 청크 취소와 변경 중 파일 배제
- [x] 순회 4개·전체 해시 2개 작업자 제한 및 후보 묶음 병합
- [x] 트리맵 직계 항목·드라이브 위치·하드링크 식별자 내부 메모리 상한
- [x] 정리 후보 직계 항목 스트리밍과 숨김 항목 포함
- [x] Tauri 레지스트리·볼륨 조회 블로킹 격리와 작업자 수명 기반 상태 잠금 해제
- [x] Windows 프로세스 핸들·스레드·private bytes 반복 계측
- [x] 런타임 위험 계약 문서와 UI 상한 경고 필드 추가

### Files Modified
| File | Changes |
|------|---------|
| crates/bloomsweepy-core/src/lib.rs | 공유 핸들, 변경 감지, 청크 취소, 제한된 작업자·후보 묶음·대형 파일 집계 |
| crates/bloomsweepy-core/src/directory.rs | 빈 폴더 스트리밍 판정, 직계 항목 집계 상한 |
| crates/bloomsweepy-core/src/drive.rs | 위치 집계·하드링크 상한, 숨김 항목 포함 |
| crates/bloomsweepy-core/src/cleanup.rs | 직계 후보 스트리밍, 직렬 하위 순회와 설정 상한 |
| crates/bloomsweepy-core/tests/windows_resource_stability.rs | 20회 반복 핸들·스레드·메모리 회귀 |
| apps/desktop/src-tauri/src/lib.rs | 블로킹 격리, 작업자 완료 가드, 동시 스캔 테스트 |
| apps/desktop/src/types.ts | 안전 상한 도달 플래그 |
| apps/desktop/src/components/DriveStoragePanel.tsx | 드라이브 집계 상한 표시 |
| apps/desktop/src/components/StorageTreemapPanel.tsx | 트리맵 집계 상한 표시 |
| apps/desktop/src/views/DuplicatesView.tsx | 하드링크 분석 상한 경고 |
| docs/architecture/scan-runtime-safety.md | 위험 모델, 검증값, 잔여 한계 |

### Decisions Made
| Decision | Rationale |
|----------|-----------|
| 파일을 독점 잠그지 않고 Windows 공유 읽기·쓰기·삭제를 허용 | 분석 때문에 실행 중 앱의 저장·이름 변경·삭제를 막지 않기 위해 |
| 전체 해시는 최대 2개 파일만 동시 읽기 | HDD 탐색 경합과 청크 버퍼 메모리를 제한하면서 병렬 이득 유지 |
| 개별 위치 목록은 상한 뒤 합계만 계속 계산 | 전체 드라이브에서 경로 맵의 무제한 성장을 막고 불완전성을 명시하기 위해 |
| 취소는 항목·256KiB 청크·후보 묶음 경계의 협력적 방식 | UI 반응성을 유지하되 이식 불가능한 커널 I/O 강제 중단을 피하기 위해 |

## Pending Work
### Immediate Next Steps
1. 실제 사용자 시스템 드라이브와 대형 AppData에서 장시간 I/O·취소 지연·최대 메모리를 계측한다.
2. macOS 러너에서 APFS 잠금·하드링크·대용량 순회와 앱 번들을 회귀 검증한다.
3. 삭제 기능 구현 전 파일 ID·크기·수정 시각·해시 재검증, 휴지통·작업 저널·부분 실패 복구 계약을 먼저 만든다.

### Blockers/Open Questions
- [ ] 느린 UNC·외장 드라이브의 단일 커널 I/O 호출은 현재 취소 플래그로 강제 중단할 수 없음
- [ ] 위치 집계 상한 도달 뒤 정확한 상위 위치를 얻으려면 MFT/USN 또는 별도 다단계 알고리즘 필요
- [ ] macOS 실제 러너 검증 필요

## Context for Resuming
### Important Context
`docs/architecture/scan-runtime-safety.md`가 안전 계약의 정본이다. Windows 합성 fixture 20회 반복 결과는 핸들 158→158, 스레드 3→3, private bytes 3,579,904→3,338,240였다. 릴리스 번들은 MSI SHA256 `86E2270F888D36652E7F8598CE9EADEE6FC2773C25EE042811B67CDF0BDF64A7`, NSIS SHA256 `0F82507DB33BA0F164F65CCD159C75E41451AC843DF263187EE8EC62ECAB1EC6`다.

### Potential Gotchas
- OS·파일시스템 드라이버 안에서 진행 중인 한 번의 `open`·`read_dir`·`metadata`·`read`는 협력적 취소로 끊을 수 없다.
- 보고서가 만들어진 뒤 파일은 다시 변경될 수 있으므로 미래 삭제 명령은 반드시 실행 직전 재검증해야 한다.
- `trackingLimitReached`와 `locationTrackingLimitReached`가 참이면 합계는 유지되지만 개별 위치 목록은 완전하지 않다.
