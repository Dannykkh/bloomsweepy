# Handoff: 네이티브 화면 구조와 쉬운 용어 정리

## Session Metadata
- Created: 2026-08-31 15:30:35 +09:00
- Project: `D:\git\bloomsweepy`
- Branch: `main`

## Current State Summary
사용자 캡처의 오른쪽·아래 검은 빈 영역은 제품 CSS가 아니라 E2E가 WebView만 760×600으로 줄인 결과였다. CDP 크기 강제를 해제하고 네이티브 창을 직접 리사이즈하는 검증으로 교체했으며, 기본 1280×820과 최소 760×600에서 앱 셸·사이드바·세로 스크롤을 확인했다. 이어서 사용자 피드백에 따라 빠른 파일 찾기뿐 아니라 문서 검색, 중복, 정리, 용량 지도, 안전 작업 화면의 개발자 용어를 쉬운 한국어로 바꾸고 고급 검색 문법은 접힌 도움말로 이동했다.

## Work Completed
- [x] 사용자 캡처와 실제 Tauri 렌더를 비교해 네이티브 창/WebView 크기 불일치 원인 확정
- [x] 로컬 E2E에서 device metrics override 제거와 잔여 override 해제
- [x] 실제 1280×820 및 760×600 네이티브 창 구조화 검색 E2E 통과
- [x] `카탈로그·색인·MFT·USN·해시·바이트` 중심 문구를 행동 중심의 쉬운 한국어로 교체
- [x] `glob·ext·size` 고급 문법을 `검색을 더 정확하게 하는 법` 안으로 이동
- [x] 긴 검색어 줄바꿈, 숫자 정렬, 100개 결과 행의 화면 밖 렌더 비용 제한
- [x] Experience Contract, Layout Blueprint, 벤치마크 근거와 구현 로그 작성·검증
- [x] Rust·TypeScript·Tauri E2E·정식 패키징과 원격 디버깅 비활성 확인

### Files Modified
| File | Changes |
|------|---------|
| `apps/desktop/src/views/FastFileSearchView.tsx` | 쉬운 검색 문구, 접힌 고급 도움말, 한국어 오류 안내, 폼·결과 접근성 보강 |
| `apps/desktop/src/App.css` | 도움말 구조, 긴 제목, 숫자 정렬, 대량 결과 렌더 최적화 |
| `apps/desktop/src/App.tsx`, `components/AppShell.tsx` | 진행 상태·내비게이션·범위 문구 단순화 |
| `apps/desktop/src/views/DocumentSearchView.tsx` | 색인 용어 제거, 문서 미리 읽기와 지원 범위 설명 단순화 |
| `CleanupView.tsx`, `DuplicatesView.tsx`, `OverviewView.tsx`, `LargeFilesView.tsx`, `SettingsView.tsx` | 사용자 노출 기술 용어를 쉬운 설명으로 교체 |
| `apps/desktop/src/components/*.tsx` 관련 5개 파일 | 영문 eyebrow, 논리 용량, AppData·레지스트리 표현 단순화 |
| `crates/bloomsweepy-core/src/file_catalog.rs` | 파일 목록 진행·오류 메시지 한국어 단순화 |
| `crates/bloomsweepy-core/src/file_catalog/windows_ntfs.rs` | Windows 빠른 읽기와 fallback 오류를 쉬운 말로 변환 |
| `crates/bloomsweepy-core/src/document_search.rs`, `actions.rs` | 문서 읽기·중복 확인 메시지 단순화 |
| `docs/design-refs/2026-08-31-*fast-file*` | 화면 근거, 경험 계약, 레이아웃, 구현·검증 기록 |
| `MEMORY.md`, `memory/gotchas.md`, `memory/patterns.md` | 재발 방지 규칙과 쉬운 용어 패턴 기록 |
| `codemap/*` | 변경된 코드 위치 자동 갱신 |

### Decisions Made
| Decision | Rationale |
|----------|-----------|
| 제품 레이아웃을 억지로 재구성하지 않음 | 실제 네이티브 1280×820에서는 기존 `216px sidebar + fluid main` 구조가 전체 창을 정상적으로 채움 |
| 좁은 화면은 네이티브 창 리사이즈로 검증 | WebView 에뮬레이션은 실제 데스크톱 창과 좌표계를 분리해 잘못된 화면을 만듦 |
| 기술 문법은 기본 화면에서 접음 | 일반 사용자는 파일 이름만 입력해도 되고, 고급 사용자는 같은 엔진 기능을 필요할 때 펼쳐 볼 수 있음 |
| 내부 정확성은 유지하고 사용자 문구만 단순화 | MFT·USN·SQLite·BLAKE3 구현은 바꾸지 않고 설명을 행동과 결과 중심으로 번역함 |

## Pending Work
### Immediate Next Steps
1. 사용자에게 최종 쉬운 용어 화면을 확인받는다.
2. 다음 기능은 사용자 우선순위에 따라 최근 검색·즐겨찾기 또는 날짜 선택 UI 중 하나를 진행한다.

### Blockers/Open Questions
- [ ] 쉬운 용어 수준이 충분한지 실제 사용자 확인 필요. 기능·테스트 차단 항목은 없음.

## Context for Resuming
### Important Context
- 로컬 E2E 스크립트와 디버그 설정은 `.termsnap/runtime-e2e/` 아래의 Git 제외 자료다.
- 화면 근거 PNG는 `.termsnap/runtime-e2e/screenshots/fast-file-simple-1280.png`와 `fast-file-simple-760.png`다.
- 정식 실행 파일과 설치본은 `target/release/` 아래에 새로 생성됐다.
- 사용자 AppData 시험 카탈로그, BroomSweepy 프로세스, 9334 디버깅 포트는 남아 있지 않다.

### Potential Gotchas
- E2E 설정으로 release 실행 파일을 빌드하면 마지막에 정식 `tauri build`를 다시 실행해 원격 디버깅 인수가 없는 제품을 복구해야 한다.
- 빈 결과 상태에는 스크롤 범위가 없어도 정상이다. 최소 창에서 콘텐츠가 길어졌을 때만 `scrollHeight > clientHeight`를 요구한다.
- 쉬운 문구로 바꿔도 고급 질의 토큰과 파일 공급자 enum 등 내부 계약 이름은 변경하지 않는다.
