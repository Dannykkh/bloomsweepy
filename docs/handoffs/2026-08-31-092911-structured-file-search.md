# Handoff: 구조화 빠른 파일 검색과 실제 WebView 응답성 검증

## Session Metadata
- Created: 2026-08-31 09:29:11 +09:00
- Project: `D:\git\bloomsweepy`
- Branch: `main`

## Current State Summary
문서·파일 색인 writer와 완료 스냅샷 reader를 분리하고 확정 단계 취소를 보강한 뒤, 실제 릴리스 Tauri/WebView2에서 색인·상태 조회·화면 전환·취소·완주를 검증했다. 이어 기존 파일 카탈로그와 SQLite FTS composition point에 구조화 쿼리 파서를 추가하고 React 검색창 안내, 시간대 전달, 오류 회복까지 종단간 확인했다.

## Work Completed
- [x] 활성 writer 중 완료 스냅샷 상태·검색과 취소 제어 회귀 테스트 추가
- [x] `ext:`·`type:`·`path:`·`size:`·`after:`·`before:`·따옴표·제외어 파서와 SQL 교집합 적용
- [x] 구조화 쿼리 도움말과 접근성 연결을 빠른 파일 찾기 화면에 추가
- [x] 34,341개 파일 fixture의 실제 릴리스 WebView 취소·완주 E2E 및 37,440개 카탈로그 검색 E2E 통과
- [x] Rust 전체 테스트, clippy, TypeScript, Vite, 정식 MSI·NSIS 패키징 통과

### Files Modified
| File | Changes |
|------|---------|
| `crates/bloomsweepy-core/src/file_catalog/query.rs` | 제한된 구조화 쿼리 토큰화·검증·크기·날짜 파싱 |
| `crates/bloomsweepy-core/src/file_catalog.rs` | 파서 결과를 FTS·LIKE·메타데이터 SQL 조건과 결합 |
| `crates/bloomsweepy-core/src/document_search.rs` | 완료 스냅샷용 reader 연결과 확정 단계 취소 |
| `crates/bloomsweepy-core/tests/index_responsiveness.rs` | 활성 writer 중 상태·검색·취소 응답성 회귀 |
| `apps/desktop/src/views/FastFileSearchView.tsx` | 한 줄 검색 문법 안내와 PC 시간대 오프셋 전달 |
| `apps/desktop/src/App.css` | 문법 도움말의 밀도 높은 반응형 표현 |
| `docs/architecture/fast-file-search.md` | 문법·보안 경계·실측 E2E 기록 |
| `docs/architecture/scan-runtime-safety.md` | reader/writer 및 실제 WebView 응답성 기록 |

### Decisions Made
| Decision | Rationale |
|----------|-----------|
| 기존 SQLite FTS 카탈로그 안에서 파싱 | 같은 역할의 새 검색 엔진과 캐시를 만들지 않음 |
| 사용자 값은 모두 SQL 매개변수로 전달 | SQL·셸 주입 경계를 단순하고 검증 가능하게 유지 |
| 한 줄 조건과 UI 필터는 교집합 | 검색창이 화면 필터를 몰래 덮어쓰지 않게 함 |
| 제외 조건만 있는 쿼리는 거절 | 최대 2백만 항목의 불필요한 전수 부정 검색 방지 |
| 원격 디버깅은 무시되는 E2E 설정에만 사용 | 배포 바이너리에 디버그 포트를 남기지 않음 |

## Pending Work
### Immediate Next Steps
1. 현재 AND 문법 위에 비용이 제한된 `OR`와 glob을 추가하고 같은 256자·32토큰 상한에서 성능 회귀한다.
2. 쿼리 빌더 UI가 필요한지 실제 사용자 입력 패턴으로 판단하고 날짜 선택 UI를 검토한다.
3. 이후 Windows 권한 서비스 또는 설치된 Everything 선택 연동 중 우선순위를 정한다.

### Blockers/Open Questions
- [ ] OR가 텍스트 그룹에만 적용될지 구조화 필터 그룹까지 포함할지 제품 의미 결정
- [ ] glob의 대소문자·경로 구분자·점 파일 규칙 결정

## Context for Resuming
### Important Context
정식 제품 설정에는 `additionalBrowserArgs`가 없다. E2E 설정과 CDP 스크립트, 45.4MB 시험 카탈로그는 Git에서 제외된 `.termsnap/runtime-e2e/` 아래에 보관했다. 사용자 앱 캐시에는 기존 `EBWebView`만 남겼고 BroomSweepy 프로세스와 9334 포트는 모두 종료 상태다.

### Potential Gotchas
- `after:`·`before:` 날짜는 `timezoneOffsetMinutes`가 빠진 구버전 호출에서 UTC로 해석되고, 현재 React UI는 `Date.getTimezoneOffset()`을 항상 전달한다.
- 세 글자 이상 포함 조건은 FTS trigram, 한두 글자와 제외 조건은 이스케이프한 `LIKE`이므로 OR 설계에서 두 경로의 의미와 성능을 함께 보존해야 한다.
- CodeMap 생성기가 일부 파일 끝에 빈 줄을 하나 더 넣어 `git diff --check`가 해당 자동 생성 파일만 경고할 수 있다.
