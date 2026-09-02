# Experience Contract: 언어 선택과 앱 아이콘 식별성

## Source Mode

- Mode: product-derived
- Evidence: 사용자 피드백, `DESIGN.md`, 현재 `SettingsView`, Windows 작업 표시줄용 32px 아이콘 점검

## Product Facts

| Claim | Source | Captured at | Freshness/status | Allowed presentation |
|---|---|---|---|---|
| 현재 데스크톱 앱은 한국어 문구를 직접 렌더하며 언어 선택을 저장하지 않는다 | `apps/desktop/src`, `apps/desktop/src/views/SettingsView.tsx` | 2026-09-03 | current | 언어 설정 추가 전 상태 설명에만 사용 |
| Windows와 macOS용 기존 아이콘은 최초 추가 뒤 같은 원본을 유지했다 | Git blob history for Tauri and Swift icon assets | 2026-09-03 | current | 기존 아이콘의 빗자루 식별성 개선 근거로 사용 |
| 기존 32px 아이콘에서는 가는 손잡이와 빗살보다 별 장식이 먼저 보인다 | `apps/desktop/src-tauri/icons/32x32.png` 실제 렌더 | 2026-09-03 | verified | 작은 크기 보정의 성공 조건으로 사용 |

## Benchmark Sources

- 해당 없음 — product-derived. 외부 제품의 언어 선택이나 아이콘을 복제하지 않는다.

## Page Goal

- 사용자가 이 화면에서 달성할 결과: 설정에서 English, 한국어, 日本語, 简体中文 중 하나를 선택하고 즉시 전체 앱에 적용한다.
- 제품이 얻어야 하는 결과: 언어별 설치본을 분리하지 않고 한 설치본의 접근성과 배포 일관성을 유지한다.
- 관찰 가능한 성공 조건: 첫 실행은 English이고 재시작 뒤 선택이 유지되며, 16~32px 아이콘에서 빗자루가 먼저 보인다.

## Audience and Tasks

- 주요 사용자와 사용 상황: 한국어 또는 영어 Windows/macOS 환경에서 저장공간을 확인하는 개인 사용자.
- 최우선 과업: 설정에서 언어를 이해 가능한 이름으로 고르고 곧바로 결과를 확인한다.
- 시작 조건과 완료 조건: 기존 앱을 실행한 상태에서 시작하며 전역 메뉴와 현재 화면 문구가 선택 언어로 바뀌면 완료한다.
- 주요 불안·마찰·실패 가능성: 시스템 언어와 앱 언어 혼동, 일부 화면만 번역되는 혼합 언어, 재시작 시 초기화, 작은 아이콘에서 브랜드 대상 식별 실패.

## Header and Navigation

- 브랜드·현재 위치·전역 이동·주 행동의 순서: 기존 구조를 보존하고 내비게이션 라벨만 선택 언어로 전환한다.
- 데스크톱 내비게이션: 기존 전역 메뉴 수와 순서를 바꾸지 않는다.
- 모바일 대체 구조: 기존 920px 레일과 680px 오버레이 구조를 보존한다.

## Core Message

- 핵심 약속: 한 설치본에서 English, 한국어, 日本語, 简体中文 표시를 즉시 바꿀 수 있다.
- 설명: 별도 선택이 없는 첫 실행은 English를 사용한다.
- 증거: 현재 적용 언어를 설정 행에 평문으로 표시하고 HTML `lang` 속성도 함께 갱신한다.
- 사용자가 다음에 이해해야 할 것: 언어 선택은 이 컴퓨터에만 저장되며 검사 결과나 파일에는 영향을 주지 않는다.

## Content Integrity

| Content item | Classification | Evidence | Presentation rule |
|---|---|---|---|
| 현재 적용 언어 | verified | 저장된 사용자 선택과 English 기본값 | 설정 행에 실제 적용 결과만 표시 |
| 앱 아이콘 | verified | 생성 결과와 16/32/128px 파생 렌더 | 실제 번들 자산과 같은 결과만 표시 |

## Section Order

