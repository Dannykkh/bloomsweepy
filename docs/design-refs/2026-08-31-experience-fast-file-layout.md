# Experience Contract: Fast File Search 화면 구조

## Source Mode

- Mode: benchmark
- Evidence: 사용자 제공 2026-08-31 캡처, 실제 Tauri 1280×820·최소 창 렌더, `DESIGN.md`, 기존 Tauri 포트 계약, FastFind 0.71.0 벤치마크.

## Product Facts

| Claim | Source | Captured at | Freshness/status | Allowed presentation |
|---|---|---|---|---|
| 파일명·경로·크기·수정 시각만 로컬 검색 목록에 저장한다 | `FastFileSearchView.tsx`, Rust 카탈로그 구현 | 2026-08-31 | current | `파일 목록`이라는 쉬운 이름으로 상태와 결과 근거에 표시 |
| 기준 fixture는 37,440개 항목이며 질의 자체는 6~7ms에 처리됐다 | 실제 Tauri E2E 보고 | 2026-08-31 | test-only | 개발 검증 문서에만 표시하고 일반 제품 수치로 홍보하지 않음 |
| 기본 창의 웹뷰는 1280×820이고 최소 네이티브 리사이즈에서는 744×561로 계측됐다 | CDP `documentElement.clientWidth/clientHeight` | 2026-08-31 | current-test | 화면 구조 검증 근거로만 사용 |

## Benchmark Sources

- `.termsnap/snapshots/clipboard_20260831_132553_958.png` — 사용자 제공 문제 화면, 2026-08-31 캡처.
- `.termsnap/runtime-e2e/screenshots/fast-file-native-1280.png` — 실제 기본 창 비교 렌더, 2026-08-31 캡처.
- `.termsnap/runtime-e2e/screenshots/fast-file-native-760.png` — 네이티브 창 축소 비교 렌더, 2026-08-31 캡처.
- `.termsnap/runtime-e2e/screenshots/fast-file-simple-1280.png`, `fast-file-simple-760.png` — 쉬운 용어와 접힌 고급 검색 도움말의 최종 렌더, 2026-08-31 캡처.
- `docs/design-refs/2026-08-30-benchmark-fastfind-0.71.0.md` — 검색 도구의 정보 위계 참고. 외형과 권한 모델은 복제하지 않음.

## Page Goal

- 사용자가 이 화면에서 달성할 결과: 파일을 열지 않고 이름·경로·조건으로 후보를 찾은 뒤 파일을 열거나 위치를 확인한다.
- 제품이 얻어야 하는 결과: 빠른 검색과 정리 추천의 신뢰 경계를 분리하면서 파일 목록의 새로고침 상태를 이해시킨다.
- 관찰 가능한 성공 조건: 기본 창 전체가 하나의 앱 셸로 채워지고, 최소 창에서도 검색·필터·상태·결과에 세로 스크롤로 도달한다.

## Audience and Tasks

- 주요 사용자와 사용 상황: 저장공간이 갑자기 부족하거나 파일 위치를 잊은 Windows·macOS 사용자.
- 최우선 과업: 범위 확인 → 검색어 입력 → 결과 비교 → 파일 열기 또는 위치 표시.
- 시작 조건과 완료 조건: 검색용 파일 목록이 있으면 즉시 검색하고, 없으면 위치를 선택해 만든 뒤 결과 행동을 완료한다.
- 주요 불안·마찰·실패 가능성: 검색이 파일을 잠글 우려, 오래된 위치 정보, 빠진 파일, 검색 결과를 삭제 추천으로 오해하는 상황.

## Header and Navigation

- 브랜드·현재 위치·전역 이동·주 행동의 순서: 좌측 브랜드와 과업 내비게이션 → 우측 화면 제목과 범위 → 검색 명령.
- 데스크톱 내비게이션: 216px 고정 사이드바에 이름·설명·상태 배지를 표시한다.
- 모바일 대체 구조: 920px 이하에서는 72px 아이콘 레일, 680px 이하에서는 메뉴 버튼과 오버레이 내비게이션으로 교체한다. 고급 검색 도움말은 기본적으로 접힌 상태를 유지한다.

