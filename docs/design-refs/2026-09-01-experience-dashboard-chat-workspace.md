# Experience Contract: 대시보드와 폴더 중심 대화

## Source Mode

- Mode: product-derived
- Evidence: 사용자 화면 피드백, `docs/design-refs/2026-09-01-benchmark-dashboard-chat-workspace.md`, 기존 Tauri 시스템 용량·파일 카탈로그·휴지통 저널, OpenAI Developers의 Codex 프로젝트 화면 설명

## Product Facts

| Claim | Source | Captured at | Freshness/status | Allowed presentation |
|---|---|---|---|---|
| Tauri 앱은 운영체제가 보고한 여러 볼륨의 총량과 여유 공간을 읽는다 | `apps/desktop/src-tauri/src/lib.rs`, `SystemOverview` | 2026-09-01 | current | 드라이브별 현재 사용량 행 |
| 휴지통 이동은 작업별 planned/moved/completed 저널을 남긴다 | `apps/desktop/src-tauri/src/trash_actions.rs` | 2026-09-01 | current | 경로를 숨긴 최근 정리 요약 |
| 파일 검색 목록은 SQLite 세대와 NTFS 변경분 갱신을 가진다 | `crates/bloomsweepy-core/src/file_catalog.rs` | 2026-09-01 | current | 첫 목록을 기준점으로 하고 다음 갱신부터 새 항목 표시 |
| 저장공간 트리맵은 폴더 크기에 비례한 사각형과 하위 폴더 이동을 제공한다 | `apps/desktop/src/components/StorageTreemapPanel.tsx` | 2026-09-01 | current | `저장공간 트리맵`과 쉬운 이름 `폴더 용량 지도` 병기 |
| 대화 공급자 어댑터는 설치된 Codex·Claude Code·Grok·Antigravity·Ollama 상태를 확인한다 | `apps/desktop/src-tauri/src/assistant_provider.rs` | 2026-09-01 | current | 사용자가 실제 사용할 수 있는 공급자·모델만 선택 |

## Page Goal

- 사용자가 달성할 결과: 컴퓨터 전체 용량과 최근 변화를 확인하고, 필요한 드라이브·폴더·파일 또는 대화로 바로 이동한다.
- 제품이 얻어야 하는 결과: 첫 화면의 설명량을 줄이고 상태 확인과 다음 행동을 분리한다.
- 관찰 가능한 성공 조건: 드라이브 여유 공간 비교, 최근 정리/새 파일 확인, 한 번의 폴더 선택으로 지도 생성, 선택 폴더에 대한 실제 대화 응답.

## Audience and Tasks

- 주요 사용자와 사용 상황: 갑자기 디스크가 찼지만 원인과 최근 변화를 모르는 개인 사용자.
- 최우선 과업: 드라이브 상태 확인, 최근 변화 확인, 문제가 큰 드라이브 또는 폴더로 진입.
- 시작 조건과 완료 조건: 앱 실행에서 시작해 확인할 위치를 골라 지도·검색·대화 중 하나로 진입하면 완료한다.
- 주요 불안·마찰·실패 가능성: 긴 검사 시간, 첫 목록의 오해, 공급자 권한, 삭제 복구 가능성.

## Header and Navigation

- 순서: 대시보드 → 용량 관리 → 파일 찾기 → 문서 찾기 → 대화 → 설정.
- 데스크톱: 216px 사이드바와 단일 메인 작업면. 하위 용량 탭은 용량 관리에서만 노출한다.
- 최소 창: 920px 이하 아이콘 레일을 유지하고 대시보드 행은 세로 묶음으로 바꾼다.

## Core Message

- 핵심 약속: 지금 어디가 찼고 무엇이 새로 생겼는지 먼저 보여준다.
- 설명: 폴더를 고르면 지도를 바로 만들고, 대화에서는 그 폴더만 현재 문맥으로 사용한다.
- 증거: 운영체제 볼륨 값, 완료된 작업 저널, 파일 목록 세대, 실제 검사 보고서.
- 다음 이해: 정리는 앱의 대상 확인과 최종 확인 뒤 운영체제 휴지통으로 이동한다.

## Content Integrity

| Content item | Classification | Evidence | Presentation rule |
|---|---|---|---|
| 드라이브 사용량 | verified | `SystemOverview` 런타임 값 | 값이 있을 때만 수치·비율 표시 |
| 최근 정리 | verified | 완료된 action journal event | 절대 경로 없이 수·용량·상태만 표시 |
| 최근 추가 파일 | verified | `first_seen_generation > 1`인 현재 카탈로그 행 | `BroomSweepy가 새로 발견`이라고 표현 |
| 첫 목록 상태 | verified | 파일 카탈로그 generation 1 | 새 파일 0개가 아니라 `기준 목록 준비됨`으로 표현 |
| 공급자 상태 | verified | 로컬 `codex --version`, `codex login status` 결과 | 자동 연결로 과장하지 않음 |

