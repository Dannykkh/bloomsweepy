# Experience Contract: 변경 파일 검토

## Source Mode

- Mode: product-derived
- Evidence: 2026-10-08 사용자 요청 “감지는 한다… 파일 검토해보고 넘길건 넘기는… 옵션으로”; 기존 DESIGN.md/Settings.

## Product Facts

| Claim | Source | Captured at | Freshness/status | Allowed presentation |
|---|---|---|---|---|
| 기존 최근 파일은 수동 catalog 갱신의 새 관측 결과 | file_catalog.rs / App.tsx | 2026-10-08 | current | 실시간 감시와 구분 |
| 새 루틴은 metadata 비교이며 백신 아님 | 이 기능 계약 | 2026-10-08 | implementation pending | 악성 여부·안전 보장 금지 |

## Benchmark Sources

해당 없음 — product-derived. 기존 Settings 방향을 국소 확장한다.

## Page Goal

- 사용자가 이 화면에서 달성할 결과: 선택 폴더의 변경을 주기적으로 확인하고 검토 사유를 읽어 넘김 처리.
- 제품이 얻어야 하는 결과: 앱 소유 사실을 제공하되 자원 사용과 자동 외부 공개를 늘리지 않는다.
- 관찰 가능한 성공 조건: OFF → 폴더 선택 → ON → baseline → 변경 목록 → 정확한 항목 넘김 → 변경 시 재검토.

## Audience and Tasks

- 주요 사용자와 사용 상황: 저장공간과 메모리가 부족한 개인 Mac/Windows 사용자.
- 최우선 과업: 옵션을 켜고 변경 목록의 검토 사유 확인.
- 시작 조건과 완료 조건: 기본 OFF; 선택한 로컬 폴더 하나에만 적용. 검토 완료는 해당 변경의 기록만 변경.
- 주요 불안·마찰·실패 가능성: 백신으로 오해, 자동 삭제/업로드, 메모리 누적, 미완료를 변경 없음으로 오해.

## Header and Navigation

- 브랜드·현재 위치·전역 이동·주 행동의 순서: 기존 설정 화면에서 변경 파일 검토 섹션.
- 데스크톱 내비게이션: 기존 전역 내비 보존, 새 전역 메뉴 없음.
- 모바일 대체 구조: 기존 오버레이 내비 보존.

## Core Message

- 핵심 약속: 선택한 폴더의 변경을 확인하고 검토할 항목을 모은다.
- 설명: 주기 비교이며 실시간 감시/백신 아님. 내용 전송·자동 삭제 없음.
- 증거: 마지막 완전 검사 시각, 비교 기준 항목 수, 항목별 변경/검토 사유.
- 사용자가 다음에 이해해야 할 것: 넘김은 악성 여부나 안전 판정이 아니며 다시 바뀌면 재검토.

## Content Integrity

| Content item | Classification | Evidence | Presentation rule |
|---|---|---|---|
| 검토 항목과 집계 | verified | 실제 Rust 결과 | 임의 위험 점수/안전 배지 금지 |
| fixture 항목 | prototype | 합성 테스트 데이터 | 테스트 화면만, 제품 데이터로 승격 금지 |
| 실행형/이중 확장자/대량 변경 | hypothesis | 단순 로컬 규칙 | 검토 사유이지 악성 확정이 아님 |

## Section Order

1. 제목·한계 설명·ON/OFF: 행동의 의미를 먼저 알려준다.
2. 대상 폴더·주기·지금 확인: 범위와 시점을 명시한다.
3. 상태·마지막 검사·한도: 변경 목록의 신뢰도를 보여준다.
4. 검토 필요/넘김 필터·항목 목록·페이지: 근거 읽기와 정확한 항목 처리를 이어준다.

## CTA Strategy

- Primary: 대상 미선택 시 “폴더 선택”, 활성 상태에서는 “지금 확인”.
- Secondary: 행의 “넘김” / “다시 검토”; 실행 파일 열기 없음.
- 반복 규칙: primary는 섹션에 하나, 행은 조용한 secondary.
- 완료·실패 피드백: 저장/검사 실패와 다음 행동, 처리 완료를 그 자리에서 표시.

## Trust Strategy

- 사용자가 불안을 느끼는 지점: 옵션 ON, 파일 행의 검토 라벨.
- 그 직전에 제시할 근거: 선택 경로/간격, 메타데이터 전용/자동 업로드·삭제 없음.
- 출처·날짜·검증 가능성: 실제 마지막 검사 시각과 완료 여부; root 변경·재시작 시 새 baseline.
- 근거가 없을 때 생략할 요소: 바이러스/안전 점수, 랜섬웨어 예방/실시간 감시 주장.

## Asset Provenance

해당 없음 — 프로젝트 내부 폰트와 기존 lucide 아이콘만 사용. 신규 외부/생성 자산 없음.

## Desktop Structure

- 기준 뷰포트: 1280×820, 1440×900.
- 첫 뷰포트: 현재 Settings 기존 연결/모델 섹션 보존, 새 섹션은 파일 관련 설정 앞 full width.
- 그리드·pane·콘텐츠 위계: heading/toggle → folder/action → period → status strip → dense rows.
- 스크롤 흐름과 밀도 변화: 본문 스크롤, 목록 페이지20행 이하; 중첩 blur 없음.

