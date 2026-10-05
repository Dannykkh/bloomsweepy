# Experience Contract: Rust 기반 Swift 경험 통합

## Source Mode

- Mode: product-derived
- Evidence: `docs/design-refs/2026-09-04-brief-rust-swift-experience.md`, `docs/design-refs/2026-09-04-benchmark-swift-native-glass.md`, Swift 소스와 실제 화면, Rust 런타임 타입과 안전 API

## Product Facts

| Claim | Source | Captured at | Freshness/status | Allowed presentation |
|---|---|---|---|---|
| 디스크 전체·사용·남은 용량은 운영체제가 보고한 런타임 값이다 | `VolumeInfo`, `get_system_overview` | 실행 시점 | runtime | 단위와 사용률을 함께 표시 |
| 큰 파일과 중복 파일은 Rust 스캔 보고서의 결과다 | `ScanReport` | 스캔 완료 시점 | runtime | 스캔 완료 시각과 결과 수를 함께 표시 |
| 중복은 hard-link 제외, 샘플·전체 해시와 최종 비교를 거친다 | Rust scan core와 테스트 | 2026-09-04 | current | `검증된 중복`으로 표시 가능 |
| 정리 후보는 confidence가 `likelySafe`인 항목과 검토 항목을 구분한다 | `CleanupScanReport` | 스캔 완료 시점 | runtime | 안전 후보와 검토 후보를 합산하지 않음 |
| 일반 파일 이동은 실행 직전 재검증하고 운영체제 휴지통과 작업 저널을 사용한다 | trash action API와 테스트 | 2026-09-04 | current | 최종 확인 직전에 복구 경계를 표시 |

## Benchmark Sources

- 프로젝트 소유 Swift 대시보드: `demo-assets/01_dashboard.png`, 2026-09-04 재검토.
- Swift UI 소스: `BroomSweepy/ContentView.swift`, `BroomSweepy/Views/DashboardView.swift`, `BroomSweepy/Views/SmartCleanView.swift`.
- 현재 Rust 비교 화면: `docs/assets/screenshots/v1.2.0-dashboard.png`, 2026-09-04 재검토.

## Page Goal

- 사용자가 이 화면에서 달성할 결과: 현재 저장공간 상태를 이해하고 한 번의 클릭으로 검사 또는 필요한 도구를 시작한다.
- 제품이 얻어야 하는 결과: Swift의 명료한 첫인상과 Rust의 검증 가능한 기능·안전성을 하나의 앱에서 제공한다.
- 관찰 가능한 성공 조건: 드라이브 카드 교대, 선택 드라이브 검사 시작·취소·완료, 정리 후보 검사, 네 도구 진입이 실제 상태와 연결된다.

## Audience and Tasks

- 주요 사용자와 사용 상황: 저장공간이 부족하지만 어떤 기능부터 써야 하는지 모르는 개인 사용자.
- 최우선 과업: 디스크 상태 확인 후 `이 드라이브 검사`로 큰 파일과 검증된 중복을 찾는다.
- 시작 조건과 완료 조건: 앱 실행과 함께 시작하고 검사 결과 수·용량·완료 시각 또는 명확한 빈 상태가 보이면 완료한다.
- 주요 불안·마찰·실패 가능성: 폴더 선택 필요, 긴 검사, 권한 오류, 자동 삭제 오인, 여러 용량 수치의 중복 합산.

## Header and Navigation

- 2026-10-05 delta: macOS 네이티브 제목과 창 버튼을 표시하고 독립된 제목 표시줄을 드래그 영역으로 유지한다. 본문에 가짜 창 버튼이나 추가 헤더를 만들지 않는다. 현재 메뉴 순서는 `DESIGN.md`를 따른다.
- 브랜드·현재 위치·전역 이동·주 행동의 순서: 브랜드 → 대시보드 → 공간 정리 → 파일 관리 → AI 도우미 → 조건부 Docker → 설정 → 현재 화면.
- 데스크톱 내비게이션: 약 220px 고정 글래스 사이드바, 큰 색상 아이콘, 이름과 짧은 설명, 활성 항목 하나.
- 모바일 대체 구조: 920px 미만 아이콘 레일, 680px 미만 메뉴 버튼과 오버레이. 파일 관리와 공간 정리는 화면 내부 보조 탭으로 leaf route를 보존한다.

## Core Message