## Section Order

1. 드라이브 용량: 현재 위험과 진입점을 먼저 보여준다.
2. 최근 정리: 실제로 수행된 변화와 복구 경계를 보여준다.
3. 최근 추가 파일: 이전 목록 이후 새로 발견한 파일을 보여준다.
4. 대화 화면: 폴더 문맥 → 대화 → 입력 → 접힌 연결/권한 상세.
5. 용량 관리: 폴더 선택/새로 고침 → 트리맵 → 상세 검사 결과.

## CTA Strategy

- 대시보드 primary: 각 드라이브의 `용량 보기`; 전역 `새로 고침`은 secondary.
- 최근 파일 empty: `파일 목록 만들기` 한 개.
- 용량 관리 primary: 폴더가 없으면 `폴더 선택`; 있으면 `지도 새로 고침`만 남긴다.
- 대화 primary: `보내기`; 폴더가 없으면 같은 자리에 `폴더 선택`을 둔다. 폴더 이름 옆에는 완료된 논리 용량과 드라이브 총용량 대비 비율을 표시한다.
- 완료·실패 피드백: 해당 목록이나 입력창 바로 아래에 표시한다.

## Trust Strategy

- 검사 직전: 선택한 실제 폴더와 읽기 전용 검사임을 한 줄로 표시한다.
- 공급자 전송 직전: 파일 I/O는 앱이 실행하고, 공급자에는 사용자 질문과 제한된 결과 요약만 전달한다고 표시한다.
- 정리 기록: 휴지통으로 이동한 논리 용량임을 명시하고 즉시 확보 용량으로 표현하지 않는다.
- 근거가 없을 때: 새 파일, 정리 이력, 공급자 연결 상태를 추측해 채우지 않는다.

## Asset Provenance

- 신규 이미지·영상 없음. 기존 Pretendard, Lucide, CSS 토큰만 사용한다.
- 후보 렌더: `.termsnap/design-candidates/dashboard-chat-workspace.html`, 2026-09-01.

## Desktop Structure

- 기준 뷰포트: 1280×820, 최소 창 760×600.
- 첫 뷰포트: 드라이브 행 1~3개와 최근 정리·새 파일 제목.
- 그리드: 대시보드 하단 1.7:1, 대화는 단일 지속 pane, 용량 관리는 트리맵 중심.
- 스크롤: 목록이 길어져도 메인 콘텐츠 한 곳만 세로 스크롤한다.

## Mobile Transformations

| Desktop element | Operation | Compact result | Reason |
|---|---|---|---|
| 드라이브 5열 행 | reorder | 이름 → 사용/여유 → 막대 → 행동 | 14px를 유지하고 수치 비교 보존 |
| 최근 정리 + 새 파일 2열 | stack | 최근 정리 뒤 새 파일 | 폭보다 읽기 순서 우선 |
| 대화 scope 2열 | stack | 폴더 경로 뒤 공급자 상태 | 긴 경로 잘림 방지 |
| composer 보조 문구 | compress | 한 줄 또는 숨김 | 입력과 보내기 우선 |
| 트리맵+순위 | stack | 지도 뒤 순위 | 면적 비교 우선 |

## States

| State | Trigger | User sees | Available action | Recovery |
|---|---|---|---|---|
| loading | 볼륨/이력/최근 파일 조회 | 300ms 뒤 현재 항목의 짧은 상태 | 취소 가능한 장기 검사만 취소 | 마지막 완료 목록 유지 |
| baseline | 파일 목록 첫 작성 완료 | 비교 기준이 준비됐다는 설명 | 목록 새로 고침 | 다음 갱신부터 새 파일 표시 |
| empty | 이력 또는 새 파일 없음 | 무엇이 0인지와 기준 시점 | 관련 화면 이동 | 새로 고침 |
| error | DB·저널·Codex 오류 | 영향을 받은 영역과 원인 | 다시 시도 | 다른 기본 기능은 계속 사용 |
| stale | 휴지통 이동 뒤 파일 목록이 오래됨 | 새 파일 목록 정확도 경고 | 목록 새로 고침 | 새 세대 확정 |
| disconnected | 선택한 AI CLI 없음/로그아웃 | 대화만 사용할 수 없음 | 설치·로그인 후 다시 확인 | 기본 기능 정상 사용 |
| success | 조회·검사·대화 완료 | 실제 값과 완료 시각 | 상세 진입·후속 질문 | 필요 시 새로 고침 |

