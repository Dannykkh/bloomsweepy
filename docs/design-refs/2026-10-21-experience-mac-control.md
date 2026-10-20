# Experience Contract: Mac control — selected A implementation

## Source Mode

Mode: product-derived. 사용자 “추천대로가자”로 A · 시스템 설정형을 선택했다. 기존 제품 구성 지점에 반영하는 단일 화면 delta이며 다른 제품의 exact clone이 아니다. 비교 시안은 별도 보존한다.

## Product Facts

| Claim | Source | Captured at | Freshness/status | Allowed presentation |
|---|---|---|---|---|
| 제품의 Finder 설정은5개 | MacManagementView·mac_settings.rs | 2026-10-21 | current | 지원 범위5개만 표시 |
| 저장값 null은 OFF가 아님 | mac_settings/native.rs·설치본 조회 | 2026-10-21 | current | 저장값 없음, 실제 표시 미확인 |
| 제품은 고정 native API와 durable history 사용 | MacManagementView·macSettings.ts | 2026-10-21 | current | 기존 조회/적용/복원 연결 보존 |
| 비교 시안과 시험 fixture는 실제 OS 연결 없음 | preview·mac-settings-fixture 코드 | 2026-10-21 | prototype | 제품 성공과 모의 결과를 구별 |

## Benchmark Sources

해당 없음 — product-derived. 사용자 이미지는 복제할 목표가 아니라 피할 현재 문제의 근거다.

## Page Goal

사용자가 기능 묶음과 조작 위치를5초 안에 찾고, 예측 가능한 선택/적용/복원을 경험한다. A를 실제 Mac 설정 화면에 구현하고 설치본에서 읽기 전용 확인한다. 제품이 지원하지 않는 기능이나 OS 변경 성공을 주장하지 않는다.

## Audience and Tasks

개인 Mac 사용자1명. 현재5개 저장값을 읽고 [끄기/켜기]로 초안을 고른 뒤 [적용]한다. 처음에는 저장값 없음이므로 양쪽 모두 미선택이다. 재확인 전 실패·관리 상태를 끄짐으로 오인하거나 단순 메뉴 이동을 설정 실행으로 착각하지 않도록 한다.

## System Roles

NOT APPLICABLE: single role. AI가 관리자 역할을 갖지 않는다.

## Role Variants

NOT APPLICABLE: single role.

## Header and Navigation

기존 AppShell 내비게이션을 재사용하며 큰 제목은1개만 둔다. Mac 화면 헤더는 sticky를 해제해 내용 위에 겹치지 않는다. 파일 표시/경로와 정보/변경 기록은 화면 내부의 실제 내용이며 다른 메인 메뉴로 보내는 기능 카드는 제외한다. narrow에서는 기존 overlay 내비게이션 계약을 유지한다. 제품에는 시안 toolbar·가짜 sidebar·프리뷰 안내를 넣지 않는다.

## Core Message

Finder에서 무엇을 보이게 할지 고른다. 설명보다 제목·현재 상태·선택·적용을 먼저 읽는다. 상태/완료는 텍스트와 버튼 상태로 전달한다.

## Content Integrity

| Content item | Classification | Evidence | Presentation rule |
|---|---|---|---|
| 설정명5개 | verified | 실제 제품 코드 | 이름을 축약해도 의미 유지 |
| 현재 저장값과 관리/오류 상태 | verified | 현재 native snapshot | null≠false, 미지원≠오류; 추정하지 않음 |
| 적용/복원 결과 | verified | 기존 native API 반환 snapshot | 저장 확인과 Finder 화면 반영을 분리 |
| 시험 결과 | prototype | mock IPC fixture | 실제 설정 변경·검증으로 주장하지 않음 |

## Section Order

1. 단일 제목·새로 확인·변경 기록 진입과 짧은 결과 상태.
2. 기능 묶음과5개 조작: 최초 뷰포트의 핵심.
3. 변경 기록/복원: 찾아 헤매지 않는 독립 진입.
4. 설정 설명: 추가 설명만 details로 접음. 현재값/권한/실행결과는 접지 않음.

## CTA Strategy

Primary: 선택한 항목의 `적용`만 기존 native write를 실행한다. `[끄기][켜기]`는 초안만 바꾸며 현재값과 같으면 적용되지 않는다. null일 때 `미선택`, 변경되면 `아직 적용 안 됨`을 명시한다. Secondary: `변경 기록`, `복원`은 해당 화면 내에서 처리한다. 다른 메뉴 연결 CTA는0개다. 성공/실패는 role=status/alert에 표시한다. 기록 진입은 해당 제목으로 이동하고 키보드 포커스도 보낸다.

## Trust Strategy

저장값 없음≠꺼짐을 표기하며 실제 Finder 화면 반영과 저장 확인을 혼동하지 않는다. 현재값 충돌·중복 실행 ref lock·늦은 응답 generation·실패 후 재조회 보호를 보존한다. 관리/형식오류/조회실패는 해당 행에 드러내고 조작을 잠근다. 근거 없는 건강점수·확보용량·지원 기능은 표시하지 않는다. 디자인 테스트로 사용자 OS설정·DB·권한을 변경하지 않는다.

## Asset Provenance

| Asset | Source | Local path | License/trademark/attribution | Modification allowed | Status/fallback |
|---|---|---|---|---|---|
| 아이콘 | 기존 lucide-react | node_modules/lucide-react | 기존 프로젝트 라이선스 유지 | yes | 기존 라이브러리만 |
| 한글/라틴 폰트 | 프로젝트 Pretendard Variable | apps/desktop/src/assets/fonts/PretendardVariable.woff2 | 기존 프로젝트 라이선스 유지 | no | local load, sans fallback |
| 원본 화면 | 이번 실제 native capture | docs/qa/mac-design-review-2026-10-21/ | 비개인 합성 시험 화면 | no | 원본 보존 |