1. 표시 언어: 앱 전체의 이해 가능성에 영향을 주므로 설정 첫 영역에 둔다.
2. 기존 검사 설정: 파일 크기 기준과 결과 한도를 현재 순서대로 유지한다.
3. 선택 기능과 안전 계약: Docker, MCP, 안전 설명의 기존 순서를 유지한다.

## CTA Strategy

- Primary: 별도 저장 버튼 없이 언어 선택 자체가 즉시 적용 행동이다.
- Secondary: 해당 없음.
- 반복 규칙: 언어 선택은 설정 화면 한 곳에만 둔다.
- 완료·실패 피드백: 선택값과 현재 적용 언어가 즉시 바뀌며 저장 실패 시 현재 세션에는 적용하고 다음 실행 유지 실패를 설명한다.

## Trust Strategy

- 사용자가 불안을 느끼는 지점: OS 설정을 바꾸는지, 검사나 파일에 영향을 주는지.
- 그 직전에 제시할 근거: `이 앱의 표시만 바뀝니다`와 로컬 저장 범위를 같은 행에 표시한다.
- 출처·날짜·검증 가능성: localStorage 값과 English 기본값, 2026-09-03 구현 검증.
- 근거가 없을 때 생략할 요소: 자동 번역 품질, 서버 동기화, 계정별 동기화를 암시하지 않는다.

## Asset Provenance

| Asset | Source | Local path | License/trademark/attribution | Modification allowed | Status/fallback |
|---|---|---|---|---|---|
| 기존 앱 아이콘 | 프로젝트 내부 원본 | `BroomSweepy/Resources/Assets.xcassets/AppIcon.appiconset/` | 프로젝트 소유 자산 | yes | verified |
| 굵은 빗자루 아이콘 보정본 | 기존 원본을 built-in image generation으로 편집 | `apps/desktop/src-tauri/icons/app-icon-master.png` | 프로젝트 자산의 생성형 편집본 | yes | 검증 뒤 번들 파생본 생성 |

## Desktop Structure

- 기준 뷰포트: 1280×820, 최소 창 760×600.
- 첫 뷰포트: 설정 화면 첫 행에서 언어 선택과 현재 적용 결과를 확인한다.
- 그리드·pane·콘텐츠 위계: 기존 단일 설정 열과 `setting-row` 정렬을 재사용한다.
- 스크롤 흐름과 밀도 변화: 신규 독립 카드 대신 첫 설정 패널 하나만 추가하고 나머지 순서를 밀어낸다.

## Mobile Transformations

| Desktop element | Operation | Mobile result | Reason |
|---|---|---|---|
| 언어 설명 + 선택 | reorder | 설명 뒤 44px 전폭 select | 760px 최소 창과 좁은 폭에서 읽기 순서 유지 |
| 현재 적용 언어 보조 문구 | retain | 선택 아래 한 줄 | 실제 적용 결과를 숨기지 않음 |

## States

| State | Trigger | User sees | Available action | Recovery |
|---|---|---|---|---|
| loading | 앱 시작 시 저장값 해석 | 저장 언어 또는 English로 즉시 렌더 | 없음 | 유효하지 않은 저장값은 English로 복구 |
| empty | 해당 없음 | 해당 없음 | 해당 없음 | 해당 없음 |
| error | 저장소 접근 실패 | 현재 세션 언어와 유지 실패 설명 | 다시 선택 | 다음 실행은 English로 복구 |
| success | 언어 선택 완료 | 전체 UI와 현재 적용 언어가 즉시 변경 | 다른 언어 선택 | 재시작 뒤 같은 선택 유지 |

## Performance Budget

- 첫 화면 필수 자산: 네 언어의 텍스트 사전과 현재 아이콘만 번들에 포함한다.
- 지연 가능한 자산: 없음.
- 폰트 weight·이미지·영상·모션 예산: 신규 폰트와 런타임 번역 의존성 0, 아이콘 파생본은 기존 크기 범위를 유지한다.
- 저성능 기기와 느린 네트워크 폴백: 모든 번역과 아이콘은 로컬 번들에서 제공한다.

## Accessibility Contract