- 핵심 약속: 저장공간 상태를 한눈에 보고, 한 번 눌러 원인을 찾는다.
- 설명: 전체 검사는 큰 파일과 실제 중복을 찾고, 정리 후보 검사는 임시 파일과 남은 흔적을 별도로 찾는다.
- 증거: 운영체제 디스크 수치, 실제 파일 수·경로·용량, 스캔 단계와 완료 시각.
- 사용자가 다음에 이해해야 할 것: 검사와 삭제는 분리되어 있으며 파일 이동은 검토·확인·재검증 뒤 휴지통에서만 실행된다.

## Content Integrity

| Content item | Classification | Evidence | Presentation rule |
|---|---|---|---|
| 디스크 사용률·남은 용량 | verified | 운영체제 런타임 보고 | 현재 크게 선택한 물리 볼륨이 있을 때만 표시 |
| 큰 파일·중복 요약 | verified | 현재 `ScanReport` | 보고서가 있을 때만 실제 값 표시 |
| 정리 후보 요약 | verified | 현재 `CleanupScanReport` | `likelySafe`와 검토 필요를 분리 |
| 최근 정리·최근 파일 | verified | 작업 저널·파일 카탈로그 | 기록이 없으면 명시적 빈 상태 |
| 건강 점수·메모리 최적화·보안 안전 | hypothesis | Rust 근거 없음 | 사실로 승격하지 않고 렌더하지 않음 |

## Section Order

1. 대시보드 브리핑: 현재 디스크 상태와 데이터 신선도를 짧게 알린다.
2. 저장공간 hero: 선택 드라이브의 큰 링, 다른 드라이브의 작은 링 카드, 남은 용량, 한 개의 주 행동, 보조 검사 행동을 보여준다.
3. 빠른 실행: 정리 후보·큰 파일·중복 파일·파일 찾기를 큰 타일로 연결한다.
4. 실제 결과 요약: 완료된 보고서가 있을 때만 서로 겹치지 않는 지표를 표시한다.
5. 최근 활동: 최근 정리와 최근 추가 파일은 보조 정보로 아래에 둔다.

## CTA Strategy

- Primary: `이 드라이브 검사` — 현재 크게 표시한 드라이브를 Rust 스캔 루트로 확정하고 큰 파일·중복 검사를 시작한다.
- Secondary: `정리 후보 찾기` — 별도의 cleanup scan을 시작하며 destructive action은 실행하지 않는다.
- 반복 규칙: 첫 뷰포트의 solid primary는 하나다. 빠른 실행 타일은 모두 같은 중립 계층이다.
- 완료·실패 피드백: 검사 중 현재 단계·처리량·취소, 완료 후 결과 수와 검토 화면, 실패 후 원인별 재시도를 제공한다.

## Trust Strategy

- 사용자가 불안을 느끼는 지점: `원클릭`이 자동 삭제를 뜻하는지, 중복 판정이 정확한지, 정리 후보가 안전한지.
- 그 직전에 제시할 근거: `검사는 파일을 바꾸지 않음`, 검증된 중복, 안전 후보·검토 후보 분리, 최종 휴지통 확인.
- 출처·날짜·검증 가능성: 현재 런타임 보고서와 작업 저널, 완료 시각을 사용한다.
- 근거가 없을 때 생략할 요소: 건강 점수, 총 정리 가능 용량, 자동 삭제 성공 카피를 생략한다.

## Asset Provenance

| Asset | Source | Local path | License/trademark/attribution | Modification allowed | Status/fallback |
|---|---|---|---|---|---|
| Swift 대시보드 캡처 | 프로젝트 소유 | `demo-assets/01_dashboard.png` | 프로젝트 소유 | reference only | verified |
| BroomSweepy 아이콘 | 프로젝트 소유 | `apps/desktop/src-tauri/icons/` | 프로젝트 소유 | yes | verified |
| Pretendard Variable | 프로젝트 번들 | `apps/desktop/src/assets/fonts/PretendardVariable.woff2` | SIL OFL 1.1 | yes | system sans fallback |
| Lucide 아이콘 | npm dependency | `lucide-react` | ISC | yes | text label fallback |

## Desktop Structure