## Core Message

- 핵심 약속: 파일을 열지 않고 이름과 경로를 즉시 찾는다.
- 설명: 기본 화면에서는 쉬운 검색어와 필터를 쓰고, 특수 검색 문법은 접힌 도움말에서만 보여준다.
- 증거: 현재 범위, 항목 수, 갱신 시각, 공급자, 검색 시간, 실제 경로.
- 사용자가 다음에 이해해야 할 것: 검색 결과는 삭제 추천이 아니며 실행 파일은 직접 실행하지 않고 위치만 표시할 수 있다.

## Content Integrity

| Content item | Classification | Evidence | Presentation rule |
|---|---|---|---|
| 파일 목록 수·새로고침 시각·검색 시간 | verified | 런타임 IPC 응답 | 응답이 있을 때만 쉬운 표현으로 표시 |
| MFT·USN 또는 공용 순회 공급자 | verified | Rust 공급자 결과 | 실제 사용된 공급자만 표시 |
| 질의 문법 예시 | verified | Rust parser 테스트 | 지원되는 토큰만 정적 도움말로 표시 |
| fixture 성능 수치 | prototype | 로컬 E2E 보고 | 사용자 제품 화면에는 고정 수치로 넣지 않음 |

## Section Order

1. 전역 유틸리티 헤더: 현재 화면과 검색 범위를 먼저 확정한다.
2. 검색 명령 영역: 최우선 과업인 질의와 필터를 첫 작업 표면에 둔다.
3. 위치 경고 또는 파일 목록 상태: 검색 가능성과 결과 최신성을 행동 직전에 설명한다.
4. 최근 수집 근거와 오류: 결과 완전성에 영향을 주는 정보만 조건부로 노출한다.
5. 검색 결과: 이름·경로·크기·수정 시각과 행 행동을 비교한다.
6. 방식 설명: 파일 목록이 없을 때만 빠른 읽기와 일반 확인의 차이를 쉬운 말로 설명한다.

## CTA Strategy

- Primary: 파일 목록이 없을 때 `파일 목록 만들기`, 있으면 검색 입력 자체가 주 행동이다.
- Secondary: `찾을 위치 선택`, `파일 목록 새로고침`, 결과 행의 `위치 표시`.
- 반복 규칙: 전역 위치 선택과 파일 목록 위치 선택은 같은 실제 위치를 가리키며, 상태에 맞는 한 곳에서만 강조한다.
- 완료·실패 피드백: 검색 개수·시간, 열기 결과, 오류 원인과 갱신 복구 행동을 `status` 또는 `alert`로 알린다.

## Trust Strategy

- 사용자가 불안을 느끼는 지점: 검색 범위가 맞는지, 결과가 오래됐는지, 파일 내용을 읽거나 잠그는지, 삭제로 이어지는지.
- 그 직전에 제시할 근거: `이 기기 안에서만 검색` 배지, 실제 위치, 새로고침 시각, 읽는 정보 설명, 삭제 추천이 아니라는 문구.
- 출처·날짜·검증 가능성: 런타임 파일 목록 응답과 실제 파일 경로를 표시하며 정적 홍보 수치를 사용하지 않는다.
- 근거가 없을 때 생략할 요소: 항목 수, 완료 시각, 공급자, 검색 시간은 응답이 없으면 추정하지 않는다.

## Asset Provenance

| Asset | Source | Local path | License/trademark/attribution | Modification allowed | Status/fallback |
|---|---|---|---|---|---|
| UI 아이콘 | Lucide React dependency | `apps/desktop/package.json` | ISC | yes | verified |
| Pretendard Variable | 프로젝트 내부 폰트 자산 | `apps/desktop/src/assets/fonts/PretendardVariable.woff2` | 저장소 라이선스 고지 따름 | limited | verified |
| JetBrains Mono | Fontsource package | `@fontsource/jetbrains-mono` | OFL-1.1 | limited | verified |

## Desktop Structure

