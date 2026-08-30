# Layout Blueprint: BroomSweepy Tauri Port

## Dashboard and Scan Results

| # | Block | Anatomy contract | Port variation |
|---|---|---|---|
| 1 | Sidebar | 브랜드, 4개 현재 vertical-slice 항목, 활성 1개, 하단 디스크 상태 | 920px 미만 아이콘 레일, 680px 미만 오버레이 |
| 2 | Status header | 페이지 제목, 현재 범위, 마지막 스캔 시각, 폴더 선택 | 창 제목 표시줄과 겹치지 않는 compact utility row |
| 3 | Scan stage | 저장공간 링, 상태 문구, primary 1개, cancel state | 2/3 폭 signature glass panel |
| 4 | Evidence rail | 확인한 파일 수, 큰 파일 용량, 중복 낭비, 오류 수 | 카드 대신 divide-y가 있는 1/3 rail |
| 5 | Result toolbar | 결과 유형, 정렬, 결과 수, stale 상태 | 긴 결과와 함께 sticky 가능 |
| 6 | File list | 이름·경로, 수정 시각, 크기, 검증 상태 | 680px 미만 핵심 열 외 drawer로 collapse |
| 7 | Safety bar | 선택 수, 예상 확보량, 휴지통 설명, 행동 1개 | 선택이 있을 때만 sticky |

```text
+----------------+------------------------------------------------------+
| BRAND          | PAGE / SCOPE / LAST SCAN                 [FOLDER]   |
|                +--------------------------------------+---------------+
| DASHBOARD      |                                      | FILES SEEN    |
| SPACE          |          STORAGE RING                +---------------+
| DUPLICATES     |          STATUS + SCAN               | LARGE FILES   |
| SETTINGS       |                                      +---------------+
|                |                                      | DUPLICATES    |
| DISK STATUS    +--------------------------------------+---------------+
|                | RESULT TYPE / SORT / COUNT                           |
|                +------------------------------------------------------+
|                | NAME + PATH                     MODIFIED       SIZE  |
|                | ...                                                  |
+----------------+------------------------------------------------------+
| SELECTED / RECOVERABLE TO TRASH                         [MOVE]       |
+-----------------------------------------------------------------------+
```

## Responsive Transformations

- 920px 미만: 사이드바 설명과 상태값을 숨기고 72px 아이콘 레일로 압축한다.
- 820px 미만: scan stage와 evidence rail을 세로로 재배치한다.
- 680px 미만: 내비게이션을 오버레이로 바꾸고 파일 행은 이름·크기만 유지한다.
- 모든 폭에서 primary 스캔 행동과 취소 행동은 첫 화면 또는 고정 상태 영역에서 접근 가능해야 한다.

## Motion and Effects

- 장식 효과 예산 0, signature는 정적인 저장공간 링 자체다.
- 상태 피드백은 페이지 진입, 링 값 변경, 결과 행 추가에만 사용한다.
- CSS blur는 sidebar, scan stage 각각 한 계층이며 서로 중첩하지 않는다.