- 기준 뷰포트: 1280×820, 최소 지원 창 760×600.
- 첫 뷰포트: 사이드바, 짧은 브리핑, 2:1 hero, 4개 빠른 실행의 시작 또는 전체.
- 그리드·pane·콘텐츠 위계: 220px 안팎 사이드바 + 유연한 메인; hero는 드라이브 덱 2/3와 상태·행동 1/3의 비대칭 구성이다. 드라이브 덱은 큰 카드 하나와 최대 높이를 둔 작은 카드 rail로 구성한다.
- 스크롤 흐름과 밀도 변화: 첫 화면은 낮은 밀도와 큰 행동, 아래 실제 결과와 최근 활동은 controlled-medium.

## Mobile Transformations

| Desktop element | Operation | Mobile result | Reason |
|---|---|---|---|
| 220px 사이드바 | compress | 72px 아이콘 레일 | 링과 행동 폭 확보 |
| 아이콘 레일 | replace | 메뉴 버튼 + 오버레이 | 680px 미만에서 본문 우선 |
| 드라이브 덱 2/3 + 상태 1/3 hero | reorder | 상태 → 큰 카드 → 작은 카드 가로 rail → CTA | 읽기 순서와 44px 타깃 보존 |
| 빠른 실행 4열 | compress | 2×2 타일, 더 좁으면 1열 | 44px 타깃과 라벨 유지 |
| 실제 결과 요약 | collapse | 핵심값 2개와 `결과 보기` | 첫 행동을 방해하지 않음 |
| 최근 활동 2열 | reorder | 최근 정리 뒤 최근 파일 | 시간 흐름 우선 |

## States

| State | Trigger | User sees | Available action | Recovery |
|---|---|---|---|---|
| loading | 초기 디스크 조회가 300ms 이상 | 링 자리와 `디스크 확인 중` | 기다림, 필요 시 새로 고침 | 성공 데이터만 부분 유지 |
| scanning | 전체 검사 실행 | 단계·처리 파일·용량, 링의 정적 상태 변화 | `검사 중단` | 이전 결과를 유지하거나 다시 시작 |
| cleanup scanning | 정리 후보 검사 실행 | 처리 위치·후보 수 | `검사 중단` | 정리 후보만 다시 시작 |
| empty | 디스크 또는 결과 없음 | 원인 단정 없는 빈 상태와 다음 행동 | 새로 고침·폴더 선택 | 접근 가능한 범위로 재개 |
| error | 시스템·전체 검사·정리 검사 실패 | 서로 다른 오류 블록과 대상별 재시도 | 해당 작업 다시 시도 | 성공한 다른 데이터는 유지 |
| success | 검사 완료 | 실제 지표와 결과 화면 CTA | 결과 검토·다시 검사 | 파일 변경 시 stale 처리 |
| drive switched | 작은 드라이브 카드 선택 | 선택 카드가 크게, 이전 카드는 작게 교대 | 선택 드라이브 검사 | 드라이브 제거 시 시스템·고정 드라이브 순으로 폴백 |

## Performance Budget

- 2026-10-05 material delta: macOS Tauri 직접 배포 창에서만 native-glass를 선택한다. 어두운 body wash 한 층과 sidebar/hero 표면을 사용하고 app-shell의 추가 어두운 층을 없앤다. Windows·웹·투명도 감소 요청은 불투명 canvas로 폴백하며 새 blur·애니메이션·의존성은 추가하지 않는다.
- 첫 화면 필수 자산: 번들 CSS·폰트·SVG 아이콘·운영체제 디스크 요약.
- 지연 가능한 자산: 최근 파일 행과 상세 결과 목록은 hero 이후 렌더한다.
- 폰트 weight·이미지·영상·모션 예산: Pretendard variable 1개, 외부 이미지·영상 0, CSS transform/opacity와 SVG stroke 전환만.
- 저성능 기기와 느린 네트워크 폴백: 로컬 IPC만 사용하며 vibrancy·backdrop-filter 미지원 시 불투명 표면, reduced-motion 시 즉시 최종 상태.

## Accessibility Contract

- 문서·랜드마크·헤딩 읽기 순서: skip link → 전역 내비 → 대시보드 h1 → 상태 → 링 설명 → 주 행동 → 빠른 실행 → 활동.
- 키보드·포커스·Escape 동작: 모든 타일과 작은 드라이브 카드를 button으로 구현한다. 카드 교대 뒤 포커스는 새 큰 카드로 복원하고, 오버레이는 Escape로 닫으며 진행 중 취소 버튼을 명시한다.
- 레이블·오류 연결·상태 알림: 링에 사용률과 남은 용량 레이블, 진행에 `role=status`, 오류에 `role=alert`를 사용한다.
- 대비·색 외 신호·터치 타깃: 14px 이상, 최소 44px, 활성·경고는 아이콘·텍스트를 색과 함께 제공한다.
- reduced-motion과 대체 경험: entrance·stroke·카드 교대 transition을 제거하고 동일한 최종 배치를 즉시 제공한다.

