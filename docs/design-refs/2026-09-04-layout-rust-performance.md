# Layout Blueprint: Rust Performance Instrument

> Historical asymmetric composition. Superseded on 2026-09-05 by [equal CPU/RAM rings](2026-09-05-brief-dual-performance-rings.md): shared heading, 1:1 panes, two same-sized meters, and `앱 메모리 정리 · BroomSweepy 전용`.

## Block Sequence

| # | Block | Anatomy check | Swift-to-Rust adaptation |
|---|---|---|---|
| 1 | Utility header | eyebrow / h1 / one-line description / freshness | 기존 AppShell header 사용 |
| 2 | System instrument | large CPU ring / memory rail / exact values / swap / one-click memory action | fake health score와 pressure allocation 제거 |
| 3 | Process controls | title / sort CPU·RAM / compact trust copy | 실제 측정에만 연결 |
| 4 | Process rows | name / PID / CPU / resident memory / action | 최대 40개, 종료 불가 사유 제공 |
| 5 | Confirmation | target / warning / cancel / normal termination | cancel-first focus, identity revalidation |

## Wide Desktop — 1280×820

```text
+----------------------+---------------------------------------------------------+
| BroomSweepy          | 성능                                      [새로 고침] |
| 대시보드             | 실제 CPU와 메모리 사용량을 확인합니다.                  |
| 공간 정리            +---------------------------------------------------------+
| 파일 관리            | +------------------------------+----------------------+ |
| 성능                 | |          CPU RING            | MEMORY               | |
| AI 도우미            | |           34%                | 12.1 / 24 GB         | |
| 설정                 | | 8 core · just now            | meter · available    | |
|                      | |                              | [메모리 정리       ] | |
|                      | +------------------------------+----------------------+ |
|                      | 상위 프로세스        [CPU] [메모리]                   |
|                      | App name     PID      18%      1.2 GB   [종료 요청]  |
|                      | App name     PID       8%      860 MB   [종료 요청]  |
+----------------------+---------------------------------------------------------+
```

## Compact Desktop — 760×600

```text
+------+-----------------------------------------------------+
| rail | 성능                                  [새로 고침] |
|      | +-------------------------------------------------+ |
|      | | CPU RING · value                               | |
|      | | MEMORY meter · used / total                    | |
|      | | [메모리 정리 — BroomSweepy 자체만]            | |
|      | +-------------------------------------------------+ |
|      | 상위 프로세스 [CPU] [RAM]                         |
|      | name + pid              CPU / RAM   [종료 요청]  |
|      | name + pid              CPU / RAM   [종료 요청]  |
+------+-----------------------------------------------------+
```

## Surface and Motion Contract

- native glass는 창 바탕과 system instrument에만 한 단계 사용한다. 프로세스 행은 얇은 선과 hover 명도로 구분한다.
- CPU ring만 기존 blue-violet signature를 재사용하고 메모리 meter는 neutral + primary 한 색으로 제한한다.
- 메모리 정리 버튼은 메모리 pane의 전체 폭 solid primary 행동이다. 실행 결과는 같은 pane 안의 작은 status surface로 교체·갱신하며 새 modal을 만들지 않는다.
- 값 갱신은 애니메이션하지 않는다. 첫 진입과 dialog는 180ms 이하 opacity/translate만 허용한다.
- reduced-motion에서는 즉시 표시한다.
