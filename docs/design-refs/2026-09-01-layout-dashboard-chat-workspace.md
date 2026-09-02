# Layout Blueprint: 대시보드와 폴더 중심 대화

## Navigation

| Level | Item | Purpose |
|---|---|---|
| Global | 대시보드 | 드라이브 용량과 최근 변화 |
| Global | 용량 관리 | 폴더 용량 지도·큰 파일·중복·정리 후보 |
| Global | 파일 찾기 | 이름·위치 검색과 파일 목록 갱신 |
| Global | 문서 찾기 | 문서 내용 검색 |
| Global | 대화 | 선택 폴더에 대한 로컬 AI CLI 대화 |
| Global | 설정 | 검사 기준과 안전 |

## Block Sequence

| Screen | Block sequence | Anatomy check | Variation |
|---|---|---|---|
| Dashboard | Header → Drive table → Cleanup activity + Recent files | freshness, rows, empty/error, one action per row | 1.7:1 lower split |
| Storage | Folder scope → storage subnav → Treemap → result links | one folder action, last checked, stale/refresh | no onboarding card after folder selection |
| Chat | Folder scope + size/share → transcript → sticky composer → permission details | real provider state, error, bounded context note | one continuous pane, no card stack |

## Wide Desktop — 1280×820

```text
+--------------------+-------------------------------------------------------+
| Brand              | 대시보드                              [새로 고침]   |
| 대시보드           +-------------------------------------------------------+
| 용량 관리          | 드라이브 용량                                         |
| 파일 찾기          | C: used/free/bar [용량 보기]                          |
| 문서 찾기          | D: used/free/bar [용량 보기]                          |
| 대화               +------------------------------------+------------------+
| 설정               | 최근 정리                         | 최근 추가 파일   |
|                    | 시간 / 결과 / 논리 용량           | 이름 / 발견 / 크기|
| local status       |                                    |                  |
+--------------------+------------------------------------+------------------+
```

```text
+--------------------+-------------------------------------------------------+
| nav                | 대화                                                  |
|                    | 현재 폴더: C:\...  12 GB · C: 2.1%   Claude 연결됨  |
|                    +-------------------------------------------------------+
|                    |                                                       |
|                    | transcript                                            |
|                    |                                                       |
|                    +-------------------------------------------------------+
|                    | [선택한 폴더에 대해 물어보세요                ][보내기]|
+--------------------+-------------------------------------------------------+
```

## Compact Desktop — 760×600

```text
+------+-----------------------------------------------+
| rail | 대시보드                         [새로 고침] |
|      | C: / 사용·여유 / bar / [용량 보기]           |
|      | D: / 사용·여유 / bar / [용량 보기]           |
|      | 최근 정리                                    |
|      | 최근 추가 파일                               |
+------+-----------------------------------------------+
```

- 글자를 줄이지 않고 드라이브 열을 이름·수치·막대·행동 순으로 재배치한다.
- 대화 composer는 전폭을 유지하고 공급자·권한 상세는 `details`로 내려간다.
- 메인 콘텐츠 한 곳만 세로 스크롤한다.

## State Placement

| State | Placement | Next action |
|---|---|---|
| 파일 목록 없음 | 최근 추가 파일 panel | 파일 목록 만들기 |
| 첫 목록만 있음 | panel meta + empty row | 목록 새로 고침 |
| 새 파일 있음 | compact rows | 파일 위치 보기 |
| 정리 이력 없음 | 최근 정리 panel | 용량 관리로 이동 |
| 선택한 AI CLI 없음 | composer 위 inline status | 설치/로그인 뒤 다시 확인 |
| 폴더 없음 | chat scope + composer | 폴더 선택 |
| 지도 검사 중 | treemap panel + global dock | 취소 |
