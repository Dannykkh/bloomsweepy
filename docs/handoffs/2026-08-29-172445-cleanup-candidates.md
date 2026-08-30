# Handoff: 정리 후보와 중복 파일 검토 흐름

## Session Metadata
- Created: 2026-08-29T17:24:45+09:00
- Project: `<repository-root>`
- Branch: main

## Current State Summary
기존 정확 중복 엔진과 Windows 설치 앱 인벤토리를 재사용해 파일 검토 흐름을 확장했다. 중복 목록은 사진 필터와 서로 다른 폴더 안내를 제공하고, 파일 행은 더블클릭으로 기본 앱에서 열되 실행 형식은 탐색기 위치만 표시한다. 새 정리 후보 화면은 Temp, AppData, 깨진 제거 레지스트리를 근거 등급별로 읽기 전용 분석하며 실제 삭제는 계속 잠겨 있다.

## Work Completed
- [x] 결과 파일 더블클릭 열기와 실행 파일·스크립트 보호
- [x] 정확히 동일한 사진 필터와 서로 다른 폴더의 중복 그룹 안내
- [x] 7일 이상 오래된 사용자 Temp 후보 분석
- [x] 90일 이상 변경되지 않고 설치 앱 이름과 맞지 않는 AppData 검토 후보 분석
- [x] 설치 위치·아이콘·제거 프로그램 중 경로 증거가 둘 이상 끊긴 Windows 제거 정보 탐지
- [x] 위치별 후보 상한으로 Temp 편향 방지
- [x] 정리 후보 Tauri 명령, 진행·취소, React 화면 연결
- [x] Windows 실제 스캔, 탐색기 위치 표시, 설치 패키지 검증

### Files Modified
| File | Changes |
|------|---------|
| crates/bloomsweepy-core/src/cleanup.rs | 정리 위치 계약, 나이·용량·설치 앱 대조, 위치별 상한 및 테스트 |
| crates/bloomsweepy-core/src/lib.rs | 정리 후보 코어 계약 공개 |
| apps/desktop/src-tauri/src/system_inventory.rs | 깨진 제거 프로그램 경로 증거 분석 |
| apps/desktop/src-tauri/src/lib.rs | 정리 후보 명령, 플랫폼 위치 구성, 진행 이벤트 |
| apps/desktop/src-tauri/Cargo.toml | OS 캐시 위치 확인용 dirs 의존성 연결 |
| apps/desktop/src-tauri/capabilities/default.json | 기본 앱 파일 열기 권한 추가 |
| apps/desktop/src/views/CleanupView.tsx | 정리 후보 요약, 필터, 근거 목록, 레지스트리 검토 UI |
| apps/desktop/src/views/DuplicatesView.tsx | 동일 사진 필터와 다른 폴더 안내 |
| apps/desktop/src/components/FileTable.tsx | 더블클릭 및 Enter 파일 확인 |
| apps/desktop/src/components/AppShell.tsx | 정리 후보 내비게이션과 결과 배지 |
| apps/desktop/src/App.tsx | 정리 스캔 상태, 취소, 화면 조합 |
| apps/desktop/src/lib/bridge.ts | 정리 IPC, 파일 열기 및 탐색기 표시 |
| apps/desktop/src/types.ts | 정리 후보·레지스트리·진행 모델 |
| apps/desktop/src/App.css | 정리 후보와 중복 필터 반응형 스타일 |
| README.md | 구현 범위와 판정 경계 문서화 |
| apps/desktop/README.md | 정리 후보 안전 계약 문서화 |
| docs/architecture/cross-platform-desktop.md | 정리 후보 파이프라인 추가 |

### Decisions Made
| Decision | Rationale |
|----------|-----------|
| AppData 불일치를 항상 `검토 필요`로 표시 | Store 앱, 포터블 앱, 계정 데이터 등은 Uninstall 레지스트리에 없을 수 있음 |
| 레지스트리는 경로 증거 두 개 이상이 끊긴 경우만 표시 | 이름이나 빈 설치 위치 하나만으로 삭제된 앱이라고 단정하지 않기 위함 |
| 후보 상한을 위치별로도 적용 | Temp가 많은 컴퓨터에서 AppData 분석이 전역 한도를 먼저 소진하는 문제 방지 |
| 실행 형식은 열지 않고 탐색기에서 표시 | 결과 확인 동작이 실행 파일이나 스크립트를 시작하지 않도록 하기 위함 |
| 동일 사진만 현재 범위에 포함 | 지각적으로 비슷한 사진은 별도 알고리즘과 오탐 검토 UX가 필요함 |

## Verification
- Rust 포맷 검사 통과
- Rust 테스트 14개 통과
- Clippy workspace all-targets `-D warnings` 통과
- TypeScript 검사와 Vite 프로덕션 빌드 통과
- Windows 런타임에서 854ms 동안 Temp 150개, AppData 검토 73개, 후보 2.4GB 분류 확인
- AppData 후보 더블클릭으로 해당 폴더의 탐색기 선택 확인
- MSI와 NSIS 설치 프로그램 재생성

## Pending Work
### Immediate Next Steps
1. 휴지통 어댑터, 삭제 직전 재검증, 작업 저널, 복구 UX를 함께 설계·구현한다.
2. 중복 그룹에서 보존본 하나를 강제하는 선택 및 삭제 미리보기를 추가한다.
3. AppData 후보에 실행 프로세스, Microsoft Store 앱, 서비스·시작 프로그램 소유 증거를 추가한다.
4. 유사 사진은 지각 해시와 썸네일 비교 화면을 별도 기능으로 추가한다.
5. CLI provider에는 메타데이터 기반 `analyze_folder` 계약부터 연결하고 삭제 권한은 주지 않는다.

### Blockers/Open Questions
- [ ] Windows 휴지통 항목을 앱에서 되돌리는 복구 계약 결정
- [ ] macOS 앱 인벤토리와 실제 런너에서 캐시 후보 동작 검증
- [ ] 유사 사진 오탐 임계값과 대표 사진 보존 규칙 결정

## Context for Resuming
### Important Context
현재 `정리 가능성 높음`도 삭제 승인이 아니라 추천 등급이다. Temp는 7일, AppData는 90일의 가장 최근 하위 항목 수정 시각을 기준으로 하며, 링크는 따라가지 않는다. Windows 레지스트리는 Uninstall 키만 읽고 임의의 HKCU\Software 키를 고아 항목으로 추정하지 않는다.

### Potential Gotchas
- Temp 후보가 수백 개면 전역 상한만 사용할 때 AppData 스캔이 누락된다. `max_candidates_per_root`를 유지해야 한다.
- AppData 이름이 설치 앱과 맞지 않는 것은 삭제 증거가 아니다. 포터블·Store 앱과 공유 데이터가 존재한다.
- Tauri `opener:default`에는 `open_path`가 포함되지 않으므로 파일 열기에는 `opener:allow-open-path`가 별도로 필요하다.
- 작업 트리는 이전부터 대부분 미추적 상태였으므로 정리나 리셋을 하지 않는다.
