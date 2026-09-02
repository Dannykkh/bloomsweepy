# Experience Contract: 대상별 대화 세션

## Source Mode

- Mode: product-derived
- Evidence: 사용자의 세션 이어하기·삭제·새 폴더 대화·Docker 선택 요구, `DESIGN.md`, 기존 대화 화면과 제한 요약 계약

## Product Facts

| Claim | Source | Captured at | Freshness/status | Allowed presentation |
|---|---|---|---|---|
| 대화 공급자에는 제한된 폴더 요약과 최근 대화만 전달한다 | `apps/desktop/src-tauri/src/assistant_provider.rs` | 2026-09-01 | current | 파일 내용과 전체 경로를 보내지 않는다고 표시 |
| 폴더 용량은 앱의 완료된 디렉터리 검사에서 온다 | `DirectoryScanReport` | 2026-09-01 | current | 검사 시각과 논리 용량으로 표시 |
| 세션 데이터는 앱 데이터 폴더의 SQLite에 저장한다 | 이번 기능의 `assistant_sessions` 경계 | 2026-09-01 | implementation target | 로컬 저장이라고만 표시하고 보안 삭제로 과장하지 않음 |

## Benchmark Sources

- 해당 없음 — 기존에 승인된 BroomSweepy 대화 작업면의 국소 확장이다.

## Page Goal

- 사용자가 이 화면에서 달성할 결과: 폴더 또는 Docker 대상별 대화를 다시 열고 AI를 바꿔도 후속 질문을 이어간다.
- 제품이 얻어야 하는 결과: 공급자 세션에 의존하지 않고 앱이 대화 수명주기와 삭제를 소유한다.
- 관찰 가능한 성공 조건: 재시작 후 최근 세션 복원, 새 폴더 세션 생성, 폴더 선택 없는 Docker 세션 생성, 현재 세션 삭제와 다음 세션 전환이 동작한다.

## Audience and Tasks

- 주요 사용자와 사용 상황: 여러 폴더의 용량 원인을 며칠에 걸쳐 확인하는 개인 사용자.
- 최우선 과업: 최근 대화 이어하기, 새 폴더 대화 또는 Docker 대화 시작, 필요 없는 대화 삭제.
- 시작 조건과 완료 조건: 대화 화면 진입에서 시작해 기존 세션을 열거나 폴더/Docker 대상을 고르고 첫 질문을 보낼 때 완료한다.
- 주요 불안·마찰·실패 가능성: 새 대화와 폴더 변경 혼동, 삭제가 실제 파일에 영향을 주는지에 대한 불안, 오래된 검사 결과의 오해.

## Header and Navigation

- 브랜드·현재 위치·전역 이동·주 행동의 순서: 전역 `대화` 위치 다음에 세션 선택과 `새 대화`를 둔다.
- 데스크톱 내비게이션: 기존 전역 사이드바를 유지하고 세션은 대화 화면 내부 도구막대에서만 관리한다.
- 모바일 대체 구조: 세션 선택과 새 대화를 한 행에서 세로 재배치하고 transcript를 우선한다.

## Core Message

- 핵심 약속: 폴더와 Docker 대화를 대상별로 저장하고 나중에 그대로 이어서 질문한다.
- 설명: 폴더 대화는 새 폴더 선택과 용량 계산부터, Docker 대화는 현재 Docker 요약부터 시작한다.
- 증거: 세션의 폴더 이름, 메시지 수, 마지막 사용 시각, 저장된 검사 시각.
- 사용자가 다음에 이해해야 할 것: 세션 삭제는 대화 데이터만 삭제하고 실제 파일은 삭제하지 않는다.

## Content Integrity

| Content item | Classification | Evidence | Presentation rule |
|---|---|---|---|
| 메시지 수·마지막 사용 시각 | verified | SQLite 행과 메시지 집계 | 실제 값만 표시 |
| 폴더 점유율 | verified | 저장된 논리 용량 / 현재 볼륨 총용량 | 원형 그래프와 정확한 비율을 함께 표시 |
| 오래된 폴더 요약 | verified but stale | 세션의 검사 완료 시각 | 현재값으로 부르지 않고 검사 시각 표시 |

## Section Order

