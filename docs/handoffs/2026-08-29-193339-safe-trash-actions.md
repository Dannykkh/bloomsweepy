# Handoff: 서버 재검증 기반 안전한 휴지통 작업

## Session Metadata
- Created: 2026-08-29 19:33:39 +09:00
- Project: `<repository-root>`
- Branch: main

## Current State Summary
중복 파일과 Temp·캐시·AppData 후보를 선택해 Windows/macOS 운영체제 휴지통으로 보내는 경계를 구현했다. React가 보낸 경로를 신뢰하지 않고 Tauri가 보관한 최소 스캔 스냅샷으로 전체 선택을 사전검사한 뒤, JSONL 계획을 디스크에 동기화하고 항목별 재검사와 휴지통 이동을 수행한다. Rust·TypeScript 전체 검사, 부분 실패·취소·저널 회전 테스트, 실제 Windows 휴지통 스모크, MSI/NSIS 빌드와 릴리스 실행 스모크를 통과했다.

## Work Completed
- [x] 중복 파일의 루트 경계·파일 ID·하드링크·크기·수정시각·전체 BLAKE3 해시 재검증
- [x] 중복 그룹마다 보관본 한 개를 서버와 UI에서 강제하고 매 이동 전 보관본도 재검증
- [x] 정리 후보 파일·폴더의 항목 수·논리 용량·최종 수정시각·구조 지문 재검증
- [x] Windows junction·심볼릭 링크·드라이브/사용자 기준 폴더 거부
- [x] 최대 500개, 단일 작업 잠금, 청크 취소, 전체 사전검사 후 항목별 OS 휴지통 이동
- [x] `sync_data` JSONL 저널, 첫 실패 중단, 이동·실패·건너뜀 결과, 8 MiB 저널 회전
- [x] 중복·정리 후보 체크박스, 확인 대화상자, AppData 별도 확인, 진행·취소·결과 UI
- [x] 영구 삭제·휴지통 비우기·레지스트리 변경 제외와 실제 공간 반영 시점 안내
- [x] Windows MSI·NSIS 번들 및 릴리스 실행 스모크

### Files Modified
| File | Changes |
|------|---------|
| crates/bloomsweepy-core/src/actions.rs | 파일·폴더 실행 직전 검증 계약과 회귀 테스트 |
| crates/bloomsweepy-core/src/lib.rs | 파일 객체 ID·링크 수 조회와 action API 노출 |
| apps/desktop/src-tauri/src/trash_actions.rs | OS 휴지통 어댑터, 저널, 취소·부분 실패 명령과 테스트 |
| apps/desktop/src-tauri/src/lib.rs | 단일 작업 런타임과 서버 최소 보고서 스냅샷 |
| apps/desktop/src/{App.tsx,types.ts,lib/bridge.ts} | 휴지통 명령·진행·결과 상태 연결 |
| apps/desktop/src/components/{FileTable,SafetyActionDialog,TrashResultPanel}.tsx | 선택, 안전 확인, 결과 UI |
| apps/desktop/src/views/{DuplicatesView,CleanupView}.tsx | 중복 보관본·AppData 확인을 포함한 실행 흐름 |
| apps/desktop/src/App.css | 기존 글래스 토큰 기반 선택·대화상자·결과 반응형 스타일 |
| README.md, docs/architecture/*.md | 현재 안전 경계, TOCTOU·크래시 한계, 검증 근거 |
| MEMORY.md, memory/architecture.md | 장기 설계 결정 인덱스 |

### Decisions Made
| Decision | Rationale |
|----------|-----------|
| 영구 삭제 대신 `trash` 5.2.6의 OS 휴지통만 사용 | 사용자 복구 가능성을 기본값으로 유지 |
| 클라이언트 값 대신 서버의 최소 실행 스냅샷 사용 | 경로·크기·해시 변조와 전체 보고서 복제로 인한 피크 메모리 방지 |
| 전체 사전검사 후 항목별 재검사·이동 | 일부 파일을 옮긴 뒤 뒤 항목이 이미 바뀐 상황을 줄임 |
| 첫 실패에서 뒤 항목 중단 | 부분 실패 범위를 최소화하고 결과를 예측 가능하게 유지 |
| 레지스트리는 계속 읽기 전용 | 제거 정보만으로 안전한 키 소유권을 확정할 수 없음 |
| 폴더는 내용 전체 해시 대신 구조 지문 | 대용량 Temp·AppData를 두 번 전체 해시하는 I/O 폭주 방지 |

## Pending Work
### Immediate Next Steps
1. 실제 macOS APFS 앱 번들에서 휴지통 이동·권한·복원 회귀 테스트
2. `moving` 뒤 비정상 종료된 저널을 OS 휴지통과 대조하는 시작 시 복구 화면 설계
3. 설치본 UI에서 선택→확인→취소→부분 실패를 자동화하는 WebView/Tauri 통합 테스트 추가
4. 이후 별도 기능으로 문서 내용 검색 인덱스와 유사 사진 탐지 범위 설계

### Blockers/Open Questions
- [ ] 앱 내부 복원 버튼을 제공할지, 운영체제 휴지통 열기로만 유지할지 제품 결정
- [ ] macOS 샌드박스·공증 환경에서 외부 경로 휴지통 권한 확인

## Context for Resuming
### Important Context
Windows 실제 휴지통 테스트는 `bloomsweepy-trash-smoke-...txt` 한 개를 휴지통에 남겼으며 휴지통은 비우지 않았다. 최종 산출물은 `target/release/bundle/msi/BroomSweepy_0.1.0_x64_en-US.msi`와 `target/release/bundle/nsis/BroomSweepy_0.1.0_x64-setup.exe`다. 표준 전체 테스트에서는 실제 휴지통 테스트가 `ignored`이고 명시 실행에서만 동작한다.

### Potential Gotchas
- 최종 재검사와 경로 기반 OS 휴지통 호출 사이의 짧은 TOCTOU는 완전히 제거할 수 없다.
- 폴더 구조 지문은 하위 파일 내용을 모두 해시하지 않아 동일 크기·수정시각을 의도적으로 보존한 변경을 탐지하지 못할 수 있다.
- 크래시가 OS 이동 뒤 `moved` 저널 기록 전에 나면 `moving` 상태가 모호하게 남으며 자동 대조는 아직 없다.
- 휴지통 이동 용량은 즉시 회수된 공간이 아니고 휴지통을 비운 뒤 실제 여유 공간이 늘어난다.
- 이 셸의 PATH에는 Cargo가 없어 Tauri 빌드 전에 `%USERPROFILE%\.cargo\bin`을 명령 범위 PATH에 추가해야 한다.
- 저장소 기준선 대부분이 아직 untracked이고 `.gitignore`에는 사용자 변경이 있으므로 일괄 정리·리셋하지 않는다.
