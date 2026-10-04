# Experience Contract: Rust 성능 화면

## Source Mode

- Mode: product-derived
- Evidence: 사용자 요구, Swift `PerformanceView`·`SystemMonitor`·`MemoryManager`, Rust `sysinfo 0.39`, 기존 디자인 brief

## Product Facts

| Claim | Source | Captured at | Freshness/status | Allowed presentation |
|---|---|---|---|---|
| CPU·메모리·프로세스 수치는 현재 기기에서 측정한다 | Rust `sysinfo 0.39` 런타임 데이터 | 2026-09-04 | current; 캡처 시각 포함 | 마지막 측정 시각과 함께 표시 |
| 프로세스 메모리는 resident memory다 | `sysinfo::Process::memory` 계약 | 2026-09-04 | current | `메모리`로 표시하고 정리 가능 용량으로 해석 금지 |
| macOS 종료는 일반 GUI 앱에 보내는 정상 종료 요청이다 | macOS `NSRunningApplication.terminate` | 2026-09-04 | current | `종료 요청`으로 표시; 즉시·강제 종료라고 주장 금지 |
| macOS 메모리 정리는 현재 BroomSweepy malloc 영역에만 pressure relief를 요청한다 | macOS SDK `malloc_zone_pressure_relief` 선언과 제품 설계 결정 | 2026-09-05 | current | 다른 앱·시스템 캐시와 분리해 설명 |
| 정리 결과의 반환량은 allocator가 실제 `munmap`했다고 보고한 바이트다 | macOS SDK `malloc_zone_pressure_relief` 반환 계약 | 2026-09-05 | current | `BroomSweepy가 반환한 메모리`로만 표시 |

## Benchmark Sources

- 해당 없음 — product-derived. 프로젝트 소유 SwiftUI 화면과 기존 Rust 글래스 대시보드를 golden master로 사용한다.

## Page Goal

- 사용자가 이 화면에서 달성할 결과: 현재 부하를 확인하고 BroomSweepy 자체의 반환 가능한 메모리를 한 번에 정리하거나, 많이 사용하는 일반 앱 하나에 정상 종료를 요청한다.
- 제품이 얻어야 하는 결과: Swift의 시원한 상태 화면을 사실 기반 Rust 기능으로 대체한다.
- 관찰 가능한 성공 조건: 실제 값·측정 시각·대상 이름·종료 결과가 끊김 없이 연결되고 위험 대상은 백엔드에서 거부된다.

## Audience and Tasks

- 주요 사용자와 사용 상황: Mac이 느리거나 메모리 사용량이 궁금한 일반 사용자.
- 최우선 과업: CPU/RAM 상태 확인 → 필요하면 BroomSweepy 메모리 원클릭 정리 → 상위 프로세스 비교 → 종료 대상 검토 → 정상 종료 요청.
- 시작 조건과 완료 조건: 화면 진입으로 시작해 실제 종료 여부 또는 정확한 실패 이유를 확인하면 끝난다.
- 주요 불안·마찰·실패 가능성: 저장하지 않은 작업 손실, PID 재사용, 오래된 수치, 시스템 프로세스 오인, 종료 요청 거절.

## Header and Navigation

- 브랜드·현재 위치·전역 이동·주 행동의 순서: 기존 사이드바 → `성능` 현재 위치 → 측정 시각과 새로 고침.
- 데스크톱 내비게이션: `파일 관리` 다음, `AI 도우미` 앞에 `성능`을 둔다.
- 모바일 대체 구조: 기존 오버레이 내비게이션을 유지하고 성능 항목은 동일한 44px 이상 행으로 제공한다.

## Core Message