## Performance Budget

- 대시보드 IPC는 볼륨, 최근 정리 8건, 최근 파일 8건의 bounded 결과만 전달한다.
- 작업 저널은 기존 8MiB 회전 파일 두 개만 읽고 경로 목록을 UI에 반환하지 않는다.
- 최근 파일 SQL은 색인된 최초 발견 열과 제한된 결과 수를 사용한다.
- 공급자에는 전체 경로·파일 내용 없이 최대 24개 직계 항목의 이름·종류·용량을 담은 제한된 요약만 질문 입력으로 전달한다.
- 신규 이미지·영상·장식 모션·프런트 의존성은 0개다.

## Accessibility Contract

- 읽기 순서: skip link → 전역 내비 → h1 → 드라이브 → 최근 정리 → 최근 파일.
- 각 드라이브는 이름·사용량·여유량·비율을 텍스트로도 제공한다.
- 14px 최소, primary 44px, 내부 포커스, 색 이외 경고 문구를 유지한다.
- 진행 숫자는 live region에서 제외하고 시작·완료·오류만 한 번 알린다.
- reduced-motion에서는 view transition과 자동 smooth scroll을 제거한다.

## Adopt

- Data Instrument의 현재값·freshness·비교 행 구조.
- Agent Workbench의 지속 폴더 문맥과 중앙 대화 pane.

## Adapt

- Codex 프로젝트 선택을 일반 폴더 범위로 번역한다.
- 최근 파일은 OS creation time 대신 BroomSweepy 최초 발견 시각으로 설명한다.

## Avoid

- 대시보드에 선택 권한 패널을 상시 노출하지 않는다.
- 첫 색인의 모든 항목을 최근 파일로 표시하지 않는다.
- 실제 공급자 호출 없이 입력창만 보이게 하지 않는다.

## Prompt Contract

GOAL — 사용자가 현재 용량과 최근 변화를 읽고 한 번의 행동으로 상세 화면에 진입한다.
AUDIENCE — 디스크 부족 원인과 앱 사용 순서를 모르는 개인 사용자.
TASK — 드라이브 비교, 최근 정리/새 파일 확인, 폴더 지도 생성, 폴더 범위 대화.
FLOW — 대시보드 → 드라이브/최근 변화 → 용량 관리 또는 파일/대화 상세.
HEADER — 화면 이름과 한 문장 설명, 대시보드에는 전역 새로 고침만 둔다.
MESSAGE — 지금 어디가 찼고 무엇이 새로 생겼는지 먼저 보여준다.
FACTS — 운영체제 값, 저널, 카탈로그, 검사 보고서, 공급자 실행 결과만 사용한다.
CONTENT_INTEGRITY — 최초 발견을 생성일로 부르지 않고, 휴지통 논리 용량을 즉시 확보량으로 부르지 않는다.
SECTION_ORDER — 드라이브 → 최근 정리 → 최근 파일; 대화는 scope → thread → composer.
CTA — 영역당 primary 한 개.
TRUST — 범위와 전달 데이터를 행동 직전에 짧게 표시한다.
ASSETS — 기존 프로젝트 자산만 사용한다.
LAYOUT — 행 기반 데이터와 1.7:1 하단 그리드, 대화 단일 pane.
RESPONSIVE — 760px에서 행과 2열을 세로 재배치한다.
STATES — loading, baseline, empty, error, stale, disconnected, success.
PERFORMANCE — 모든 목록과 공급자 context를 bounded 처리한다.
ACCESSIBILITY — 14px, 44px, 텍스트 수치, 단일 live announcement.
PRESERVE — 다크 단층 표면, 안전 휴지통, 트리맵, 로컬 우선.
EXCLUDE — 설명 카드 벽, 가짜 대화, 자동 전체 디스크 색인, 외부 삭제.
SUCCESS — 첫 화면에서 상태와 변화가 보이고 폴더 선택 한 번으로 지도 또는 대화를 시작한다.

## Success Checks

- 첫 5초 안에 가장 찬 드라이브와 최근 변화 영역을 설명할 수 있는가?
- 모든 숫자와 이력이 실제 런타임 또는 저장 데이터에서 왔는가?
- 첫 목록과 새 파일의 의미가 혼동되지 않는가?
- 폴더 선택과 지도 시작이 하나의 행동인가?
- 대화 입력이 실제 공급자 응답 또는 명확한 연결 오류로 끝나는가?
- 최소 창과 키보드에서 주요 과업을 완료할 수 있는가?