## Mobile Transformations

| Desktop element | Operation | Mobile result | Reason |
|---|---|---|---|
| 제목 옆 toggle | retain | 제목 아래 한 행 | 옵션 의미와 상태를 유지 |
| 경로 + 버튼 | reorder | 경로 줄바꿈 뒤 full-width 버튼 | 긴 경로에서 버튼이 잘리지 않음 |
| 항목 다열 행 | compress | 이름/변경 → 상대 경로 → 사유/크기 → 행동 | 검토 근거와44px 타깃 유지 |
| 보조 한도 설명 | collapse | details | 기본 조작을 먼저 읽게 함 |

## States

| State | Trigger | User sees | Available action | Recovery |
|---|---|---|---|---|
| loading | 초기 조회/저장/검사 | 단계와 aria-busy | OFF/재시도 | 실제 상태 재조회 |
| empty | OFF/첫 baseline/변경 없음 | 각각 다른 설명 | 폴더 선택/켜기/지금 확인 | baseline 후 다음 확인 |
| error | 권한/저장/한도/자원 부족 | 검사 미완료와 이전 결과 시각 | 더 작은 폴더/재시도/OFF | 완전 검사만 baseline 반영 |
| success | 완전 검사 | 검토 필요/넘김 수, 시각, 항목별 사유 | 넘김/다시 검토/필터/페이지 | 같은 경로 변경 시 새 항목 |
| unavailable | 웹 preview | 설치 앱에서만 사용 | 없음 | fixture에서만 합성 상태 |

## Performance Budget

- 첫 화면 필수 자산: 기존 CSS/폰트/아이콘, native status20행.
- 지연 가능한 자산: 행 상세, 이력 다음 페이지.
- 폰트 weight·이미지·영상·모션 예산: 신규 폰트/영상/장식 모션0.
- 저성능 기기와 느린 네트워크 폴백: 항목10,000/보관경로4MiB/검사8초/최근 기록250개·1MiB/응답20행. 단일 작업 lease; busy는 다음 주기로 이월. UI는 겹치지 않는5초 상태 조회, unmount cleanup.

## Accessibility Contract

- 문서·랜드마크·헤딩 읽기 순서: section heading → toggle → label → status → list.
- 키보드·포커스·Escape 동작: native inputs/buttons, 폴더 dialog는 OS 관리, 필터 후 포커스 유지.
- 레이블·오류 연결·상태 알림: switch label, 주기 label, 사용자가 요청한 작업 결과만 live announce.
- 대비·색 외 신호·터치 타깃: DESIGN.md14px 이상/44px 행동, 검토 상태 텍스트+아이콘.
- reduced-motion과 대체 경험: 새 애니메이션 없음.

## Adopt

기존 Settings full-width panel, 상태·오류 패턴을 재사용한다.

## Adapt

일반 설정 카드와 달리 검토 목록은 full-width 행과 페이지를 사용한다.

## Avoid

새 전역 메뉴/안전 배지/임의 위험 점수/자동 파일 열기/AI 자동 호출/자동 삭제.

## Prompt Contract

GOAL — 변경 검토 옵션과 실제 근거 목록.
AUDIENCE — 자원 제한 개인 컴퓨터 사용자.
TASK — 폴더 선택→옵션 ON→baseline→변경 확인→넘김.
FLOW — 완전 결과만 비교 기준; root 변경과 재시작은 새 baseline.
HEADER — 기존 Settings.
MESSAGE — 주기 확인, 백신·실시간 감시 아님.
FACTS — 실제 Rust metadata 결과.
CONTENT_INTEGRITY — 규칙 불일치는 안전 판정 아님.
SECTION_ORDER — 옵션/범위/상태/목록.
CTA — 폴더 선택 또는 지금 확인.
TRUST — 자동 공개·실행·삭제 없음.
ASSETS — 기존 내부 자산.
LAYOUT — full-width 설정+조밀한 행.
RESPONSIVE — 경로 줄바꿈과 순서 재배치.
STATES — OFF/baseline/checking/idle/incomplete/error/unavailable.
PERFORMANCE —10k항목/4MiB경로/8초/250기록1MiB/20행.
ACCESSIBILITY —14px/44px/키보드/실제 작업만 live region.
PRESERVE — DESIGN.md, 기존 권한/설정/파일 동작.
EXCLUDE — 백신/실시간/자동 LLM/내용 읽기/삭제/보안 권한 확장.
SUCCESS — 합성 fixture의 baseline/변경/넘김/재변경/상한/OFF·취소 검증과 실제 렌더.

## Success Checks

- 첫5초에 옵션 대상·주기·한계를 읽을 수 있다.
- 검토 사유와 실제 metadata만 표시하고 안전을 주장하지 않는다.
- OFF에서 파일 검사 없고, 폴더 변경/끄기로 이전 작업 반영을 차단한다.
- mobile에서 행·경로·버튼이 겹치지 않는다.
- 자원 상한/오류를 숨기지 않는다.
