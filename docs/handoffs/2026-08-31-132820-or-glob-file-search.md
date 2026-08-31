# Handoff: 빠른 파일 찾기 OR 및 glob 검색

## Session Metadata

- Created: 2026-08-31 13:28:20 +09:00
- Project: D:\git\bloomsweepy
- Branch: main

## Current State Summary
빠른 파일 찾기에 평면 `OR` 분기와 파일명 전용 `glob:` 검색을 추가했다. Rust가 구문을 파싱하고 SQLite FTS5로 후보를 먼저 제한한 뒤 매개변수화된 `LIKE`로 glob을 확인한다. 기존 확장자·종류·크기·날짜·제외 조건과 화면 필터는 `OR` 밖의 전역 교집합으로 유지된다. 전체 회귀, 최소 화면 실제 WebView2 E2E, 정식 릴리스 실행과 패키징까지 통과했다.

## Work Completed

- [x] 최대 8개의 대안 분기를 지원하는 대소문자 비구분 `OR` 파서 추가
- [x] 파일명에만 적용되는 `*`·`?` glob과 `-glob:` 제외 조건 추가
- [x] 각 `OR` 분기와 양의 glob에 세 글자 FTS 앵커를 강제해 전체 카탈로그 스캔 방지
- [x] 단일 조건의 BM25 경로를 보존하고 다중 분기는 제한된 FTS 서브쿼리로 실행
- [x] UI 검색 예시, README, 아키텍처 문법·성능 계약 갱신
- [x] Rust 66개 테스트, Clippy, TypeScript, Vite, 실제 Tauri/WebView2, MSI·NSIS 검증

### Files Modified

| File | Changes |
|------|---------|
| `crates/bloomsweepy-core/src/file_catalog/query.rs` | `OR` 그룹과 glob 파싱·검증·상한 |
| `crates/bloomsweepy-core/src/file_catalog.rs` | 분기별 FTS/LIKE SQL, 제외 glob, 결과 출처 판정과 통합 테스트 |
| `apps/desktop/src/views/FastFileSearchView.tsx` | `OR`·glob 설명, 예시, 문법 칩 |
| `README.md` | 구현 기능과 전역 필터 의미 |
| `docs/architecture/fast-file-search.md` | 문법·우선순위·성능 경계·실제 E2E 수치 |
| `codemap/*` | 변경된 Rust 함수와 UI 줄 위치 자동 색인 |

### Decisions Made

| Decision | Rationale |
|----------|-----------|
| `OR`는 이름 단어·`path:`·`glob:`만 묶는다 | 크기·날짜·종류 같은 필터의 의미를 예측 가능하게 유지한다 |
| 괄호·중첩 없이 최대 8분기만 허용한다 | 동적 SQL 크기와 FTS 실행 비용을 제한한다 |
| 각 분기에 세 글자 FTS 앵커를 요구한다 | `LIKE` 중심의 전체 카탈로그 반복 스캔을 막는다 |
| glob은 파일명 전체에만 적용한다 | 경로 재귀 패턴의 모호함과 비용을 피한다 |
| 다중 분기 관련도는 안정적인 경로·이름 순서로 폴백한다 | 여러 FTS 서브쿼리의 BM25 점수를 억지로 합산하지 않는다 |

## Pending Work

### Immediate Next Steps

1. 기존 `after:`·`before:`를 직접 조작하지 않아도 되는 날짜 선택 UI를 검토한다.
2. regex가 필요하면 별도 비용 상한과 명시적 opt-in을 먼저 설계한다.
3. Windows Everything 선택 공급자 또는 macOS FSEvents 증분 공급자 중 다음 플랫폼 가속 단계를 정한다.

### Blockers/Open Questions

- [ ] 다음 우선순위를 날짜 UI, Windows 선택 공급자, macOS 증분 공급자 중에서 결정해야 한다.

## Context for Resuming

### Important Context

- 예시 `invoice OR receipt ext:pdf -draft`는 PDF이며 draft가 아니고 invoice 또는 receipt에 일치하는 결과다.
- `glob:report-*.pdf`는 이름 전체에만 적용하고 `/`와 `\`를 거절한다. 양의 glob에는 세 글자 이상의 고정 문자열이 있어야 한다.
- 37,440개 실제 카탈로그에서 두 glob의 `OR` 검색은 SQLite 3ms, 140ms 디바운스와 렌더 포함 약 180ms였다. 오류 후 정상 검색 회복은 약 179ms였다.
- 760×600 실제 WebView2에서 주 콘텐츠는 세로 `600 → 642px`, 가로 `678 → 678px`였고 런타임 예외가 없었다.

### Potential Gotchas

- `OR` 분기 안에 `ext:`만 두면 거절된다. 확장자 목록은 `ext:pdf,docx`처럼 전역 필터로 표현한다.
- 따옴표 없는 `OR`만 연산자다. 파일명에서 문자 OR 자체를 찾으려면 `"OR"`로 검색한다.
- 여러 `OR` 분기에서는 BM25 대신 경로 길이와 이름의 안정 정렬을 사용한다.
- 실제 E2E용 디버깅 설정과 카탈로그는 `.termsnap/runtime-e2e`에만 있으며 정식 `tauri.conf.json`에는 원격 디버깅 인수가 없다.