1. 세션 도구막대: 새 폴더 대화·조건부 Docker 대화와 기존 대화 선택.
2. 대화 대상: 폴더의 용량·드라이브 비율 또는 Docker 범주 합계·정리 가능 상한.
3. 대화 기록: 저장된 메시지와 공급자 표시.
4. 입력창: 선택한 AI로 후속 질문.
5. 연결과 권한: 접힌 고급 정보.

## CTA Strategy

- Primary: `새 폴더 대화` — 폴더 선택과 검사 후 빈 세션 생성. Docker 활성 시 `Docker 대화`를 별도 secondary로 제공한다.
- Secondary: `대화 삭제` — 현재 세션이 있을 때만 활성화.
- 반복 규칙: 새 대화는 세션 도구막대와 빈 상태에서만 제공한다.
- 완료·실패 피드백: 저장 오류는 대화 입력 바로 위, 삭제 확인은 실제 파일에 영향이 없음을 포함한다.

## Trust Strategy

- 사용자가 불안을 느끼는 지점: 세션 삭제가 폴더나 파일까지 삭제할 수 있다는 오해.
- 그 직전에 제시할 근거: 확인창에 `대화와 저장된 폴더 요약만 삭제`라고 명시한다.
- 출처·날짜·검증 가능성: 각 세션의 마지막 사용 시각과 폴더 검사 시각을 표시한다.
- 근거가 없을 때 생략할 요소: 드라이브 총용량을 찾지 못하면 점유율 그래프를 숨긴다.

## Asset Provenance

- 해당 없음 — 기존 Pretendard, JetBrains Mono, Lucide 아이콘과 CSS 토큰만 사용한다.

## Desktop Structure

- 기준 뷰포트: 1280×820, 최소 창 760×600.
- 첫 뷰포트: 한 줄 세션 도구막대, 한 줄 대화 대상, transcript와 composer.
- 그리드·pane·콘텐츠 위계: 세션 선택은 얇은 도구막대, 대화는 기존 단일 pane을 유지한다.
- 스크롤 흐름과 밀도 변화: transcript만 내부 스크롤하고 세션 목록은 네이티브 select로 제한한다.

## Mobile Transformations

| Desktop element | Operation | Mobile result | Reason |
|---|---|---|---|
| 새 폴더 대화 + Docker 대화 + 세션 선택 + 삭제 | reorder | 두 대상 행동 뒤 전폭 세션 선택, 삭제는 우측 유지 | 대상 선택을 숨기지 않음 |
| 폴더 용량 + 점유율 링 | compress | 용량과 작은 링·비율을 한 줄로 유지 | 숫자 의미를 잃지 않음 |
| 공급자 선택 | reorder | 폴더 범위 아래 전폭 배치 | 긴 모델 이름 겹침 방지 |

## States

| State | Trigger | User sees | Available action | Recovery |
|---|---|---|---|---|
| loading | 세션 목록·상세 조회 | `대화 기록 불러오는 중` | 기다림 | 실패 시 다시 시도 |
| empty | 저장된 세션 없음 | 새 폴더 대화와 활성화된 경우 Docker 대화 안내 | 대상별 새 대화 | 선택 취소 시 빈 상태 유지 |
| error | DB 생성·읽기·쓰기 실패 | 영향과 실패 원인 | `다시 시도` 또는 현재 화면 계속 사용 | 메시지는 메모리에 유지 |
| deleting | 현재 세션 삭제 중 | 삭제 버튼 비활성화 | 없음 | 실패 시 현재 세션 유지 |
| success | 세션 로드·생성·삭제 완료 | 선택한 폴더와 저장된 transcript | 후속 질문·새 대화·삭제 | 다음 세션 자동 선택 |

## Performance Budget

- 첫 화면 필수 자산: 기존 앱 자산만 사용한다.
- 지연 가능한 자산: 세션 목록과 상세 메시지는 대화 화면 진입 후 비동기로 읽는다.
- 폰트 weight·이미지·영상·모션 예산: 신규 자산과 장식 모션 0개.
- 저성능 기기와 느린 네트워크 폴백: 로컬 SQLite만 사용하고 목록·메시지 수를 상한 처리한다.

