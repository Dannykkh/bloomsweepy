# Layout Blueprint: compact composer delta

| Block | Anatomy | Emphasis |
|---|---|---|
| 기존 status | 실제 단계·시간·오류 | 개입 상태만 |
| input shell | 전체폭 textarea → footer | 초안/전송 |
| footer | 모델 trigger·추론 trigger → 44pxsend | send하나 |
| popup | Zap/current/reset → model → range | current강도 |
| exceptional state | 오류·미지원 이유/명시 복원 | 필요한 개입만 |

```text
                    [current effort / reset]
                    [model >               ]
                    [======== thumb        ]
+------------------------------------------+
| question draft                           |
| [model v] [reasoning v]               [↑] |
+------------------------------------------+
```

입력부 max-width800px 유지. popup≤360px, container폭 이내, toolbar위로 정렬.
긴 모델 목록 스크롤, footer flex min-width0, send축소금지. 사이트맵은 단일컴포넌트라 생략.
입력창 아래 안내 문장과 정상 팝업 설명은 없다. 오류·미지원·저장값 복구 안내는 별도 상태로 유지한다.
Claude/Grok/Agy도 동일한 toolbar/popup을 쓰고 지원 단계만 바뀐다. Settings는 기존 Picker를
유지하며 공급자/모델별 공유 선호를 통해 어느 화면에서 변경해도 같은 선택을 표시한다.

| Scene/component | User purpose | Trigger | Engine | Timing | Reduced-motion | Fallback | Cleanup/test |
|---|---|---|---|---|---|---|---|
| popup | 현재 선택 조절 | trigger | CSS opacity/transform | ≤120ms | 없음 | 즉시 열림 | outside/Escape listener 정리·focus복귀 |
| slider | 강도 단계 선택 | input | native range | 즉시 | 동일 | keyboard | 지원 단계·Home/End·null복원 |

#tags: layout, composer, toolbar, popover, motion-contract, multi-provider