- 핵심 약속: 느려지는 원인을 실제 수치로 확인합니다.
- 설명: CPU와 메모리의 현재 상태, 많이 사용하는 프로세스를 한 화면에서 비교한다.
- 증거: 런타임 CPU·RAM·resident memory·프로세스별 CPU와 측정 시각.
- 사용자가 다음에 이해해야 할 것: 메모리 정리는 BroomSweepy 자신에게만 적용되며, 다른 앱 메모리는 별도의 정상 종료 검토로만 줄일 수 있다.

## Content Integrity

| Content item | Classification | Evidence | Presentation rule |
|---|---|---|---|
| 전체 CPU·메모리·스왑 | verified | 현재 런타임 snapshot | 캡처 시각과 함께 표시 |
| 프로세스별 CPU·메모리 | verified | 같은 sampler의 현재 snapshot | 정렬 가능; 과거값처럼 보이면 stale 표시 |
| 종료 가능 여부 | verified | 백엔드 보호 정책과 플랫폼 capability | 버튼 활성화 근거로만 사용; 프론트 값을 권한으로 신뢰하지 않음 |
| BroomSweepy allocator 반환량 | verified | macOS allocator 반환값 | current-process 결과로만 표시; 시스템 전체 확보량으로 확대 해석 금지 |
| 시스템 여유 메모리 전후값 | verified diagnostic | 실행 직전·직후 runtime snapshot | 결과 payload에는 보존하되 정리 확보량으로 표시하지 않음 |

## Section Order

1. 상태 브리핑: 현재 화면의 목적과 데이터 신선도를 먼저 설명한다.
2. CPU·메모리 hero: 시스템 전체 부하를 크게 읽고 메모리 pane에서 자체 메모리를 원클릭 정리한다.
3. 상위 프로세스: 전체 수치의 원인을 실제 행으로 비교한다.
4. 안전 안내: 종료 행동 바로 옆에서 범위와 손실 가능성을 설명한다.
5. 종료 확인 dialog: 정확한 대상 신원을 다시 보여 주고 명시적으로 확인한다.

## CTA Strategy

- Primary: `앱 메모리 정리` — macOS 메모리 pane의 큰 버튼이며 확인창 없이 현재 BroomSweepy allocator relief만 실행한다.
- Secondary: `종료 요청` — 종료 가능한 프로세스 행에서만 보이며 확인 dialog를 먼저 연다. `지금 새로 고침`은 utility 행동이다.
- 반복 규칙: 종료 행동은 각 행에 한 번, dialog 확인에 한 번만 보인다.
- 완료·실패 피드백: 메모리는 `반환량 있음`, `이미 반환할 메모리 없음`, `실패`, `지원하지 않음`을 구분하고 종료는 기존 세부 결과를 유지한다.

## Trust Strategy

- 사용자가 불안을 느끼는 지점: 다른 앱 종료와 저장하지 않은 작업 손실.
- 그 직전에 제시할 근거: 앱 이름·PID·마지막 측정 시각·정상 종료 요청임을 dialog에 표시한다.
- 출처·날짜·검증 가능성: 런타임 snapshot ID와 프로세스 시작 시각·실행 파일·소유자를 서버에서 재검증한다.
- 근거가 없을 때 생략할 요소: 신원을 확인할 수 없으면 종료 버튼 자체를 제공하지 않는다.

## Asset Provenance

| Asset | Source | Local path | License/trademark/attribution | Modification allowed | Status/fallback |
|---|---|---|---|---|---|
| 상태·성능 아이콘 | 기존 Lucide React dependency | `apps/desktop/src/views/PerformanceView.tsx` | Lucide license, 기존 프로젝트 dependency | yes | verified |
| 글래스·폰트 토큰 | 프로젝트 디자인 시스템 | `DESIGN.md` | 프로젝트 내부 | yes | verified |

## Desktop Structure

