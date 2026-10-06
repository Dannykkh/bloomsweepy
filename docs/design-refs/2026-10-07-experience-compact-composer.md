# Experience Contract: Compact conversation composer

## Source Mode

Mode: benchmark. Evidence: 2026-10-07-brief-compact-composer.md,
2026-10-07-benchmark-compact-composer.md, 사용자 첨부2장. 기존 chat-workbench의 입력부 delta.

## Product Facts

| Claim | Source | Captured at | Freshness/status | Allowed presentation |
|---|---|---|---|---|
| 강도 지원은 선택 모델별 metadata | assistantModelPreference.ts | 2026-10-07 | current | supportedEfforts만 표시 |
| 빈 override는 CLI 기본값, none은 실제 강도 | assistantModelPreference.ts | 2026-10-07 | current | 초기화와 추론 없음을 구분 |
| 공급자/모델별 저장·동기화는 기존 hook 소유 | useAssistantModelPreference.ts | 2026-10-07 | current | 같은 상태/요청을 사용 |
| Claude CLI 메타데이터 조회는 질문 없이 가능 | 설치 CLI2.1.291 initialize control response | 2026-10-07 | metadata-only PASS, 실제 답변 아님 | 조회한 모델/지원 단계만 표시 |
| Grok/Agy 실제 목록·추론은 공급자별 CLI 계약 | 공식 CLI 문서/소스·다중 공급자 QA | 2026-10-07 | 실제 CLI 미설치, 실행 NOT RUN | 검증 범위를 구분하고 임의 실제 선택지 금지 |
| Grok/Agy 공식 참고 모델은 읽기 전용 | 공식 설치/설정/headless 문서 | 2026-10-07 | 문서 예시9개, 설치·계정 미확인 | 실제 목록이 없을 때 별도 표시, 선택/추론/선호에 사용 금지 |

## Benchmark Sources

사용자 제공 crop2장과 benchmark 문서. 원본 촬영·반응형·모션 증거는 unavailable.

## Page Goal

입력창을 떠나지 않고 모델/추론을 선택해 질문을 전송한다.
정상 상태에서 큰 select와 설명이 대화 높이를 차지하지 않는 것이 관찰 가능한 성공이다.

## Audience and Tasks

8GiB Mac의 단일 사용자. 질문 초안을 작성하고 현재 모델·강도를 확인/조절한다.
CLI 상태/목록 확인이 선택의 시작 조건이며 optional 모델은 목록 실패 시 CLI 기본값 경로를 유지한다.
질문 전송·실제 진행/취소가 완료 경로다. Settings에서도 같은 모델/강도 선호를 조절한다.
모델 변경으로 지원 단계가 달라지거나 stale 저장값 때문에 전송 불가할 수 있다.

## System Roles

NOT APPLICABLE: single role. 모델은 선택의 대상이며 실행 권한 주체가 아니다.

## Header and Navigation

기존 앱/대화 헤더·폴더/공급자·권한 버튼 유지. 하단 toolbar에 모델과 추론 버튼.
팝업 모델 이름을 누르면 같은 picker의 모델 목록으로 이동한다. 닫기/Escape로 초안 복귀.

## Core Message

질문은 입력창에, 세부 설정은 필요할 때만. 현재 강도/모델 이름이 실제 상태의 증거다.
파일 조회와 실행 권한은 기존 앱 계약을 유지한다.

## Content Integrity

| Content item | Classification | Evidence | Presentation rule |
|---|---|---|---|
| 모델·추론 라벨 | verified | provider catalog | ID/지원 metadata와 일치 |
| 공식 참고 모델 | reference | 2026-10-07 공식 문서 예시 | 읽기 전용·설치/계정 미확인; 실제 선택·강도 목록과 분리 |
| fixture 선택 모델 | prototype | 합성 harness | 합성 QA 라벨, 실제 AI 전송 없음 |
| slider particles | prototype | reference-inspired CSS | 정적 장식, 진행/토큰 수로 해석 금지 |

## Section Order

기존 transcript → 실제 진행/오류 → textarea → compact 모델/추론 toolbar와 전송.
입력창 아래 상시 설명과 전송 범위 문장은 제거한다. 정상 팝업도 현재 선택과 조작만 보여 준다.
팝업은 toolbar 위에 열리며 대화/입력의 레이아웃을 밀지 않는다.

## CTA Strategy

Primary: 질문 보내기/AI 응답 취소. Secondary: 모델 선택, 추론 강도 선택, CLI 기본값 복원.
picker의 모든 버튼은 type=button이며 선택/닫기만으로 질문을 전송하지 않는다.
선택은 즉시 기존 hook에 반영한다. 성공은 현재 라벨/aria-selected 값, 저장 실패는 기존 경고.

## Trust Strategy

