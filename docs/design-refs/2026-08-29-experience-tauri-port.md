# Experience Contract: BroomSweepy Tauri Port

## Source Mode

- Mode: product-derived
- Evidence: `docs/design-refs/2026-08-29-brief-tauri-port.md`, `docs/promo.mp4`, 기존 SwiftUI 화면, 사용자 요청

## Product Facts

| Claim | Source | Captured at | Freshness/status | Allowed presentation |
|---|---|---|---|---|
| 기존 앱은 SwiftUI 기반 macOS 저장공간 관리 앱이다 | `project.yml`, `BroomSweepy/` | 2026-08-29 | current | 기존 macOS 앱 설명에만 사용 |
| 대상 아키텍처는 Rust 코어와 Tauri 2 데스크톱 셸이다 | 사용자 결정, 설계 문서 | 2026-08-29 | current | 구현 상태와 함께 표시 |
| 확보 가능 용량과 파일 수는 사용자의 실제 스캔 결과다 | 런타임 스캔 결과 | 실행 시점 | runtime | 마지막 스캔 시각과 함께 표시 |

## Benchmark Sources

- 해당 없음 — product-derived. 기존 BroomSweepy 자체가 이식 기준이다.

## Page Goal

- 사용자가 이 화면에서 달성할 결과: 용량을 차지하는 파일을 찾고 중복 여부와 삭제 안전성을 검토한다.
- 제품이 얻어야 하는 결과: macOS와 Windows에서 같은 과업 흐름과 안전 기준을 제공한다.
- 관찰 가능한 성공 조건: 폴더 선택부터 결과 확인, 실패 복구, 스캔 취소까지 실제로 완료된다.

## Audience and Tasks

- 주요 사용자와 사용 상황: 디스크 경고를 받은 뒤 무엇이 공간을 차지하는지 즉시 확인하려는 개인 사용자.
- 최우선 과업: 스캔 범위를 선택하고 큰 파일과 정확히 검증된 중복 파일을 찾는다.
- 시작 조건과 완료 조건: 접근 가능한 폴더가 선택되면 시작하고, 결과 수·크기·스캔 시각이 표시되면 완료한다.
- 주요 불안·마찰·실패 가능성: 권한 거부, 긴 대기, 잘못된 중복 판정, 영구 삭제에 대한 불안.

## Header and Navigation

- 브랜드·현재 위치·전역 이동·주 행동의 순서: 브랜드, 대시보드, 공간 정리, 중복 파일, 설정, 현재 스캔 범위, 스캔 행동.
- 데스크톱 내비게이션: 고정 좌측 사이드바에서 활성 항목 하나만 강조한다.
- 모바일 대체 구조: 좁은 창에서는 아이콘 레일, 더 좁으면 메뉴 버튼으로 여는 오버레이 내비게이션.

## Core Message

- 핵심 약속: 어디에서 용량이 늘었는지 확인하고, 지우기 전에 근거를 검토한다.
- 설명: 큰 파일과 중복 파일을 하나의 스캔 흐름으로 찾는다.
- 증거: 실제 경로, 논리 크기, 수정 시각, 중복 검증 단계, 마지막 스캔 시각.
- 사용자가 다음에 이해해야 할 것: 삭제는 스캔과 분리된 검토 단계이며 기본 동작은 휴지통 이동이다.

## Content Integrity

| Content item | Classification | Evidence | Presentation rule |
|---|---|---|---|
| 디스크 사용률 | verified | 운영체제 런타임 API | 단위와 측정 시각 표시 |
| 큰 파일 결과 | verified | Rust 스캔 엔진 | 경로와 크기를 함께 표시 |
| 중복 파일 그룹 | verified | 크기, 샘플 해시, 전체 해시, 최종 검증 | 검증 단계가 완료된 그룹만 표시 |
| 프로토타입 스캔 데이터 | prototype | 개발 모드 fixture | 개발 모드임을 명시하고 배포 결과에 사용 금지 |

## Section Order

1. 상태 헤더: 현재 범위와 마지막 스캔 시각을 확인한다.
2. 스캔 스테이지: 저장공간 링과 단일 primary action으로 스캔을 시작한다.
3. 요약 스트립: 큰 파일, 중복 낭비, 확인한 파일 수를 비교한다.
4. 결과 목록: 실제 파일 근거를 정렬하고 상세를 검토한다.
5. 안전 행동 바: 선택한 대상과 예상 확보 용량을 확인한 뒤 휴지통 이동을 실행한다.

## CTA Strategy