- 기준 뷰포트: 1280×820 Tauri 기본 창, 1920×1080·100% 배율 실측.
- 첫 뷰포트: 216px 사이드바와 나머지 작업 영역이 전폭을 채우고, 검색·파일 목록 상태·첫 결과들이 보인다.
- 그리드·pane·콘텐츠 위계: `216px sidebar + minmax(0, 1fr) main`; 메인은 단일 세로 흐름이며 결과 행 내부만 identity/meta 두 구역으로 나눈다.
- 스크롤 흐름과 밀도 변화: 문서 전체가 아니라 `.main-content`만 스크롤한다. 결과가 화면보다 길 때만 세로 스크롤 범위를 요구한다.

## Mobile Transformations

| Desktop element | Operation | Mobile result | Reason |
|---|---|---|---|
| 216px 사이드바 | compress | 920px 이하 72px 아이콘 레일 | 작업 폭을 확보하면서 현재 위치 유지 |
| 아이콘 레일 | replace | 680px 이하 메뉴 버튼과 오버레이 | 최소 창에서 결과 폭 우선 |
| 검색 입력과 상태 | reorder | 상태를 입력 아래 전폭 행으로 이동 | 질의 문자열 가독성 보존 |
| 4열 필터 | compress | 2열 후 1열로 변환 | 레이블과 값의 최소 폭 보존 |
| 결과 우측 메타 | reorder | 파일 정체성 아래 가로 메타 행 | 경로 폭과 행동 접근성 보존 |
| 보조 수집 근거 | collapse | 상세 이슈만 `details`로 접음 | 첫 과업의 스크롤 거리를 줄임 |

## States

| State | Trigger | User sees | Available action | Recovery |
|---|---|---|---|---|
| loading | 파일 목록 만들기 또는 검색 지연 | 단계 문구, 300ms 뒤 쉬운 진행 지표, 취소 | 읽기 취소 | 기존 파일 목록 유지 또는 재시도 |
| empty | 파일 목록 없음 또는 검색 결과 0개 | 위치 안내 또는 조건 완화 안내 | 위치 선택·파일 목록 만들기·필터 변경 | 파일 목록 생성 또는 검색어 수정 |
| error | parser·IPC·권한 오류 | 원인 텍스트와 경고 역할 | 질의 수정·범위 변경·갱신 | 오류가 사라지면 같은 화면에서 재검색 |
| success | 검색 응답 수신 | 결과 수·시간과 결과 행 | 열기·위치 표시 | 다른 조건으로 즉시 재검색 |
| stale | 범위 변경 또는 앱 내 파일 이동 | 검색 잠금과 갱신 이유 | 지금 업데이트 | 갱신 완료 후 검색 복구 |

## Performance Budget

- 첫 화면 필수 자산: 앱 CSS, Pretendard 사용 weight, JetBrains Mono 500·600, Lucide SVG, 파일 목록 상태 응답.
- 지연 가능한 자산: 검색 결과 목록과 상세 수집 이슈는 응답 이후 렌더한다.
- 폰트 weight·이미지·영상·모션 예산: 사용 중인 weight만 로드하고 이미지·영상은 사용하지 않으며 transform·opacity 외 지속 모션은 두지 않는다.
- 저성능 기기와 느린 네트워크 폴백: 로컬 IPC만 사용하고 검색 요청은 140ms debounce, 최신 요청 sequence만 반영하며 reduced-motion에서는 회전을 중단한다.

## Accessibility Contract

- 문서·랜드마크·헤딩 읽기 순서: skip link → navigation → main → h1 → 검색 h2 → 상태 → 결과 h2 → 결과 행.
- 키보드·포커스·Escape 동작: 내비게이션과 모든 행동은 Tab 접근, 결과 행은 Enter로 열기, 모바일 내비게이션은 Escape로 닫는다.
- 레이블·오류 연결·상태 알림: 검색 입력은 명시적 label과 문법 도움말 연결, 결과·오류·열기 피드백은 live region 또는 alert를 쓴다.
- 대비·색 외 신호·터치 타깃: 상태는 텍스트와 아이콘을 병행하고 포커스 링을 유지하며 핵심 제어 높이는 34px 이상을 유지한다.
- reduced-motion과 대체 경험: spinner 애니메이션을 제거해도 상태 텍스트가 남는다.