목록 조회와 계정의 실제 모델 접근권, 요청한 강도와 서버가 적용한 강도를 구분한다.
이 제한은 설정/정본 문서에 두고 정상 팝업의 반복 설명은 제거한다. 미지원 값을 임의 보정하지 않는다.
stale는 입력부에 경고와 기본값 복원 버튼을 남기고 기존 전송 차단을 유지한다.
busy/provider/model 변경 시 예전 popup을 닫는다. 어떤 조작도 권한을 켜거나 파일을 실행하지 않는다.
목록이 없는 상태는 CLI 미설치/상태 확인 필요/로그인/서비스/설치 모델 없음/조회 미완료로 구분한다.
Grok/Agy 미설치에서는 공식 참고 모델과 설치 안내를 별도로 표시하되 실행 가능한 선택지로 꾸미지 않는다.
참고 항목의 설치·계정 미확인을 명시하고 최신/전체/실행 가능 목록을 보장하지 않는다.

## Asset Provenance

프로젝트 내부 Pretendard/Lucide/CSS만 사용. 첨부는 관찰 근거이며 외부 코드·로고를 복제하지 않는다.
새 이미지/폰트/영상·dependency 없음.

## Desktop Structure

1280×820와760×600. textarea 전체폭, footer 왼쪽 작은 두 trigger, 우측44px원형 전송.
popup 폭≤360px, 모델 목록은 제한 높이 스크롤. transcript만 기존 방식으로 스크롤.

## Mobile Transformations

| Desktop element | Operation | Mobile result | Reason |
|---|---|---|---|
| 모델 이름 | compress | 긴 이름 ellipsis+전체 접근성 이름 | 전송 버튼 영역 보존 |
| 정상 상태 설명 | remove | 입력부/정상 popup에 없음 | 사용자 후속 요청·대화 높이 유지 |
| 오류·미지원·복구 안내 | retain | 필요한 상태에서만 읽기 | 잘못된 선택을 숨기지 않음 |
| textarea·전송 | retain | 한 입력부 하단 유지 | 주요 질문 과업 |
| popup | compress | 컨테이너폭 내 최대360px | 390px/좁은 pane에서 잘림 방지 |

## States

| State | Trigger | User sees | Available action | Recovery |
|---|---|---|---|---|
| loading | CLI 점검 | CLI 기본값/비활성 trigger, 기존 준비 상태 | 대기 | 기존 CLI 재확인 |
| empty | 실제 목록 없음 | 상태별 원인, G/A는 별도 읽기 전용 공식 참고 모델 | 설치/로그인/서비스 확인·목록 재확인; 준비된 optional CLI만 기본값 | 실제 목록 복구 시 참고 목록 숨김 |
| error | stale model/effort | 기존값과 경고 | 명시 기본값 복원/다른 지원값 | 전송 차단 해제 |
| success | 값 선택 | 현재 모델·강도 | 질문 전송 | 재선택·초기화 |
| working | sending/sessionBusy/provider.busy | 기존 실제 진행·취소, selector 잠금 | 취소 | 작업 종료 후 선택 |
| unsupported | CLI 옵션/모델별 지원 증거 없음·구형 host | CLI 기본값과 지원하지 않는 이유 | 모델/공급자 변경 | 공급자 이름만으로 강도 추정하지 않음 |

## Provider Selection Contract

| Provider | Model source | Effort source/rule | Default/unknown behavior |
|---|---|---|---|
| Codex | 설치 CLI catalog | 모델별 실제 supported levels | 기존 계약 유지 |
| Claude Code | 질문 없는 initialize response, 실패 시 공식 aliases | 실제 모델 metadata의 지원 단계, CLI 옵션과 교차 확인 | 미확인 기본 강도를 만들어 내지 않음; Haiku 등 metadata 없으면 선택 없음 |
| Grok | 설치 CLI의 `models` 목록 | CLI effort 옵션 확인 + 확인된 정확한 모델 계약 | Grok4.7/4.6 low·medium·high·xhigh,4.5 low·medium·high; 기타 모델은 미확인 단계 없음 |
| Antigravity(agy) | 설치 CLI의 `models` 목록 | 같은 base의 실제 variant 행, 일치하는 slug/label, CLI effort 옵션 | 목록에 없는 variant/강도 생성 금지 |
| Ollama | 기존 설치 모델 목록 | 이번 변경에서 추론 단계 없음 | required 모델 계약 유지 |

기본 모델/강도 선택은 인자 override를 생략한다. 명시 선택은 공급자의 실제 모델/effort 인자로
전달하고 재시도·조사 라운드에도 유지한다. 선택값의 echo는 서버의 실제 적용 결과를 증명하지 않는다.
Settings와 채팅은 같은 선호를 사용하며 공급자 전환으로 다른 공급자의 ID/강도가 섞이지 않는다.
목록 갱신은 새 선택지를 갱신할 뿐 저장된 모델을 자동으로 최신 모델로 바꾸지 않는다.