- Primary: `스캔 시작` — 범위가 선택된 첫 화면과 완료 후 다시 스캔할 때 표시한다.
- Secondary: `폴더 선택`, 스캔 중에는 `스캔 취소`.
- 반복 규칙: 동일 화면에 primary solid 버튼은 하나만 둔다.
- 완료·실패 피드백: 완료는 결과 수·용량·시각, 실패는 경로·원인·다시 선택 행동을 보여준다.

## Trust Strategy

- 사용자가 불안을 느끼는 지점: 중복 판정과 파일 삭제 직전.
- 그 직전에 제시할 근거: 전체 콘텐츠 검증, 원본 유지 규칙, 선택 파일 수, 확보 예상량, 휴지통 이동 여부.
- 출처·날짜·검증 가능성: 런타임 엔진 결과와 작업 저널에 기록한다.
- 근거가 없을 때 생략할 요소: 안전 점수나 자동 삭제 추천을 표시하지 않는다.

## Asset Provenance

| Asset | Source | Local path | License/trademark/attribution | Modification allowed | Status/fallback |
|---|---|---|---|---|---|
| BroomSweepy 앱 아이콘 | 프로젝트 소유 자산 | `BroomSweepy/Resources/Assets.xcassets/AppIcon.appiconset/` | 프로젝트 소유 | yes | verified |
| 기존 UI 영상 | 프로젝트 소유 자산 | `docs/promo.mp4` | 프로젝트 소유 | yes | verified |
| Pretendard Variable | 공식 배포 저장소 | 웹폰트 import | SIL Open Font License 1.1 | yes | verified/system sans fallback |
| Lucide 아이콘 | npm package | 번들 의존성 | ISC | yes | verified/text label fallback |

## Desktop Structure

- 기준 뷰포트: 1440×920, 최소 지원 창 760×600.
- 첫 뷰포트: 사이드바, 상태 헤더, 저장공간 링, 스캔 CTA, 요약 스트립.
- 그리드·pane·콘텐츠 위계: 216px 사이드바와 유연한 메인, 링 스테이지 2/3와 보조 요약 1/3.
- 스크롤 흐름과 밀도 변화: 첫 화면은 상태와 행동 중심, 아래로 갈수록 조밀한 파일 결과 목록.

## Mobile Transformations

| Desktop element | Operation | Mobile result | Reason |
|---|---|---|---|
| 216px 사이드바 | compress | 72px 아이콘 레일 | 메인 결과 폭 확보 |
| 아이콘 레일 | replace | 메뉴 버튼과 오버레이 내비 | 680px 미만에서 파일 경로 가독성 유지 |
| 링과 보조 요약 2분할 | reorder | 링, CTA, 요약 순 단일 흐름 | 스캔 행동을 먼저 완료 |
| 전체 파일 열 | collapse | 이름·크기만 표시하고 상세 drawer | 핵심 비교 필드 보존 |
| 삭제 행동 바 | sticky | 창 하단 고정 요약과 행동 | 선택 후 행동을 잃지 않음 |

## States

| State | Trigger | User sees | Available action | Recovery |
|---|---|---|---|---|
| loading | 스캔이 300ms 이상 지속 | 현재 단계, 진행률, 처리 수 | 스캔 취소 | 취소 뒤 이전 결과 유지 |
| empty | 스캔 완료, 결과 없음 | 확인 범위와 완료 메시지 | 다른 폴더 선택, 다시 스캔 | 범위 변경 |
| error | 권한 또는 I/O 실패 | 실패 경로와 원인 | 다시 시도, 폴더 재선택 | 성공한 부분 결과는 유지 |
| success | 스캔 완료 | 요약, 결과, 완료 시각 | 정렬, 검토, 다시 스캔 | 파일 변경 시 stale 표시 |
| permission | 폴더 접근 불가 | 접근이 필요한 이유 | 폴더 다시 선택 | 접근 가능한 범위로 재개 |
| cancelled | 사용자가 취소 | 취소 시점과 이전 결과 | 다시 스캔 | 새로운 작업으로 교체 |

## Performance Budget

- 첫 화면 필수 자산: CSS, Pretendard 본문 weight, SVG 아이콘, 초기 디스크 요약.
- 지연 가능한 자산: 상세 결과, 전체 해시, 비주요 화면 번들.
- 폰트 weight·이미지·영상·모션 예산: Pretendard variable 1개와 숫자용 JetBrains Mono 2 weight, 영상 없음, CSS 상태 모션만.
- 저성능 기기와 느린 네트워크 폴백: 웹폰트 실패 시 sans/monospace, 블러 미지원 시 불투명 표면, reduced-motion 정적 상태.