- 문서·랜드마크·헤딩 읽기 순서: 기존 설정 헤딩 순서를 유지하며 언어 설정을 첫 번째로 읽는다.
- 키보드·포커스·Escape 동작: 기본 select를 Tab과 화살표로 조작하고 변경 뒤 포커스를 유지한다.
- 레이블·오류 연결·상태 알림: select는 가시적 label을 가지며 실제 적용 언어 설명과 연결한다.
- 대비·색 외 신호·터치 타깃: 14px 이상, 44px 컨트롤, 텍스트로 선택 상태를 표시한다.
- reduced-motion과 대체 경험: 언어 변경에 장식 애니메이션을 추가하지 않는다.

## Adopt

- 기존 설정 행, 단층 표면, 14px 최소 글자 규칙을 그대로 사용한다.

## Adapt

- 첫 실행 English와 네 언어의 자체 언어명 표기를 사용해 현재 언어를 모르는 상태에서도 바꿀 수 있게 한다.
- 기존 아이콘 팔레트는 유지하되 작은 크기에서는 장식보다 빗자루 실루엣을 우선한다.

## Avoid

- 언어별 설치 파일을 분리하지 않는다.
- 언어 변경에 재시작을 요구하지 않는다.
- 번역되지 않은 화면을 영어 모드에 한국어로 남겨 성공처럼 표시하지 않는다.
- 아이콘에 글자, 미세한 빗살, 중앙을 가리는 큰 별을 넣지 않는다.

## Prompt Contract

GOAL — 한 설치본에서 English·한국어·日本語·简体中文 표시를 즉시 전환하고 작은 아이콘에서도 빗자루를 식별한다.
AUDIENCE — 영어, 한국어, 일본어, 중국어 Windows/macOS 사용자.
TASK — 설정에서 언어 선택, 현재 적용 결과 확인, 재시작 유지.
FLOW — 설정 진입 → 언어 선택 → 전체 UI 즉시 변경 → 재시작 유지 확인.
HEADER — 기존 설정 화면과 전역 내비게이션 구조 보존.
MESSAGE — 이 앱의 표시 언어만 바뀌며 파일과 OS 설정은 바뀌지 않는다.
FACTS — 저장된 선택, English 기본값, 실제 번들 아이콘만 사용.
CONTENT_INTEGRITY — 부분 번역이나 자동 서버 번역을 완성 기능처럼 표시하지 않는다.
SECTION_ORDER — 언어 → 검사 기준 → 결과 한도 → 선택 기능 → 안전 계약.
CTA — 가시적 label이 있는 native select 한 개.
TRUST — 로컬 저장과 비파괴 범위를 선택 바로 옆에 설명.
ASSETS — 프로젝트 원본 기반으로 보정한 굵은 빗자루 아이콘.
LAYOUT — 기존 단일 설정 열과 setting-row 재사용.
RESPONSIVE — 좁은 폭에서 설명과 44px select를 세로 배치.
STATES — 유효 저장값, English 기본값, 저장 실패, 즉시 적용.
PERFORMANCE — 번역 런타임 의존성 0, 로컬 사전만 포함.
ACCESSIBILITY — HTML lang 동기화, keyboard select, 14px, 44px.
PRESERVE — 다크 글래스, 메뉴 순서, 설정 밀도, 기존 아이콘 팔레트.
EXCLUDE — 별도 언어 설치본, 재시작 요구, 언어별 기능 차이, 가는 빗자루.
SUCCESS — 네 언어에서 주요 화면이 섞이지 않고 16/32px에서 빗자루가 먼저 보인다.

## Success Checks

- 첫 5초 안에 핵심 약속과 주 행동을 설명할 수 있는가?
- 화면의 사실·수치·후기·브랜드 자산이 출처와 상태를 가지며, unverified 항목을 사실처럼 보이지 않는가?
- 주요 과업을 막는 상태·정보·행동 누락이 없는가?
- 모바일이 데스크톱 축소판이 아니라 우선순위에 맞게 재구성됐는가?
- 아름다움, 접근성, 성능 중 하나를 다른 하나의 희생으로 얻지 않았는가?
