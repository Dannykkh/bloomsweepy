# Experience Contract: 조건부 Docker 용량 작업면

## Source Mode

- Mode: product-derived
- Evidence: 사용자의 기본 비활성·대화형 정리 요구, `DESIGN.md`, Docker 공식 CLI 문서, `docs/design-refs/2026-09-01-brief-docker-management.md`

## Product Facts

| Claim | Source | Captured at | Freshness/status | Allowed presentation |
|---|---|---|---|---|
| Docker 데몬의 디스크 사용량은 `docker system df`로 조회한다 | https://docs.docker.com/reference/cli/docker/system/df/ | 2026-09-01 | current | Docker가 보고한 사용량·정리 가능량으로 표시 |
| `docker system prune`은 기본적으로 볼륨을 지우지 않으며 `--volumes`는 별도 위험 옵션이다 | https://docs.docker.com/reference/cli/docker/system/prune/ | 2026-09-01 | current | 볼륨을 자동 정리하지 않는 근거로만 사용 |
| `docker builder prune`은 빌드 캐시를 정리하고 기간 필터를 지원한다 | https://docs.docker.com/reference/cli/docker/builder/prune/ | 2026-09-01 | current | 7일 이상 사용하지 않은 캐시 정리로 제한 |
| Docker Desktop의 VHDX와 Docker.raw는 전체 Docker 데이터 백업·복구 대상이다 | https://docs.docker.com/desktop/settings-and-maintenance/backup-and-restore/ | 2026-09-01 | current | 직접 삭제 금지 경고에만 사용 |

## Benchmark Sources

- 해당 없음 — 기존 BroomSweepy 설정과 대화 작업면에 들어가는 제품 유도형 기능이다.

## Page Goal

- 사용자가 이 화면에서 달성할 결과: Docker가 차지한 용량을 확인하고 복구 불가능성을 이해한 뒤 필요한 범주만 정리한다.
- 제품이 얻어야 하는 결과: 일반 사용자 화면을 복잡하게 만들지 않으면서 개발자에게 공식 Docker 경계를 제공한다.
- 관찰 가능한 성공 조건: 설정 활성화, 사용량 조회, 대화 요약, 미리보기, 취소 또는 확인, 실행 후 재조회가 동작한다.

## Audience and Tasks

- 주요 사용자와 사용 상황: Docker Desktop 또는 Docker Engine을 사용하며 저장공간 부족 원인을 확인하는 개발자.
- 최우선 과업: Docker 용량 확인, 빌드 캐시 정리 검토, 필요 시 매달린 이미지·오래된 중지 컨테이너 추가 선택.
- 시작 조건과 완료 조건: 설정에서 기능을 켜는 것으로 시작해 사용량을 확인하거나 정리 후 갱신된 값을 보는 것으로 완료한다.
- 주요 불안·마찰·실패 가능성: 볼륨 데이터 손실, 휴지통 복원 오해, Docker 데몬 중지, 오래된 미리보기, 부분 정리 후 취소.

## Header and Navigation

- 브랜드·현재 위치·전역 이동·주 행동의 순서: `설정`에서 Docker를 켜면 사이드바의 `용량 관리` 다음에 `Docker 용량`이 나타나며, 상태 확인과 정리는 그 전용 작업면에서 수행한다.
- 데스크톱 내비게이션: Docker를 끄면 기존 여섯 메뉴만 유지하고, 켰을 때만 조건부 메뉴를 정확히 한 개 추가한다. 대시보드에는 Docker 항목을 추가하지 않는다.
- 모바일 대체 구조: 조건부 Docker 메뉴는 기존 오버레이 내비게이션에 같은 순서로 나타나고, 전용 화면의 상태·사용량·행동은 한 열로 재배치한다.

## Core Message

- 핵심 약속: Docker가 보고한 용량을 확인하고 BroomSweepy가 허용한 정리만 실행한다.
- 설명: AI CLI는 설명과 제안을 맡고 앱이 미리보기·최종 확인·Docker 명령 실행을 맡는다.
- 증거: Docker CLI 버전, 데몬 상태, `docker system df`의 범주별 값, 실행 후 다시 조회한 값.
- 사용자가 다음에 이해해야 할 것: Docker 정리는 운영체제 휴지통을 거치지 않으며 볼륨은 자동 정리하지 않는다.

## Content Integrity

| Content item | Classification | Evidence | Presentation rule |
|---|---|---|---|
| 범주별 사용량·정리 가능량 | verified | 현재 Docker CLI 출력 | 조회 시각과 함께 표시하고 실제 확보량으로 단정하지 않음 |
| 정리 예상량 | verified estimate | 정리 직전 `docker system df` | `최대 예상`으로 표시하고 실행 결과와 구분 |
| AI 설명 | generated advice | 선택한 로컬 AI CLI 응답 | 실행 결과로 표현하지 않고 앱 검토 행동과 분리 |
| 볼륨 | verified usage, excluded action | Docker CLI 출력과 공식 prune 경고 | 사용량만 표시하고 자동 정리 버튼은 제공하지 않음 |