- 기준 뷰포트: 1280×820.
- 첫 뷰포트: 짧은 상태 줄, 1:1 CPU/RAM instrument, 자체 메모리 정리 행동.
- 그리드·pane·콘텐츠 위계: 공통 제목 아래 같은 크기의 CPU ring + RAM ring, RAM 사용/전체·사용 가능·스왑, 아래 단일 프로세스 panel; 위험 행동은 우측 끝.
- 스크롤 흐름과 밀도 변화: 전체 상태는 low density, 실제 프로세스는 controlled-medium으로 점차 조밀해진다.

## Mobile Transformations

| Desktop element | Operation | Mobile result | Reason |
|---|---|---|---|
| 같은 크기의 CPU ring + RAM ring | reorder | 콘텐츠 폭 560px 미만에서 CPU 다음 RAM을 세로 배치 | 큰 수치의 읽기 순서와 원 크기를 보존 |
| 메모리 정리 행동 | retain | RAM 수치 아래 전체 폭 버튼과 짧은 범위 설명 | 원클릭 과업과 44px 타깃을 보존 |
| 프로세스 table | replace | 이름·CPU·RAM·행동의 2행 카드 목록 | 가로 스크롤 없이 비교 유지 |
| PID·보조 상태 | compress | 이름 아래 한 줄 메타데이터 | 핵심 수치 우선 |
| 헤더 새로 고침 | retain | 우측 아이콘 버튼 | 오래된 수치 복구 행동 유지 |
| 종료 dialog | replace | 폭에 맞는 bottom-aligned dialog | 대상·경고·취소 버튼의 충분한 폭 확보 |

## States

| State | Trigger | User sees | Available action | Recovery |
|---|---|---|---|---|
| loading | 첫 CPU 표본 준비 | 300ms 뒤 `측정 준비 중` | 없음 | 첫 snapshot 자동 완료 |
| empty | 표시할 프로세스 없음 | 전체 수치와 빈 목록 설명 | 새로 고침 | 다음 snapshot |
| error | snapshot 실패 | 마지막 정상값은 stale, 정확한 오류 | 지금 새로 고침 | 성공 시 정상 복귀 |
| success | snapshot 완료 | 실제 수치·시각·프로세스 목록 | 정렬·종료 검토 | 주기 갱신 |
| stale | 2개 이상 주기 실패 또는 오래된 값 | `오래된 측정`과 마지막 시각 | 지금 새로 고침 | 종료 비활성화 |
| terminating | 사용자가 확인 | 대상 이름과 진행 상태 | 중복 요청 불가 | deadline 후 결과 |
| cleaning-memory | 사용자가 메모리 정리 클릭 | 버튼 spinner와 `정리 중…` | 중복 요청 불가 | 완료 뒤 자동 snapshot 갱신 |
| memory-clean-success | allocator relief 완료 | 반환 바이트 또는 이미 정리됨 | 다시 실행 가능 | 같은 자리에서 다음 실행 |
| memory-clean-error | command 실패 | 정확한 오류와 기존 측정값 | 다시 시도 | 성공 시 결과로 교체 |
| memory-clean-unsupported | macOS 외 플랫폼 | 현재 앱 정리를 지원하지 않는 설명 | 없음 | 지원 플랫폼에서 사용 |

## Performance Budget

- 첫 화면 필수 자산: 기존 폰트·Lucide SVG·한 번의 backend snapshot.
- 지연 가능한 자산: 프로세스 행은 첫 상태 card 뒤 렌더한다.
- 폰트 weight·이미지·영상·모션 예산: 신규 폰트·이미지·영상 없음; 행동 피드백 transform/opacity 180ms 이하. 사용자 요청에 따라 두 성능 링의 값 전환만 CSS stroke-dashoffset 900ms ease-out을 허용하며, polling·React 렌더 빈도는 늘리지 않는다.
- 저성능 기기와 느린 네트워크 폴백: 로컬 데이터만 사용하고 polling 중첩을 막으며 최대 40개 행만 IPC로 전달한다.

## Accessibility Contract