Grok/Agy 실제 `provider.models`가 없을 때만 frontend의 별도 읽기 전용 참고 목록을 보여 준다.
공식 문서 예시는 Grok `grok-build`/`grok-4.7`2개와 Agy Gemini3.8/3.7/3.6 Flash의
High/Medium 및 Gemini3.1 Pro High7개다(2026-10-07 확인). 참고 목록은 `provider.models`,
실제 선택 옵션·선호 저장·지원 추론 계산·질문 인자로 합치지 않는다. 실제 모델이 있으면 참고를 숨긴다.
미설치 상태의 설치 안내가 질문 입력창 아래 정상 상시 설명을 되살리지 않게 한다.
구체적인 경계와 공식 링크는 [모델 선택 정본](../architecture/assistant-model-selection.md)을 따른다.

이 표는 확장된 구현 계약이다. 현재 검사 완료와 남은 검사는
[QA 기록](../qa/2026-10-07-multi-provider-model-selection.md)에서 구분한다.

## Performance Budget

새 dependency/Canvas/WebGL/상시 timer/blur 없음. static track dots만 signature로 제한한다.
open 상태에서만 outside/Escape listener를 쓰고 닫힘/unmount에서 정리한다. 기존 host 메모리 계약 유지.

## Accessibility Contract

44px hit area, label 있는 native range의 방향키/Home/End·aria-valuetext.
popup은 labelled nonmodal dialog, 열림 상태 aria-expanded, Escape 닫기/trigger focus 복귀.
입력 초안과 IME Enter guard 유지, 밖 클릭/모델 전환/기본값 복원이 submit하지 않는다.
본문≥14px, focus-visible·실제 합성 대비·reduced-motion 무모션. 색 외 현재값 텍스트.

## Adopt

compact trigger/round send, 현재 강도 중심 popup, 모델 부제·초기화·slider.

## Adapt

지원 단계만 정수로 매핑, CLI 기본값을 별도로 유지, 정상 설명 제거/복구 안내 유지,
기존 neutral glass와 violet token. 같은 UI를 각 공급자의 실제 모델 계약에 연결한다.

## Avoid

미구현 마이크/고정 Ultra/새 저장소/자동 모델보정/상시 파티클/권한 확대/파일 작업.

## Prompt Contract

GOAL — 입력창에서 모델·추론을 간결하게 선택.
AUDIENCE — 저자원 단일 사용자 데스크톱.
TASK — 초안 작성·모델/강도 선택·전송/취소.
FLOW — trigger→popup→선택→기존 hook→기존 요청.
HEADER — 기존 헤더 보존, popup 현재값 우선.
MESSAGE — 현재 상태만 기본 노출, 정상 설명 제거, 오류/복구만 필요할 때.
FACTS — 공급자별 실제 목록·지원 증거와 빈 override 의미 유지.
CONTENT_INTEGRITY — 합성 QA/읽기 전용 공식 참고/실제 선택 목록 구분, 가짜 모델/진행 금지.
SECTION_ORDER — 입력→작은 toolbar·전송, 위쪽 popup.
CTA — 질문 보내기 primary, picker type=button.
TRUST — stale/저장 오류·기본값 복원, 권한 경계 유지.
ASSETS — 내부 폰트·Lucide·CSS만.
LAYOUT — textarea 전체폭/44px send/≤360pxpopup.
RESPONSIVE — 라벨 압축·정상 설명 제거·container clamp.
STATES — loading/empty/error/success/working/unsupported.
PERFORMANCE — 새 dependency/상시 timer/blur 없음.
ACCESSIBILITY — native range·focus/Escape·≥14px·44px.
PRESERVE — Settings picker 재사용, hook·공유 저장, progress/취소, 권한·삭제 경계.
EXTEND — 사용자 후속 요청의 Claude/Grok/Agy 모델 catalog·effort 인자·설정/채팅 공통 선택.
EXCLUDE — 계정/로그인/권한 정책 변경·CLI 자체 파일 작업·미구현 마이크·임의 모델/강도.
SUCCESS — 390px·최소 창·키보드·오발송0·공급자별 실제 지원값/저장 복원·검증 범위 구분.

## Success Checks

- 모델/강도 선택·Escape가 초안을 전송하지 않는다.
- 지원 단계가 바뀌면 popup/state가 현재 모델에 맞고 stale는 기존처럼 전송 차단한다.
- 기본값 초기화는 null override이며 none과 다르다.
- 입력부와 popup이 390px/760×600에서 접근 가능하고 현재값/대비가 읽힌다.
- 실제 파일 작업·AI 전송 없는 fixture와 native 설치 확인을 구분한다.
- Claude/Grok/Agy 선택도 Settings와 채팅에 공유되고 실제 지원하지 않는 강도를 만들지 않는다.
- 입력창 아래 및 정상 팝업의 상시 설명이 없으며 오류·복구 행동은 접근 가능하다.
- 미설치 G/A의 공식 참고 항목이 실제 선택·추론·선호·요청에 섞이지 않고 실제 목록이 있으면 숨겨진다.

#tags: compact-composer, model-picker, reasoning-slider, experience-contract, multi-provider