## Section Order

1. 설정 토글: 사용자가 명시적으로 기능을 켜고 전용 메뉴를 연다.
2. Docker 상태: 전용 화면에서 설치·데몬·버전·조회 시각을 확인한다.
3. 범주별 사용량: 이미지·컨테이너·볼륨·빌드 캐시를 행으로 비교한다.
4. Docker 대화: 전용 화면에서 폴더 선택 없이 Docker 대상 새 대화를 시작한다.
5. 최종 확인: 범주 선택, 최대 예상량, 고정 명령 설명, 복구 불가 확인.
6. 실행 결과: 범주별 성공·실패·취소와 갱신된 사용량을 보여준다.

## CTA Strategy

- Primary: 설정에서는 `Docker 용량 관리 사용`, 전용 화면에서는 `Docker 정리 검토`, 확인창에서는 `선택 항목 정리`.
- Secondary: `Docker 대화 시작`, `사용량 다시 확인`, `취소`.
- 반복 규칙: 정리 행동은 기능이 켜지고 데몬 조회가 성공했을 때만 표시한다.
- 완료·실패 피드백: 실행 결과와 재조회 값을 확인창 안에서 보여주고 부분 완료 가능성을 숨기지 않는다.

## Trust Strategy

- 사용자가 불안을 느끼는 지점: 정리 버튼을 누르기 직전과 작업 중 취소할 때.
- 그 직전에 제시할 근거: 정리 범주, 최대 예상량, 7일 기준, 볼륨 제외, 휴지통 미사용을 평문으로 보여준다.
- 출처·날짜·검증 가능성: Docker CLI 버전과 마지막 조회 시각을 함께 표시한다.
- 근거가 없을 때 생략할 요소: 데몬에 연결하지 못하면 용량과 정리 행동을 모두 숨긴다.

## Asset Provenance

- 해당 없음 — 기존 Pretendard, JetBrains Mono, Lucide 아이콘과 CSS 토큰만 사용한다.

## Desktop Structure

- 기준 뷰포트: 1280×820, 최소 창 760×600.
- 첫 뷰포트: 설정에는 전폭 활성화 행과 `Docker 용량 열기`만 두고, 전용 화면은 상태 메타 → 핵심 총량 → 얇은 사용량 행 → 행동 순서로 시작한다.
- 그리드·pane·콘텐츠 위계: 전용 화면에서 상태 → 사용량 비교 → 대화 또는 정리 검토 순서이며 카드 중첩은 만들지 않는다.
- 스크롤 흐름과 밀도 변화: 기존 메인 스크롤 하나를 유지하고 확인창 내부만 필요할 때 스크롤한다.

## Mobile Transformations

| Desktop element | Operation | Mobile result | Reason |
|---|---|---|---|
| 토글 설명 + 스위치 | retain | 설명 아래 44px 스위치 | 명시적 동의 유지 |
| 4열 사용량 행 | reorder | 범주 → 사용량/정리 가능량 → 상태 | 14px를 줄이지 않고 비교 보존 |
| 전용 화면 행동 2개 | reorder | `Docker 대화 시작` 뒤 `Docker 정리 검토`를 세로 배치 | 위험 행동을 마지막에 유지 |
| 확인창 범주 목록 | retain | 체크 행을 세로 유지 | 위험 정보를 숨기지 않음 |

## States

| State | Trigger | User sees | Available action | Recovery |
|---|---|---|---|---|
| disabled | 기본값 또는 사용자가 끔 | Docker를 검사하지 않는다는 한 줄 | 기능 켜기 | 없음 |
| loading | 상태·용량 조회 | 300ms 뒤 `Docker 사용량 확인 중` | 없음 | 실패 시 다시 확인 |
| empty | 데몬은 연결됐지만 Docker 데이터가 없음 | 모든 범주 0과 완료 시각 | 후속 질문 | 새 이미지·컨테이너 생성 뒤 다시 확인 |
| unavailable | CLI 없음 또는 데몬 중지 | 원인과 영향 | 설치·실행 후 다시 확인 | 일반 기능 계속 사용 |
| ready | 사용량 조회 성공 | 범주별 값과 조회 시각 | 정리 검토·다시 확인 | 새 조회로 갱신 |
| preview | 정리 검토 시작 | 범주·예상량·복구 불가 | 확인 또는 취소 | 만료 시 새 미리보기 |
| running | prune 실행 | 현재 범주와 단계 | 중단 요청 | 실행한 범주는 되돌릴 수 없음 |
| partial | 일부 성공 뒤 실패·취소 | 범주별 결과와 재조회 값 | 다시 검토 | 실패 범주만 새 미리보기 |
| error | 조회·미리보기·실행 준비 실패 | 원인과 영향 범위 | 다시 확인 또는 닫기 | 일반 파일 기능은 계속 사용 |
| success | 모든 선택 범주 완료 | 완료 요약과 갱신된 값 | 닫기·후속 질문 | 필요 시 다시 확인 |

## Performance Budget