## Adopt

- 검색창·주요 필터·결과를 장식보다 앞세우는 고밀도 데이터 도구 구조.
- 파일명·경로·크기·수정일의 반복 가능한 열 정렬.
- 파일 목록의 위치와 새로고침 상태를 검색 근처에서 확인하는 원리.

## Adapt

- FastFind식 표를 그대로 복제하지 않고 BroomSweepy의 안전 문구와 행 단위 행동을 결합한다.
- 모바일 사이트가 아니라 데스크톱 최소 창을 대상으로 아이콘 레일·오버레이·세로 재배치를 검증한다.
- 반응형 검증은 WebView 에뮬레이션 대신 네이티브 창 리사이즈와 실제 client metrics를 함께 사용한다.

## Avoid

- 검색 UI와 파일 작업을 관리자 권한 하나로 묶는 구조.
- 결과를 삭제 추천처럼 보이게 하는 카피.
- 네이티브 창과 웹뷰 크기를 분리하는 DevTools 에뮬레이션.
- 검은 빈 영역, 수평 스크롤, 중첩 glass 카드, 장식용 효과.

## Prompt Contract

GOAL — 실제 Tauri 창 전체를 채우는 빠른 파일 검색 작업면을 유지한다.
AUDIENCE — 파일 위치를 찾거나 용량 증가 원인을 조사하는 Windows·macOS 사용자.
TASK — 범위 확인, 질의 입력, 필터 조정, 결과 비교, 열기 또는 위치 표시.
FLOW — navigation → header → query → catalog trust → evidence/error → results.
HEADER — 현재 화면, 설명, 실제 범위를 한 행에 표시한다.
MESSAGE — 파일을 열지 않고 이름과 경로를 즉시 찾는다.
FACTS — 런타임에서 받은 범위·항목 수·시각·공급자·검색 시간만 사실로 표시한다.
CONTENT_INTEGRITY — 실제 값은 verified, fixture 성능은 prototype으로 분리한다.
SECTION_ORDER — 검색 명령을 먼저, 결과 신뢰 근거를 바로 뒤, 결과를 다음에 둔다.
CTA — 파일 목록 없음에서는 만들기, 준비됨에서는 검색 입력, 결과에서는 위치 표시.
TRUST — 로컬 전용·메타데이터만 수집·삭제 추천 아님을 행동 전 표시한다.
ASSETS — 프로젝트 폰트와 Lucide 아이콘만 사용한다.
LAYOUT — 216px sidebar + fluid main, main-only vertical scroll, no horizontal overflow.
RESPONSIVE — 920px 아이콘 레일, 680px 오버레이, 필터와 결과 메타 순차 재배치.
STATES — loading, empty, error, success, stale를 실제 기능과 연결한다.
PERFORMANCE — 140ms debounce, 최신 요청만 반영, 이미지·영상 없음.
ACCESSIBILITY — skip link, label/help 연결, 키보드 열기, live status, reduced-motion.
PRESERVE — DESIGN.md의 dark data instrument, 단층 glass, Pretendard·JetBrains 체계.
EXCLUDE — device emulation 잔여 상태, 검은 빈 영역, 중첩 카드, 삭제 오인 카피.
SUCCESS — 기본·최소 네이티브 창에서 앱 셸이 전폭을 채우고 모든 핵심 행동에 도달한다.

## Success Checks

- 첫 5초 안에 이름·경로 검색 화면과 현재 범위를 설명할 수 있는가?
- 파일 목록 수치와 읽기 방식이 실제 응답일 때만 쉬운 말로 표시되는가?
- 검색 준비·오류·빈 결과·성공·오래된 상태에서 다음 행동이 있는가?
- 최소 창에서 사이드바와 필터가 과업 순서에 맞게 재구성되고 세로 스크롤로 도달 가능한가?
- 글래스 외관을 유지하면서 대비·키보드·응답성과 네이티브 창 일치가 모두 검증됐는가?
