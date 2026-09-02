# Layout Blueprint: 대상별 대화 세션

## Direction

- Interface mode: Agent Workbench delta.
- Preserve: 기존 단일 transcript, provider picker, composer, 접힌 권한 정보.
- Render candidates: 생략. 승인된 대화 작업면에 세션 도구와 비율 링만 추가하는 국소 변경이다.

## Block Sequence

| # | Block | Anatomy check | Variation |
|---|---|---|---|
| 1 | Session toolbar | 새 폴더 대화, 조건부 Docker 대화, native session select, 현재 세션 삭제 | 카드 대신 얇은 행 |
| 2 | Conversation scope | 폴더 이름·논리 용량·ring 또는 Docker 범주 합계, 공급자 | 대상 종류를 텍스트와 아이콘으로 병기 |
| 3 | Transcript | 저장 메시지, 공급자 라벨, loading/empty/error | 기존 단일 scroll pane |
| 4 | Composer | textarea, 보내기 또는 취소, 전송 범위 | 기존 구조 보존 |

## Wide — 1280×820

```text
[새 폴더 대화] [Docker 대화] [최근 대화: views · 8개 · 9월 1일 17:42 v] [삭제]
views                         120 KB  (ring) D: 0.1% 미만    Claude Code
--------------------------------------------------------------------------
대화 기록
...
--------------------------------------------------------------------------
[선택한 폴더에 관해 질문                                      ][보내기]
```

## Compact — 760×600

```text
[새 대화]
[views · 8개 · 17:42                         v] [삭제]
views
120 KB  (ring) D: 0.1% 미만
[Claude Code                                             v]
------------------------------------------------------------
대화 기록
```

- 글자는 14px 아래로 줄이지 않는다.
- 세션 도구가 두 행이 되어도 transcript 폭을 줄이지 않는다.
- 원형 그래프는 장식이 아니라 `aria-label`과 정확한 텍스트를 가진 데이터 표시다.