## Adopt

- Swift의 중앙 링, 넓은 여백, 큰 색상 아이콘, 단층 native material, phase-based flow를 채택한다.

## Adapt

- 건강 링을 실제 디스크 사용률 링으로, Swift 빠른 기능을 현재 Rust의 네 실제 기능으로 바꾼다.
- Swift의 macOS material은 Tauri native effect와 CSS 폴백으로 변환한다.
- Swift 카테고리는 현재 Rust route만 노출하는 표시용 상위 그룹으로 축소한다.

## Avoid

- 건강 점수·가짜 최적화 수치, 자동 삭제, 결과 용량 중복 합산, 중첩 blur, 지속 glow, 플랫폼 장식 복제를 피한다.

## Prompt Contract

GOAL — Rust/Tauri를 단일 제품으로 유지하며 Swift의 native-glass 첫 경험을 재현한다.
AUDIENCE — 저장공간 부족 원인을 빠르고 안전하게 찾고 싶은 개인 사용자.
TASK — 디스크 상태 확인 → 한 번 눌러 검사 → 결과 검토 → 별도 확인 뒤 휴지통 이동.
FLOW — 대시보드 → 전체 검사 또는 정리 후보 검사 → 진행 → 결과 → 검토.
HEADER — 브랜드와 큰 카테고리만 남기고 대시보드의 반복 페이지 헤더는 제거한다. macOS 제목·버전·창 버튼은 네이티브 제목 표시줄에 표시한다.
MESSAGE — 저장공간 상태를 한눈에 보고, 한 번 눌러 원인을 찾는다.
FACTS — 운영체제와 Rust 런타임이 제공한 수치·경로·시각만 사실로 표시한다.
CONTENT_INTEGRITY — 건강 점수와 총 정리 가능 용량을 만들지 않는다.
SECTION_ORDER — 브리핑 → hero → 빠른 실행 → 실제 결과 → 최근 활동.
CTA — `이 드라이브 검사` 하나가 primary, `정리 후보 찾기`는 시스템 범위 secondary.
TRUST — 검사는 읽기 전용이며 삭제는 별도 확인·재검증·휴지통이라는 경계를 행동 근처에 둔다.
ASSETS — 프로젝트 소유 reference, 번들 폰트, Lucide만 사용한다.
LAYOUT — 220px 글래스 사이드바, 큰 선택 드라이브+작은 드라이브 rail의 비대칭 hero, 4개 큰 타일, 하단 밀도 증가.
RESPONSIVE — 사이드바 레일·오버레이, hero 단일 열, 작은 드라이브 가로 rail, 타일 2×2/1열로 재구성한다.
STATES — 초기 loading, 두 종류 scanning, empty, error, success를 실제 상태에 연결한다.
PERFORMANCE — 단층 blur, CSS-only motion, 로컬 IPC, 외부 자산 0.
ACCESSIBILITY — 14px, 44px, 명시 레이블, status/alert, reduced-motion을 보존한다.
PRESERVE — Rust scan·duplicate·trash·journal 안전 계약과 모든 기존 결과 화면.
EXCLUDE — Swift 런타임, 가짜 수치, 자동 삭제, 구현되지 않은 메뉴, nested glass.
SUCCESS — 실제 macOS 설치 앱에서 Swift와 같은 제품으로 느껴지며 검사·취소·결과 진입이 동작한다.

## Success Checks

- 첫 5초 안에 핵심 약속과 주 행동을 설명할 수 있는가?
- 화면의 사실·수치·후기·브랜드 자산이 출처와 상태를 가지며, unverified 항목을 사실처럼 보이지 않는가?
- 주요 과업을 막는 상태·정보·행동 누락이 없는가?
- 모바일이 데스크톱 축소판이 아니라 우선순위에 맞게 재구성됐는가?
- 아름다움, 접근성, 성능 중 하나를 다른 하나의 희생으로 얻지 않았는가?