## Accessibility Contract

- 문서·랜드마크·헤딩 읽기 순서: 내비게이션 다음 메인 헤딩, 상태, 행동, 요약, 결과 순서.
- 키보드·포커스·Escape 동작: 모든 내비와 버튼 접근, Escape로 오버레이·폴더 선택 취소, 스캔 취소는 명시 버튼.
- 레이블·오류 연결·상태 알림: 아이콘 버튼 이름, `role=status`, `role=alert`, 진행률 value 제공.
- 대비·색 외 신호·터치 타깃: 최소 44px, 상태에 아이콘과 텍스트 병행, 반투명 합성 대비 검증.
- reduced-motion과 대체 경험: entrance와 링 전환 제거, 최종 수치 즉시 표시.

## Adopt

- 기존 제품의 사이드바 과업 분류, 중앙 저장공간 링, 다크 글래스 재질을 채택한다.

## Adapt

- macOS 시스템 재질은 CSS 글래스 토큰과 플랫폼 창 재질로 변환한다.
- macOS 전용 메뉴와 단축키는 운영체제 관례에 맞춰 적응한다.
- 기존 카드 위주의 하단 영역은 비교 가능한 요약 스트립과 파일 목록으로 정돈한다.

## Avoid

- macOS 신호등 버튼을 Windows에 복제하지 않는다.
- 부분 해시만으로 중복을 확정하지 않는다.
- 중첩 backdrop blur, 무한 링 애니메이션, 전체 결과 JSON 일괄 전송을 사용하지 않는다.
- 성공 근거 없는 `원클릭 최적화`를 첫 크로스플랫폼 구현에 노출하지 않는다.

## Prompt Contract

GOAL — 기존 BroomSweepy를 Rust/Tauri 2의 안전한 크로스플랫폼 저장공간 도구로 이식한다.
AUDIENCE — 디스크 부족 원인을 빠르게 찾고 삭제 전 근거를 원하는 macOS·Windows 사용자.
TASK — 폴더 선택, 스캔, 큰 파일과 정확한 중복 확인, 복구 가능한 정리.
FLOW — 범위 선택 → 스캔 → 단계 진행 → 요약 → 결과 검토 → 휴지통 이동.
HEADER — 브랜드와 현재 페이지, 현재 범위, 마지막 스캔 시각.
MESSAGE — 어디에서 용량이 늘었는지 확인하고 지우기 전에 근거를 검토한다.
FACTS — 런타임 파일 크기·경로·해시 검증·스캔 시각만 사실로 표시한다.
CONTENT_INTEGRITY — fixture는 개발용으로 분리하고 운영 결과로 표시하지 않는다.
SECTION_ORDER — 상태 헤더 → 스캔 스테이지 → 요약 → 결과 → 안전 행동.
CTA — `스캔 시작` 하나를 primary로, 폴더 선택과 취소를 secondary로 둔다.
TRUST — 중복 검증 단계와 휴지통 이동을 삭제 직전에 설명한다.
ASSETS — 프로젝트 아이콘·영상, 라이선스가 확인된 웹폰트·SVG 아이콘만 사용한다.
LAYOUT — 216px 사이드바와 링 중심 2/3 작업면, 아래 조밀한 결과 목록.
RESPONSIVE — 사이드바를 레일과 오버레이로 변환하고 결과 열을 drawer로 접는다.
STATES — loading, empty, error, permission, success, cancelled, stale를 구현한다.
PERFORMANCE — Rust에 인덱스를 유지하고 집계·페이지 결과만 IPC로 전달한다.
ACCESSIBILITY — 키보드, 명시 포커스, 상태 라이브 영역, reduced-motion을 지원한다.
PRESERVE — 기존 다크 글래스, 저장공간 링, 과업 중심 사이드바, 상태색 의미.
EXCLUDE — 영구 삭제 기본값, 부분 해시 확정, 중첩 블러, 지속 장식 모션.
SUCCESS — Windows 빌드와 실제 스캔이 동작하고 기존 제품으로 인식되는 렌더를 만든다.

## Success Checks

- 첫 5초 안에 핵심 약속과 주 행동을 설명할 수 있는가?
- 화면의 사실·수치·후기·브랜드 자산이 출처와 상태를 가지며, unverified 항목을 사실처럼 보이지 않는가?
- 주요 과업을 막는 상태·정보·행동 누락이 없는가?
- 모바일이 데스크톱 축소판이 아니라 우선순위에 맞게 재구성됐는가?
- 아름다움, 접근성, 성능 중 하나를 다른 하나의 희생으로 얻지 않았는가?
