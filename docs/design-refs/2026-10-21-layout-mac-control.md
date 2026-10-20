# Layout Blueprint — Mac setting controls

2026-10-21 · product-derived · 단계: grayscale 구조 검토→3시안 선택. 새 제품 라우트/OS 기능 추가 없음.

## Desktop structure

1280×820 기준. 전역 위치는214px 조용한 사이드바, 메인은 Finder 정체성·짧은 범위·기능 조작·기록이다. 중복 페이지 제목과 큰 설명 배너를 없앤다.

| Block | Anatomy | Priority |
|---|---|---|
| 프리뷰 경계 | 실제 OS 변경 없음 | 상시 짧게 |
| Header | 맥 설정1개·Finder 현재 위치·변경 기록 진입 | 기능과 조작 앞 |
| 파일 표시 | 아이콘·제목·상태·끄기/켜기·적용 ×3 | 주 과업 |
| 경로와 정보 | 같은 anatomy ×2 | 주 과업 |
| 기록 | 항목·원래값→적용값·복원 | 직접 진입, 보조 이동 뒤에 숨기지 않음 |
| 설명 | details 하나 | 상태/조작은 여기 넣지 않음 |

## Narrow transformation

390×844 실제 viewport와 narrow=1 CSS container를 구분한다. 사이드바는 제거, 제목/기능 분류/현재 상태는 유지, 조작은 다음 줄에 재배치해44px 표적을 유지한다. 기록은 동일 위치에서 재발견한다. 페이지 폭을 줄여 글자를 작게 만들지 않는다.

## Grayscale gate

원본은 `apps/desktop/mac-control-design.html`과 `src/mac-control-design.*`. 먼저 무채색 버전으로 구현하며 root가 desktop/narrow를 직접 캡처한 뒤 구조를 판정한다. 시각 방향은 이 판정 이후에만 추가한다. 최종 관찰/캡처 경로는 같은 파일의 후속 기록에 남긴다.

2026-10-21 00:42 KST root가 IAB 실제1280×820/390×844 viewport로 캡처·저장·다시 열어 관찰했다. [desktop](../qa/mac-design-review-2026-10-21/03-wireframe-desktop.png), [narrow](../qa/mac-design-review-2026-10-21/04-wireframe-narrow.png).

| Criterion | Observed evidence | Decision |
|---|---|---|
| Hierarchy | 제목1개, 두 기능 묶음,5개 상태/선택/적용이 desktop 첫 뷰포트에 보임 | 큰 안내 배너 없이 유지 |
| Structure | 파일표시3/경로정보2, 기록은 header에서 직접 접근 | 그룹 유지, 기록을 메뉴 이동 뒤에 숨기지 않음 |
| Flow | 초기 두 선택 모두 false aria-pressed, 적용 비활성, local 적용/복원 경로 존재 | 원래 null과 초안을 보존 |
| Consistency | 각행 아이콘→이름/상태→선택/적용 순서 동일 | 모든 시안 동일 과업/데이터 |
| Clarity | narrow에서도 상태와44px 선택/적용이 다음 줄에 보임, 긴 설명만 접힘 | 텍스트축소 대신 재배치 |

구조 gate는 통과했고 시각3방향 탐색을 후속 지시했다. 이것은 사용자의 시각 선택이나 제품 적용 승인이 아니다. narrow 기본 root 높이가820이라 아래 여백이 남는 점은 시각 단계에서844 높이에 맞추도록 수정 요청했다.

시각 탐색 후3안 모두390×844 높이를 확인했다. 무채색 비평에서 초기 양쪽 값의 ‘미선택’ 여부가 모호하다는 지적을 받아, styled 조작부에 `변경할 값 / 미선택`을 붙였다. C의 master/detail 구조는 A와 달라 선택될 경우 별도 저충실도 blueprint·좁은 화면 흐름 재검토가 필요하다. 이번 탐색으로 C의 제품 반영 구조까지 승인한 것이 아니다.

## Design boundaries

초기 모든 값은null, 끄기/켜기 모두 미선택. 선택은 아직 적용 안 됨, 적용은 local mock만, 기록/복원은 실제 화면 내부. Finder5개 외OS 기능·다른 메뉴 실행 카드·가짜상태·실제 OS/IPC/권한 호출은 없다. 구조 선택 전에 production MacManagementView/AppShell/DESIGN 정본을 바꾸지 않는다.

## Selected A production delta

사용자 “추천대로가자”로 A를 확정했다. 위 경계는 비교시안의 기록이며 실제 제품은 [갱신한 Experience Contract](2026-10-21-experience-mac-control.md)를 따른다. 기존 AppShell의 단일 h1과 native glass를 재사용하고 Mac 전용 헤더만 일반 스크롤 흐름에 둔다. 가짜 sidebar와 preview toolbar는 제외한다. 두 기능 묶음·직접 조작·기록 진입 구조는 그대로이며, null/관리/실패/복원은 기존 native snapshot을 표시한다. narrow는 기존 overlay내비게이션+조작부 줄바꿈이다. 제품 구현은 동일 A 방향의 범위 조정이므로 새로운3시안 탐색은 하지 않는다.