## Desktop Structure

1280×820 대표, 기존 sidebar와 나머지 기능 작업면. 글은14px 이상. 선택한 A의 파일 표시3개/경로와 정보2개 그룹 행을 사용한다. 현재값·초안선택·적용이 정렬되고 최초 화면에5개 조작이 보이는 것을 목표로 한다. 중복 제목과 큰 설명 배너는 없다. 그룹 표면 외 중첩 blur·장식 gradient를 추가하지 않는다.

## Mobile Transformations

| Desktop element | Operation | Mobile result | Reason |
|---|---|---|---|
| 전역 sidebar | replace | 기존 overlay 내비게이션 | 현재 앱 탐색 계약 유지 |
| 기능 묶음 | reorder | 제목→행동→보조 설명 | 주 행동 보존 |
| 가로 조작 | replace |44px 선택·적용을 다음 행 | 작은 터치 방지 |
| 긴 설명 | collapse | 설정 설명 details | 상태는 그대로, 설명만 필요 시 |
| 변경 기록 | retain | 동일한 진입과 복원 | 원복 재발견 보장 |

## States

| State | Trigger | User sees | Available action | Recovery |
|---|---|---|---|---|
| loading | native 조회/apply/restore | 처리 중 | 중복 잠금 | 결과 대기 |
| empty | 최초 null/history 없음 | 저장값 없음/기록 없음 | 선택 가능 | 초안 선택 |
| error | 읽기/쓰기/복원 실패·현재값 충돌 | 오류·재확인 필요 | 변경 잠금, 새로 확인 | 성공 조회 후 다시 선택 |
| success | native 적용/복원 snapshot | 저장 확인·화면 반영은 별개 | 기록/다음 선택 | 충돌 검증 후 복원 |
| pending | 값 선택 | 아직 적용 안 됨 | 적용 | 선택 취소 |
| managed/invalid | 관리됨/형식불일치/조회오류 | 행별 원인 | 읽기만 | 새로 확인 |
| unsupported | 비지원 snapshot | 지원하지 않음 | 쓰기 없음 | 해당 OS 기능 사용 |

## Performance Budget

기존 React/Lucide/local font/native API만. 새 의존성·이미지·영상·원격 AI0. 기존 mount시1회 조회와 명시 새로 확인만 유지하며 polling을 추가하지 않는다. 장식 효과0, 모션은 버튼 opacity/transform 피드백만.390px은 좁은 창 검증이며 실제 모바일 제품으로 주장하지 않는다.

## Accessibility Contract

nav/main/section과 제목 순서, 모든 버튼 실제 텍스트/접근성 이름, aria-pressed로 초안 상태 전달. 포커스 가시성·44px 타깃·색 이외 상태 라벨. reduced-motion에서 전환 제거. 전체 WCAG 준수·실제 스크린리더 검증으로 확대하지 않는다.

## Adopt

제품의 기존 토큰·현재값 보호·명시 적용·변경 기록.

## Adapt

중복 경고문을 짧은 상태/보조 설명으로 분리하고 기능 조작을 전면 배치한다.

## Avoid

가짜 토글 상태, 기능처럼 보이는 메뉴 바로가기, 미구현 설정의 활성 카드, 중복 제목, 떠있는 큰 설명, 투명 레이어 중첩, 장식용 AI 그라데이션.

## Prompt Contract

GOAL — Finder 설정을 기능 묶음과 직접 조작으로 이해.
AUDIENCE — 개인 Mac 사용자1명.
TASK — 현재 상태→초안 선택→명시 적용→기록에서 복원.
FLOW — 기존 조회→초안선택→명시native적용→history복원, fixture만 mock.
HEADER — 큰 제목1개, 기존sidebar, Mac헤더 비sticky.
MESSAGE — 설명보다 기능과 조작이 먼저.
FACTS —5개 지원 범위, null≠false.
CONTENT_INTEGRITY — native상태만 실제로, mock검증과 실제OS쓰기 구분.
SECTION_ORDER — 현재 위치→기능/조작→변경 기록→보조 설명.
CTA — 적용·변경 기록·복원, 메인 메뉴 이동 없음.
TRUST — 정확한 저장 상태, 명시 적용, 권한 확대 없음.
ASSETS — 기존 Lucide/local Pretendard만.
LAYOUT — 사용자선택 A grouped rows, 파일표시3/경로정보2.
RESPONSIVE — 기존overlay내비·행동재배치·설명만접기·복원보존.
STATES — null/pending/loading/success/empty/error/managed/unsupported/lateunmount.
PERFORMANCE — 새 패키지·AI·원격 자산·polling0.
ACCESSIBILITY —14px/44px/visiblefocus/aria-pressed/status/reducedmotion.
PRESERVE — DB/권한/dirty변경·native安全계약, 설치교체시rollback.
EXCLUDE — 신규OS기능·Windows변경·테스트용실제OS쓰기·가짜지원판정·메뉴연결위장.
SUCCESS — A제품구현, fixture상태/좁은창검증, 설치본읽기전용확인.

## Success Checks

- 주 기능과 조작이 설명보다 먼저 읽히는가?
- 초기 null에 켜짐/꺼짐 어느 것도 선택되지 않았는가?
- 선택만으로 적용되지 않으며 기록/복원을 찾을 수 있는가?
-390px에서도 행동/상태가 잘리거나 작은 타깃으로 바뀌지 않는가?
- 중복 제목·스크롤 겹침·위장된 메뉴 연결이 제거됐는가?
- 모의 적용/복원과 실제 설치본의 읽기 전용 확인을 명확히 구분했는가?