- 첫 화면 필수 자산: 신규 자산 없음.
- 지연 가능한 자산: Docker CLI 탐색과 사용량 조회는 기능을 켠 뒤 blocking worker에서만 실행한다.
- 폰트 weight·이미지·영상·모션 예산: 신규 폰트·이미지·장식 모션 0개.
- 저성능 기기와 느린 네트워크 폴백: 출력 1MiB 이하, 조회 절대 제한 시간, 정리 직렬 실행, 취소 토큰을 적용한다.

## Accessibility Contract

- 문서·랜드마크·헤딩 읽기 순서: 설정 제목 → 개발 도구 제목 → 토글 → 상태 → 사용량 행 → 행동.
- 키보드·포커스·Escape 동작: native checkbox와 button을 사용하고 dialog는 초점을 가두며 Escape는 취소 또는 닫기로 동작한다.
- 레이블·오류 연결·상태 알림: 시작·완료·오류만 live region으로 알리고 빠른 숫자 변화는 반복 낭독하지 않는다.
- 대비·색 외 신호·터치 타깃: 상태는 텍스트와 아이콘을 함께 쓰고 모든 행동은 최소 44px이다.
- reduced-motion과 대체 경험: 장식 모션 없이 진행 상태를 단계 텍스트로 제공한다.

## Adopt

- Docker 공식 CLI의 범주와 기본 볼륨 제외 원칙을 채택한다.
- 기존 BroomSweepy의 앱 최종 확인과 단일 작업 직렬화 원칙을 채택한다.

## Adapt

- Docker의 터미널 확인을 앱의 범주 선택·복구 불가 확인창으로 바꾼다.
- AI CLI의 역할을 요약·판단·제안으로 제한하고 실행 권한은 앱에 남긴다.

## Avoid

- Docker 데이터 파일, WSL 가상 디스크, Docker.raw를 직접 삭제하거나 압축하지 않는다.
- `docker system prune -a --volumes`, 임의 셸 문자열, 공급자 생성 명령을 실행하지 않는다.
- 기능이 꺼진 상태에서 설치 감지나 백그라운드 조회를 하지 않는다.

## Prompt Contract

GOAL — Docker 사용자만 조건부 전용 화면에서 용량을 확인하고 복구 불가능성을 이해한 뒤 허용 범주를 정리한다.
AUDIENCE — Docker를 사용하는 개인 개발자와 사용하지 않는 일반 사용자.
TASK — 기능 켜기, 사용량 조회, AI 설명, 정리 검토, 최종 확인, 결과 재조회.
FLOW — settings opt-in → conditional Docker navigation → status/df → optional Docker-scoped chat → typed preview → native confirmation → allowlisted prune → refresh.
HEADER — Docker 사용 시에만 전용 메뉴를 표시하고 대시보드는 일반 저장공간에 집중한다.
MESSAGE — AI는 제안하고 BroomSweepy가 확인 뒤 Docker CLI를 실행한다.
FACTS — Docker 공식 문서와 현재 CLI 출력만 사용한다.
CONTENT_INTEGRITY — 정리 가능량은 최대 예상이며 실제 확보량과 구분한다.
SECTION_ORDER — toggle → conditional menu → status → usage rows → Docker chat or review → confirmation → result.
CTA — 화면마다 primary 한 개, 정리는 최종 확인 뒤 한 번만 실행한다.
TRUST — 볼륨 제외, 7일 기준, 휴지통 미사용, 부분 완료 가능성을 행동 직전에 표시한다.
ASSETS — 기존 프로젝트 자산만 사용한다.
LAYOUT — 얇은 행과 한 겹 dialog, 카드 벽 없음.
RESPONSIVE — 760px에서 행을 세로 재배치하고 글자를 줄이지 않는다.
STATES — disabled, loading, unavailable, ready, preview, running, partial, success.
PERFORMANCE — opt-in 전 CLI 호출 0, bounded output, deadline, serial execution.
ACCESSIBILITY — 14px, 44px, 명시적 label, focus trap, 단일 상태 알림.
PRESERVE — 다크 단층 표면, 앱 실행 경계, 로컬 우선, 일반 사용자 기본 화면.
EXCLUDE — volumes prune, raw disk deletion, arbitrary shell, provider direct execution, automatic cleanup.
SUCCESS — Docker를 쓰지 않는 사용자는 기능을 보지 않고, 쓰는 사용자는 전용 화면과 폴더 없는 Docker 대화에서 실행 결과까지 한 흐름으로 완료한다.

## Success Checks

- 기능이 꺼진 상태에서 Docker CLI 프로세스가 실행되지 않는가?
- AI 응답과 실제 실행 상태가 명확히 분리되는가?
- 볼륨·가상 디스크 직접 삭제가 모든 경로에서 불가능한가?
- 최종 확인 없이 prune 명령이 실행될 수 없는가?
- 실패·취소 뒤 부분 완료 여부와 새 사용량을 확인할 수 있는가?
- 760×600과 키보드만으로 설정·검토·취소를 완료할 수 있는가?
