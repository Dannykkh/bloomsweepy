# Experience Contract: Workspace navigation

## Source Mode

- Mode: product-derived
- Evidence: 2026-10-20 사용자 승인, 직전 native 설치 화면 감사, 현재 AppShell/App/SettingsView/DashboardView.

## Product Facts

| Claim | Source | Captured at | Freshness/status | Allowed presentation |
|---|---|---|---|---|
| Mac 설정은 Finder5개 | mac_settings.rs/MacManagementView | 2026-10-20 | current source | 전체 OS/보안 관리로 과장하지 않음 |
| 파일/정리 작업은 앱 엔진 수행 | App.tsx/common dispatcher | 2026-10-20 | current source | AI는 결과 해석, 승인 경계 유지 |
| 설치본과 개발본은 다름 | native UI 감사/미설치 소스 | 2026-10-20 | current | fixture를 설치형 성공으로 표시하지 않음 |

## Benchmark Sources

해당 없음 — product-derived. 기존 앱과 사용자 선택 glass 방향을 보존한다. 외부 앱 픽셀 복제가 아니다.

## Page Goal

- 사용자가 현재 상태와 다음 작업을 구분하고 필요한 화면/설정에 바로 도달한다.
- 기존 기능 누락 없이 정보 구조를 일관되게 만든다.
- 1280×820 첫 화면에서 주 검사 행동/빠른 작업, 설정6분류의 현재 위치가 보인다.

## Audience and Tasks

- 단일 개인 사용자, 8GiB Mac 및 Windows 지원을 고려한다.
- 용량 원인 찾기/정리 검토/이름·내용 검색/앱·성능·Finder 관리/앱 설정을 구분한다.
- 동일 의미 버튼은 동일 목적지, 이동은 검사나 권한 변경을 실행하지 않는다.
- 주 불안은 불명확한 범위와 자동 삭제이며 기존 검증을 보존한다.

## System Roles

NOT APPLICABLE: single role. AI는 OS 관리자가 아니며 기존 공개·변경 경계를 유지한다.

## Header and Navigation

- 기존 sidebar를 컴퓨터/파일과 공간/도구로 묶고 앱 설정은 유틸리티로 분리한다.
- dashboard→performance→applications 순서 유지. macos-management는 Mac에서만 노출하고 이름/설명을 Finder 범위에 맞춘다.
- storage/file leaf에서도 부모 aria-current가 유지된다.
- 좁은 창은 기존 icon rail/overlay, 설정 세로 분류는 줄바꿈 가능한 가로 선택으로 바꾼다.

## Core Message

- 현재 상태를 보고 필요한 작업을 고른다.
- 설명은 짧게, 실제 수치·측정 시각·제약은 해당 작업 옆에 둔다.
- CLI 설치 설명을 전역 메뉴의 기능 설명으로 쓰지 않는다.
- 시스템 설정과 앱 사용 환경, 캐시 후보와 폴더 선택 정리를 구분한다.

## Content Integrity

| Content item | Classification | Evidence | Presentation rule |
|---|---|---|---|
| 프리뷰256GB/24GB | prototype | ui-layout-preview 합성 데이터 | Synthetic preview 고지, 제품 값으로 복제하지 않음 |
| 실제 저장공간/결과 수 | verified | runtime props | 미측정/부분/오래됨 그대로 표시 |
| Windows 새 설정 기능 | hypothesis | Windows 연구 | 새 메뉴/가짜 토글로 노출하지 않음 |

## Section Order

1. 대시보드: 짧은 제목/현재 성능 → compact drive deck+검사 행동 → 빠른 작업 → 기존 결과/활동.
2. 설정: 분류 선택 → 선택한 분류 제목/짧은 설명 → 기존 패널/동일 콜백. 다른 분류는 화면·접근성에서 숨긴다.
3. 공간 정리: 현재 범위 → 네 하위 작업 → 해당 결과/빈 상태 → 휴지통 보조 행동. 시스템/폴더 후보 전환 명확히.

## CTA Strategy

- Primary: 대시보드의 기존 검사 시작/다시 스캔 행동. 실제 측정 대상을 바꾸는 기존 확인 흐름 유지.
- Secondary: 정리 후보/파일 찾기/결과 이동. 화면 이동만 수행한다.
- 반복: 같은 섹션에 같은 경로 카드와 버튼을 중복하지 않는다.
- 완료·실패: 기존 상태/결과/재시도. 근거 없는 성공·확보량 없음.

## Trust Strategy

- 검사/휴지통/OS 변경 직전 불안을 다룬다.
- 삭제·재검증·복원 경계를 작업 근처에 유지한다. 앱 설정 정보에는 전체 안전 계약을 보존한다.
- 저장값과 effective OS 상태를 혼동하지 않는다.
- 연결 안 됨을 성공 녹색으로 표현하지 않는다.

## Asset Provenance

프로젝트 내부 로고·Lucide 아이콘·Pretendard Variable·JetBrains Mono만 사용. 새 외부/생성 자산 없음. 기존 출처·라이선스를 변경하지 않는다.

## Desktop Structure

- 기준1280×820 및1440×900. sidebar220px 안팎, 본문 유연 폭.
- 대시보드 hero를 축소해 빠른 행동이 첫 화면에 등장. 링은 signature지만 같은 용량의 반복 표기를 줄인다.
- 설정은 왼쪽 분류/오른쪽 작업 패널, 카드 안에 카드가 반복되지 않도록 배경·divider 위계 사용.
- 정밀 결과는 기존 bounded 행/페이지를 유지한다.

