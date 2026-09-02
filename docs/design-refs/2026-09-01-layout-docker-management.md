# Layout Blueprint: 조건부 Docker 용량 작업면

## Direction

- Interface mode: 전용 Docker 화면은 Data Instrument, 정리는 Waiting State, 대화는 기존 Agent Workbench의 Docker 대상 delta.
- Preserve: 기존 설정 2열, 대화 transcript/composer, 한 겹 dialog, 14px 최소 글자.
- Render candidates: 생략. 승인된 BroomSweepy 화면에 전폭 설정 행과 문맥형 대화 행동을 추가하는 국소 변경이다.

## Block Sequence

| # | Block | Anatomy check | Variation |
|---|---|---|---|
| 1 | Developer tool opt-in | 제목, 짧은 설명, native checkbox, 기본 꺼짐 | 설정 그리드 전폭 얇은 패널 |
| 2 | Conditional navigation | `Docker 용량`, Boxes 아이콘, 활성 상태 | 켠 경우에만 용량 관리 다음에 삽입 |
| 3 | Docker status | CLI·daemon·version·조회 시각 | 전용 화면 첫 블록 |
| 4 | Usage rows | 범주, 사용량, 정리 가능 최대값, 위험 설명 | 카드 대신 divide-y 행 |
| 5 | Actions | `Docker 대화 시작`, `Docker 정리 검토` | 대화는 폴더 없이, 정리는 primary |
| 6 | Cleanup dialog | 범주 체크, 볼륨 제외, 복구 불가 확인, primary 1 | 위험 정보 한 겹 |
| 7 | Result | 범주별 결과, 부분 완료, 갱신 값 | dialog 안에서 상태 전환 |

## Wide — 1280×820

```text
[설정]
| 개발 도구 관리             Docker 용량 관리 [사용] |
| 사이드바에 Docker 용량 메뉴가 표시됩니다. [열기]  |

[사이드바] 용량 관리 / Docker 용량 / 파일 이름 찾기

[Docker 용량]
| Docker Desktop 4.x · daemon 연결됨             [다시 확인]      |
| 이미지       39.2 GB     정리 가능 최대 2.7 GB                  |
| 컨테이너      0.9 GB     정리 가능 최대 8.1 MB                  |
| 볼륨          5.0 GB     사용량만 표시 · 자동 정리 안 함        |
| 빌드 캐시    26.7 GB     정리 가능 최대 21.1 GB                  |
|                              [Docker 대화 시작] [정리 검토]     |
```

## Compact — 760×600

```text
Docker 용량
Docker 연결됨 · 방금 확인
------------------------------
빌드 캐시
26.7 GB · 정리 가능 최대 21.1 GB
[Docker 대화 시작]
[Docker 정리 검토]
```

- 글자를 줄이지 않고 각 사용량 행의 수치를 두 번째 줄로 내린다.
- 대화 제안은 요약 뒤 행동을 전폭으로 둔다.
- 확인창은 범주 체크 행과 복구 불가 확인을 생략하지 않고 내부 스크롤을 허용한다.

## State Placement

| State | Placement | Next action |
|---|---|---|
| disabled | 설정 개발 도구 첫 행 | 기능 켜기 |
| CLI 없음 | Docker 전용 화면 상태 행 | Docker 설치 후 다시 확인 |
| daemon 중지 | Docker 전용 화면 상태 행 | Docker 실행 후 다시 확인 |
| ready | Docker 전용 화면 사용량 행 | 다시 확인, Docker 대화 또는 정리 검토 |
| preview | modal | 범주 선택·복구 불가 확인·취소 |
| running | 같은 modal | 중단 요청 |
| partial/error | 같은 modal | 결과 확인 후 새 미리보기 |
| success | 같은 modal | 닫기·후속 질문 |
