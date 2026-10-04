# Layout Blueprint: Rust Native Glass Dashboard

## Block Sequence

| # | Block | Anatomy check | Swift-to-Rust variation |
|---|---|---|---|
| 1 | Sidebar | brand / 5 primary categories / optional Docker / active 1 / disk status | 32px category-color icons, thin selected tint, no dead routes |
| 2 | Briefing | h1 / one-line promise / freshness-refresh | dashboard owns h1; generic utility header omitted here |
| 3 | Storage hero | active drive / 240px ring / compact sibling rings / primary / secondary / safety note | system drive starts large; selection swaps cards without scanning |
| 4 | Quick actions | four buttons / icon / title / short result-oriented copy | cleanup, large files, verified duplicates, file search |
| 5 | Result evidence | scan summary only when report exists | values remain separate; no fake combined reclaimable total |
| 6 | Recent activity | cleanup journal / recent file catalog | compact rows below primary experience |

## Wide Desktop — 1280×820

```text
+----------------------+---------------------------------------------------------+
| ✦ BroomSweepy        | 저장공간 상태                            [새로 고침]  |
|                      | 한눈에 보고, 한 번 눌러 원인을 찾습니다.               |
| ◉ 대시보드           +---------------------------------------------------------+
| ◉ 공간 정리          | +-----------------------------------+-----------------+ |
| ◉ 파일 관리          | |                                   | 71% 사용        | |
| ◉ AI 도우미          | |    LARGE RING  [small ○ cards]    | 143 GB 남음     | |
| ◉ 설정               | |                                   | [드라이브 검사] | |
|                      | |                                   | 정리 후보 찾기 | |
|                      | +-----------------------------------+-----------------+ |
|                      | [정리 후보] [큰 파일] [중복 파일] [파일 찾기]          |
|                      +---------------------------------------------------------+
| 143 GB 여유          | 실제 결과 요약 / 최근 정리 / 최근 파일                 |
+----------------------+---------------------------------------------------------+
```

- hero 안의 드라이브 덱과 상태·행동 rail은 약 2:1이다. 드라이브 덱 내부는 큰 카드와 작은 카드 rail로 다시 나뉜다.
- main 최대 폭을 두되 창이 넓어질 때 링 주변 빈 공간이 먼저 늘어난다.
- quick action은 동일 anatomy를 쓰며 현재 결과가 있는 타일 하나만 실제 수치로 강조할 수 있다.

## Compact Desktop — 760×600

```text
+------+-----------------------------------------------------+
| rail | 저장공간 상태                         [새로 고침] |
|      | +-------------------------------------------------+ |
|  ◉   | | 상태와 남은 용량                                 | |
|  ◉   | |             ACTIVE STORAGE RING                 | |
|  ◉   | | [○ drive] [○ drive]                            | |
|  ◉   | | [이 드라이브 검사] [정리 후보 찾기]            | |
|  ◉   | +-------------------------------------------------+ |
|  ◉   | [정리 후보] [큰 파일]                            | |
|      | [중복 파일] [파일 찾기]                          | |
| disk | 최근 활동은 아래 스크롤                           | |
+------+-----------------------------------------------------+
```

- 920px 미만에서 사이드바 설명을 숨긴 72px rail을 사용한다.
- hero는 한 열이 되며 상태 → 링 → 행동의 DOM 순서를 유지한다.
- 작은 드라이브는 큰 카드 아래의 가로 스크롤 rail로 바꾸며 모든 카드를 44px 이상으로 유지한다.
- 글자나 아이콘을 축소하지 않고 세로 스크롤로 활동 영역에 접근한다.

## Narrow Transformation — under 680px

- sidebar를 overlay로 교체하고 menu button을 유지한다.
- quick actions는 1열 또는 충분한 폭의 2열로 바꾼다.
- 최근 파일은 이름·크기만 먼저 보여 주고 경로는 세부 줄로 내린다.
- primary CTA는 hero 폭을 채우지만 sticky로 만들지 않는다.

## Surface Contract

- OS window effect → transparent app canvas → one sidebar surface + one hero surface 순서다.
- hero의 gradient edge는 1px neutral reflection + ring accent 한 스톱만 사용한다.
- 빠른 실행과 활동 패널에는 `backdrop-filter`를 다시 적용하지 않는다.
- vibrancy가 없으면 같은 alpha 합성 결과와 대비를 내는 `surface-strong` 폴백을 사용한다.

## Motion Contract

- 기본 피드백은 CSS engine ladder 1, 카드 역할 교대만 FLIP 좌표 측정과 Web Animations API ladder 2를 사용한다.
- opacity·translateY entrance 220ms, tile feedback 160ms, drive push/swap 460ms, ring stroke 360ms.
- 작업 진행은 장식 루프가 아니라 실제 단계·처리량 텍스트로 갱신한다.
- reduced-motion에서는 transition과 entrance를 모두 제거한다.