## Mobile Transformations

| Desktop element | Operation | Mobile result | Reason |
|---|---|---|---|
| 전역 sidebar | replace | 기존 overlay navigation | 본문 폭과 현재 위치 보존 |
| 설정 분류 | reorder | 작업면 상단 줄바꿈 버튼 | 모든6분류 발견 가능 |
| drive deck | compress | 링·수치 축소, 행동 먼저 | 핵심 행동 도달 거리 |
| 빠른 작업 | reorder | 1열 짧은 행 | 작은 화면 식별/44px |
| 보조 이력 | defer | 주 작업 아래 | 첫 화면 집중 |
| 결과 표 | retain | 기존 pagination/세부 보기 | 비교·성능 보존 |

## States

| State | Trigger | User sees | Available action | Recovery |
|---|---|---|---|---|
| loading | 기존 검사/조회 | 기존 단계·취소·busy | 취소/다른 읽기 화면 | 완료 또는 기존 오류 |
| empty | 미검사/지도 없음 | 미생성임을 명시 | 기존 지도 생성/폴더 선택 | 실제 결과로 전환 |
| error | 기존 패널 실패 | 원인·재시도 | 기존 재시도만 | 자동 우회 없음 |
| success | 기존 결과 | 실제 수치·기준 시각 | 다음 화면 | stale 구별 |
| permission | 권한 부족 | 기존 권한 안내 | 명시 설정 | 새 권한 자동 부여 없음 |
| stale | 범위/파일 변경 | 이전값과 만료 표시 | 재검사 | 기존 generation 유지 |

## Performance Budget

- 새 의존성·폰트·이미지·무한 모션 없음. CSS와 기존 컴포넌트 조합만 사용.
- 설정 탭마다 CLI 재탐색/패널 재마운트를 추가하지 않는다.
- 기존 bounded 결과·취소·메모리 정책을 보존한다. 새 window/WebView 만들지 않음.
- 기존 bundle 경고와 fixture 측정은 구분한다. 장시간 RAM/설치형 수치를 추정하지 않는다.

## Accessibility Contract

- nav/main/heading 순서, 부모 메뉴 현재 위치, 분류 버튼 aria-current/aria-controls.
- 키보드 Tab/Enter/Space, 선택 후 분류 제목으로 명시적 포커스 처리 또는 포커스 유지. hidden 패널 tabbable 금지.
- 44px target,14px 이상 글자, 새 상태는 텍스트+의미색.
- 기존 reduced-motion/투명도 감소 폴백 유지. 좁은 화면 가로 넘침0 확인.
- disabled control를 선택한 분류에서 숨겨 접근성을 피하는 방식으로 처리하지 않는다.

## Adopt

기존 native glass·드라이브 링·실제 상태와 결과 엔진.

## Adapt

기능 누적 메뉴를 작업별 그룹으로, 긴 설정을6분류로, 대시보드를 실제 행동이 보이는 비례로 조정.

## Avoid

새 OS 기능이 완성된 듯한 빈 메뉴, 가짜 용량/성공, 자동 권한/삭제, 새 재질·장식으로 구조 문제 가리기.

## Prompt Contract

GOAL — 기존 기능을 찾고 실행하는 경로 정렬.
AUDIENCE — 저장공간 부족을 겪는 개인 Mac/Windows 사용자.
TASK — 상태→검사/검색→검토→기존 실행 결과.
FLOW — 동명 동선 통일, tree/system 모드 명시.
HEADER — 앱 설정 제목, Finder 범위가 드러나는 Mac 메뉴.
MESSAGE — 기능·범위·다음 행동을 짧게.
FACTS — runtime 값을 그대로 사용.
CONTENT_INTEGRITY — 합성 프리뷰는 고지, 실제 앱에 예시값 없음.
SECTION_ORDER — compact상태→검사행동→빠른작업→근거.
CTA — 기존 검사 primary 하나, 설정은 기존 즉시 콜백.
TRUST — 승인·공개·재검증·원복 경계 보존.
ASSETS — 기존 아이콘/폰트/로고만.
LAYOUT — 기존 shell와 분류/작업면.
RESPONSIVE — overlay/분류재배치/행동우선.
STATES — 기존 busy/empty/error/success/permission/stale 유지.
PERFORMANCE — 새 의존성/패널 중복 마운트/무한 모션 없음.
ACCESSIBILITY — 현재 위치/44px/hidden/키보드/글꼴 확인.
PRESERVE — App 데이터·generation·선택·승인·엔진·chat.
EXCLUDE — OS기능 추가/자동정리/설치/커밋/푸시.
SUCCESS — 기존 과업 경로·프런트 회귀·실제1280/390렌더 통과.

## Success Checks

- 메뉴명·목적지 일치, Mac 캐시 진입이 system mode, tree 경로는 tree 유지.
- 설정6분류 접근/기존 상태보존/숨은컨트롤 비탐색.
- 대시보드 주요 행동과 빠른 작업 첫 화면,390px overflow0.
- unit/type/build와 실제 컴포넌트 상호작용을 실행하되 설치형으로 과장하지 않음.