## Accessibility Contract

- 문서·랜드마크·헤딩 읽기 순서: 화면 제목 → 세션 도구막대 → 폴더 범위 → transcript → composer.
- 키보드·포커스·Escape 동작: 모든 세션 행동은 native button/select로 접근하고 확인창은 OS 키보드 규칙을 따른다.
- 레이블·오류 연결·상태 알림: 아이콘 삭제 버튼에 현재 폴더 이름을 포함한 이름을 주고 오류는 `role=alert`로 알린다.
- 대비·색 외 신호·터치 타깃: 원형 그래프 옆에 정확한 텍스트를 두고 주요 버튼은 44px 이상이다.
- reduced-motion과 대체 경험: 신규 모션은 없고 비율 변화도 즉시 반영한다.

## Adopt

- Agent Workbench의 지속 문맥과 최근 작업 선택 구조를 채택한다.

## Adapt

- 프로젝트 선택을 폴더별 대화 세션으로 바꾸고, 새 세션 생성 전 실제 폴더 검사를 완료한다.

## Avoid

- 공급자 고유 세션을 앱의 대화 원본으로 사용하지 않는다.
- 세션 삭제를 파일 삭제 또는 보안 삭제로 표현하지 않는다.
- 화면을 영구 세션 사이드바와 카드 목록으로 과밀하게 만들지 않는다.

## Prompt Contract

GOAL — 폴더와 Docker 대상별 대화를 로컬에 저장하고 재시작·공급자 변경 뒤에도 이어간다.
AUDIENCE — 여러 폴더와 선택형 Docker 용량 문제를 반복해서 확인하는 개인 사용자.
TASK — 최근 세션 열기, 새 폴더 또는 Docker 세션 만들기, 현재 세션 삭제, 후속 질문.
FLOW — 대화 진입 → 최근 세션 복원 또는 대상별 새 대화 → 앱 요약 → 질문 → 저장.
HEADER — 기존 대화 제목을 유지하고 화면 내부 첫 줄에 세션 도구막대를 둔다.
MESSAGE — 대화 기록은 로컬에 저장되며 폴더와 Docker 대상이 명시적으로 구분된다.
FACTS — SQLite 세션, 완료된 폴더 검사 또는 현재 Docker 요약, 실제 공급자 응답만 사용한다.
CONTENT_INTEGRITY — 오래된 검사 시각과 드라이브 비율을 정확히 표시한다.
SECTION_ORDER — session toolbar → scope → transcript → composer → permission details.
CTA — `새 폴더 대화`를 primary로, 활성화된 `Docker 대화`와 현재 세션 삭제를 secondary로 둔다.
TRUST — 삭제 확인 직전에 실제 파일은 그대로라는 문구를 제공한다.
ASSETS — 기존 폰트·아이콘·토큰만 사용한다.
LAYOUT — 기존 단일 대화 pane과 얇은 행 구조를 보존한다.
RESPONSIVE — 760px에서 글자를 줄이지 않고 세션 도구를 재배치한다.
STATES — loading, empty, error, deleting, success.
PERFORMANCE — 세션·메시지·문자 길이를 상한 처리하고 DB I/O는 blocking worker에서 실행한다.
ACCESSIBILITY — 14px, 44px, native select, 명시적 label과 alert.
PRESERVE — 기존 다크 단층 표면, provider picker, bounded 요약, 실제 파일·Docker 작업과의 분리.
EXCLUDE — 새 전역 메뉴, 상시 세션 사이드바, 자동 파일 삭제, 공급자 세션 의존.
SUCCESS — 재시작 후 최근 대화가 열리고 새 폴더/Docker 대화·삭제·AI 변경 후 후속 질문이 작동한다.

## Success Checks

- 첫 5초 안에 현재 세션과 새 대화 행동을 설명할 수 있는가?
- 새 대화가 반드시 폴더 선택과 완료된 검사에서 시작하는가?
- 세션을 삭제해도 실제 파일과 공용 검색 인덱스가 유지되는가?
- 공급자를 바꿔도 앱이 저장한 최근 대화가 전달되는가?
- 760×600에서 세션 선택·삭제·질문을 키보드로 완료할 수 있는가?
