# Implementation Log: 최소 14px 타이포

## Source Mode

- Mode: delta
- User decision: "글자들이 전체적으로 좀 작은데 최소 14pt이상으로 해볼까?"
- 적용 해석: `14pt`(약 18.7px)가 아니라 데스크톱 앱의 사용자 표시 글자 최소 `14 CSS px`로 적용했다. 사용자가 원한 전역 가독성을 확보하면서 네이티브 최소 창 760×600의 정보 구조를 보존하기 위한 해석이다.
- 기존 정본: `DESIGN.md`, `2026-08-29-experience-tauri-port.md`, `2026-08-31-experience-fast-file-layout.md`.

## Direction and Scope

기존 `existing-dark-glass` Data Instrument 방향, 화면 구조, 색, 카피, 기능은 보존했다. 이번 변경은 한 가지 변수인 글자 하한만 조정하는 승인된 디자인의 국소 변경이므로 3방향 후보와 새 사이트맵을 만들지 않았다.

계약은 다음과 같다.

- 라벨, 설명, 경로, 표 셀, 버튼 등 사용자에게 보이는 글자는 14px 미만으로 줄이지 않는다.
- 제목과 중요 수치처럼 이미 15px 이상인 계층은 유지한다.
- 좁은 창에서는 글자를 다시 줄이지 않고 줄바꿈, 재배치, 세로 스크롤로 공간을 확보한다.
- 색, 글래스 재질, 저장공간 링, 내비게이션 구조, 모션 예산은 변경하지 않는다.

## Changes

- `DESIGN.md`: metric 크기를 `0.8125rem`에서 `0.875rem`으로 올리고 사용자 표시 글자 최소 14px 규칙을 추가했다.
- `apps/desktop/src/App.css`: `--font-size-ui-min: 14px` 토큰을 추가하고 기존 7–13px 선언 166곳을 같은 토큰으로 통일했다.
- 기존 15–27px 제목·강조 계층과 responsive layout은 그대로 유지했다.

## Render Critique

### 1280×820

- 대시보드의 제목, 상태 배너, 채팅 CLI 권한 설명, 드라이브 분류가 이전보다 쉽게 읽힌다.
- 216px 사이드바의 보조 문구는 의도된 말줄임을 유지하며 메인 영역 가로 넘침은 없다.
- 증거: `.termsnap/runtime-e2e/screenshots/control-scan-type14-1280.png`.

### Native minimum 760×600

- 실제 WebView client 744×561에서 72px 아이콘 레일과 메인 세로 스크롤을 유지한다.
- 대시보드, 빠른 파일 찾기, 문서 검색, 정리 후보, 중복 파일, 설정에서 계산된 최소 표시 글자 크기는 모두 14px이며 메인 가로 넘침은 0이다.
- 채팅 검사 허용 버튼은 첫 화면 아래에 있을 때 메인 스크롤 `0 → 324px` 이동 후 키보드 Enter로 해제·재허용할 수 있고, 터치 목표 높이는 44px다.
- 증거: `.termsnap/runtime-e2e/screenshots/control-scan-type14-final-760.png`, `file-search-type14-760.png`, `document-search-type14-760.png`, `cleanup-type14-760.png`, `duplicates-type14-760.png`, `settings-type14-760.png`.

## Verification

- `npm run check`: 통과.
- `npm run build`: 통과.
- `cargo fmt --all -- --check`: 통과.
- `cargo test --workspace`: 106개 통과, 실제 관리자 권한·휴지통 시험 5개 ignored, 실패 0개.
- `cargo clippy --workspace --all-targets -- -D warnings`: 통과.
- 실제 Tauri + CLI 검사: 기본 거부, 승인, 240ms 안의 작업 번호 반환, 잘못된 작업 번호 취소 거부, 완료 결과 UI 반영, 정확한 작업 번호 취소, 다른 화면의 진행 dock과 종료 안내까지 통과.
- 1280×820과 760×600 모두 `main.scrollWidth === main.clientWidth`.
- 진행률을 여러 live region에서 반복 낭독하지 않는다. 대시보드 밖의 완료·취소는 하나의 숨은 `status`, 실패는 실제 오류 원인을 포함한 별도 `alert`로 알리며 E2E에서 둘을 구분해 확인했다.

## Quality Review

- Independent UI/accessibility review: P0–P3 잔여 결함 없음. 완료·취소 `status`, 실패 `alert`, 실제 오류 상세와 실패 E2E를 재검토했다.
- Product Design Gate: `UNKNOWN`. `openai-curated` marketplace와 CLI 조회는 가능했지만 설치·가용 목록에서 정확한 Product Design selector를 확인하지 못했다. 설치를 추측하거나 권하지 않고 기존 local adapter를 사용했다.
- Adapter comparison: `NOT RUN`. 새 adapter 도입이나 비교 요청이 아닌 기존 구현의 단일 변수 delta다.
- Web Motion Contract: `NOT APPLICABLE`. 모션을 추가하거나 바꾸지 않았다.
- 남은 관찰 범위: Windows 125%·150% 배율과 macOS WKWebView는 이번 로컬 100% Windows 실행에서 직접 관찰하지 않았다.

## Module Coverage

| Module | Resolved path | Coverage |
|---|---|---|
| `frontend-design` | `C:/Users/Administrator/.codex/.olympus/source-skills/frontend-design/SKILL.md` | 전체 읽기, Data Instrument 타이포·밀도 계약과 coder interface playbook 적용 |
| `mermaid-diagrams` | `C:/Users/Administrator/.codex/.olympus/source-skills/mermaid-diagrams/SKILL.md` | 전체 읽기, IA·흐름 변경이 없어 새 다이어그램은 불필요 |
| `ui-ux-auditor` | `C:/Users/Administrator/.codex/.olympus/source-skills/ui-ux-auditor/SKILL.md` | 전체 읽기, Typography·Responsive·Accessibility 규칙과 실제 스크린샷 대조 |
| `web-design-guidelines` | `C:/Users/Administrator/.codex/.olympus/source-skills/web-design-guidelines/SKILL.md` | 전체 읽기, 2026-09-01 최신 원문을 가져와 포커스·타이포·overflow·dark control 규칙 대조 |