- 문서·랜드마크·헤딩 읽기 순서: h1은 AppShell, h2는 시스템 상태와 상위 프로세스 순서다.
- 키보드·포커스·Escape 동작: 모든 행 행동 접근 가능; dialog 취소 기본 포커스, focus trap, Escape 닫기, 닫힌 뒤 원래 버튼 복귀.
- 레이블·오류 연결·상태 알림: 새로 고침·메모리 정리·종료 결과만 polite status, 오류는 alert; 2초 수치는 매번 읽지 않는다.
- 대비·색 외 신호·터치 타깃: 두 ring에 개별 meter 레이블과 퍼센트 텍스트를 함께 표시하고 버튼은 44px 이상.
- reduced-motion과 대체 경험: entrance·값 전환을 제거해 즉시 최종 상태를 표시한다.

## Adopt

- 해당 없음 — product-derived. Swift의 넓은 카드, 큰 수치, 명시적 확인 흐름을 제품 내부 근거로 채택한다.

## Adapt

- Swift의 앱 목록을 실제 프로세스 측정으로 바꾼다. Swift `메모리 정리`의 즉시 실행 UX는 유지하되 대량 임시 할당은 제거하고 BroomSweepy 자체 allocator relief로 범위를 좁힌다.

## Avoid

- 메모리 압박용 임시 할당, 시스템 전체 확보량 주장, 하드코딩한 고사용 앱, 자동 종료, force kill, raw PID 명령, 지속적인 장식 애니메이션을 피한다.

## Prompt Contract

GOAL — 실제 CPU·RAM과 원인 프로세스를 확인하고 일반 GUI 앱 하나에 정상 종료를 요청한다.
AUDIENCE — Mac이 느리거나 메모리 상태가 궁금한 일반 사용자.
TASK — 상태 확인 → 자체 메모리 원클릭 정리 또는 프로세스 비교 → 대상 검토 → 정상 종료 요청 → 결과 확인.
FLOW — sidebar → system instrument + memory action → result → process rows → confirmation → result.
HEADER — `성능`, 설명, freshness, refresh.
MESSAGE — 느려지는 원인을 실제 수치로 확인합니다.
FACTS — runtime CPU, memory, swap, resident bytes, captured time.
CONTENT_INTEGRITY — verified runtime data only; no estimated reclaimed memory.
SECTION_ORDER — briefing, instrument, processes, trust, dialog.
CTA — hero-level `앱 메모리 정리` (BroomSweepy only); row-level `종료 요청`; utility `지금 새로 고침`.
TRUST — identity revalidation, same-user GUI only, no force fallback.
ASSETS — project fonts and Lucide only.
LAYOUT — shared heading and equal-sized CPU/RAM rings in a symmetric 1:1 hero over one process panel.
RESPONSIVE — instrument stacks; table becomes two-line rows; dialog fits viewport.
STATES — loading, empty, error, success, stale, cleaning-memory, memory-clean-result, terminating.
PERFORMANCE — persistent sampler, no overlapping poll, 40 bounded rows.
ACCESSIBILITY — semantic list/table, cancel-first dialog, focus restore, restrained live regions.
PRESERVE — native glass, near-black canvas, blue-violet ring signature, large whitespace.
EXCLUDE — memory-pressure allocation, system-wide cleanup claims, force kill, arbitrary process control, command/env collection.
SUCCESS — actual values, safe current-process memory relief, and honest one-app termination work in the installed macOS build.

## Success Checks

- 첫 5초 안에 핵심 약속과 주 행동을 설명할 수 있는가?
- 화면의 사실·수치·후기·브랜드 자산이 출처와 상태를 가지며, unverified 항목을 사실처럼 보이지 않는가?
- 주요 과업을 막는 상태·정보·행동 누락이 없는가?
- 모바일이 데스크톱 축소판이 아니라 우선순위에 맞게 재구성됐는가?
- 아름다움, 접근성, 성능 중 하나를 다른 하나의 희생으로 얻지 않았는가?
