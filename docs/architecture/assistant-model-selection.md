# 내장 채팅의 CLI 모델·추론 강도 선택

현재 개발본 계약. 공개 v1.7.0 다운로드와 구분한다.

## 사용자 흐름과 저장

채팅 입력창의 compact 모델/추론 controls와 설정의 기존 Picker는 같은 선호 hook을 사용한다.
공급자마다 마지막 모델을, 모델마다 추론 강도를 별도로 기억하며 공급자를 바꿔도 다른 공급자의 ID를 보내지
않는다. 질문이 실행 중이면 입력창에서 모델을 바꿀 수 없다. 설정은 현재 native busy
상태로 선택을 제한하되 상태 다시 확인 버튼은 사용할 수 있다.

채팅은 전체폭 textarea 아래 작은 모델/추론 trigger와44px 원형 전송/취소 버튼을 둔다.
추론 팝업은 현재 강도·모델 이름·실제 지원 단계의 native range를 중심으로 위쪽에 열린다.
사용자 요청대로 입력창 아래 및 정상 팝업의 반복 설명은 제거한다. stale·미지원·저장 실패 등
복구 안내는 숨기지 않는다. 기본 강도를 확인하지 못하면 현재 단계를 만들어 표시하지 않고
명시 강도 선택 또는 CLI 기본값을 유지하는 경로를 제공한다. 권한/전송 범위 안내는 기존
연결·권한/설정 경로에서 확인한다.

`bloomsweepy.assistant-models.v1`은 WebView의 로컬 저장소에 version1·provider·모델
map과 optional `reasoningEfforts[provider][model]`만 담는다. 기존version1은 그대로 읽으며
기본 강도는 저장하지 않는다. 강도 선호는 최대16개로 현재 선택을 보존하며 다른 기존 항목을
제한한다. 질문·계정·인증·권한·실행 계획은 포함하지 않는다. 기존 provider/Ollama
키는 읽기 마이그레이션하며 새 schema가 우선한다. 저장 읽기/쓰기가 실패하면 현재 실행의
선택은 유지하고 경고한다. 형식·공급자·ID를 검증하며 입력 JSON4096자와 ID160자 상한을
둔다. 같은 창의 구독/event와 다른 창의 storage event로 동기화한다.

## 공급자별 계약

| 공급자 | 선택 | 목록 출처 | 실제 실행 |
|--------|------|-----------|-----------|
| Codex | 기본값 또는 ID | 설치 CLI `debug models`, 실패 시 `--bundled` | 선택 시 별도 `--model ID`, 기본값은 생략 |
| Claude Code | 기본값 또는 ID/별칭 | 설치 CLI initialize metadata, 실패 시 공식 aliases | 선택 시 별도 `--model ID` |
| Grok | 기본값 또는 ID | 준비 상태 검사의 실제 `models` 출력 | 선택 시 별도 `--model ID` |
| Antigravity(agy) | 기본값 또는 slug | 준비 상태 검사의 실제 `models` 출력 | 선택 시 별도 `--model slug` |
| Ollama | 설치 모델 필수 | `ollama list` | 기존 `run model`, 실행 전 설치 여부 재검증 |

Codex/Claude/Grok/Agy의 실제 CLI help에서 `--model`을 확인하지 못하면 모델 선택은 기본값
전용으로 표시하고 그 인자를 보내지 않는다. 이는 필수 headless/격리 옵션이 확인된 경우에만
기존 기본값 실행 경로를 유지한다는 뜻이며, 호환되지 않는 CLI의 실행을 허용하지 않는다.
공급자 자체가 모델을 지원하지 않는다는 일반적 주장과 현재 앱 연동의 범위를 구분한다.
구형 native host가 metadata를 보내지 않으면 Ollama만 required, 나머지는 unsupported다.

### 조회 시점과 상한

Codex/Claude의 추가 카탈로그 조회는 상태 갱신에서만 수행하며 질문 사전 검사나 같은 조사
루프의 각 라운드에서 반복하지 않는다. Grok/Agy는 준비 상태 검사에 필요한 `models` 출력
한 번을 목록으로도 재사용한다. 질문 시작 때 준비 상태를 재확인하면 이 조회가 다시 발생할
수 있지만 별도의 카탈로그 명령을 중복 실행하지 않으며 조사 라운드마다 반복하지 않는다.

각 카탈로그 stdout/stderr는1MiB, 반환 모델은 최대64개다. ID·표시명·지원 enum·실제로
확인된 유효한 기본 강도만 UI에 반환한다. 모델 지침·임의 능력 설명·세션/인증/계정 raw 출력은
UI나 선호 저장소에 붙이지 않는다. CLI의 목록 metadata 조회는 가능하지만 앱의 모델/강도
선택으로 사용자 CLI 전역 설정을 수정하지 않는다.

### 공급자별 목록 파싱

- Codex: live5초→bundled3초. `list`/`show_ui`만 표시하며 hide/unknown을 제외한다.
- Claude:8초 제한의 stream-json initialize control request만 전송한다. user/prompt frame 없이
  request ID가 일치하는 성공 control response의 `models`만 읽고 `default` 행은 앱의 빈
  기본값과 중복되므로 제외한다. 실제 ID/표시명 및 effort metadata만 추출한다.
  `resolvedModel`의 검증된 canonical `claude-{family}-{major}[-{minor}][-{snapshot}]`에서
  `Opus 5.5` 같은 버전 라벨을 만든다. 날짜는 모델 버전과 구분하고, 알 수 없는 값과
  임의 description에서 버전을 추정하지 않는다. 실행·선호 ID는 `opus` 등 원래 값 그대로다.
  동일한 초기화에 startup `--model fable`을 지정해 현재 해석되는 Fable alias 행도 받는다.
  기존 Fable 5 pinned 행을 5.1로 바꾸지 않고 별도 실제 `fable` 행을 표시하며 추가 probe는 없다.
  이는 metadata-only 임시 프로세스의 선택일 뿐 채팅 기본값·전역 CLI 설정을 바꾸지 않는다. 추가 조회의
  필수 옵션을 help에서 확인하지 못하거나 목록 조회가 실패하면 공식 `sonnet`·`opus`·`haiku`
  aliases로 돌아가되 버전·추론 단계·기본 강도를 추정하지 않는다. SDK는 프로토콜 근거이며
  Python SDK 실행/새 의존성을 추가한 구조가 아니다.
- Grok: `Available models:` 다음의 실제 bullet 행만 파싱한다. 다른 header/footer/오류 문장을
  모델 ID로 만들지 않는다. `models`를 정식 command로 광고한 CLI에서만 실행한다.
- Agy: 실제 `models`의 hyphenated slug/표시명2열만 읽고 header/오류/유효하지 않은 ID·표시명을
  제외한다. 확인되지 않은 JSON schema나 IDE 모델 페이지로 CLI 목록을 대신하지 않는다.

목록 제공과 계정별 사용 권한 검증은 다르다. Claude 별칭의 실제 모델 버전은 공급자가
결정한다. 목록 실패는 CLI 연결 실패가 아니며 기본값 사용/재확인이 가능하다. 저장된 ID가
목록에 없으면 “목록에서 확인되지 않음”으로 보존하고 임의로 대체하지 않는다.
CLI/구형 host가 모델 선택을 지원하지 않는데 저장 ID가 있으면 숨기지 않고 전송을 차단한다.
사용자가 명시적으로 기본값으로 변경해야 null 전송이 가능하며, 원래 빈 기본값은 계속 허용한다.
로그인되지 않은 건강한 CLI도 질문 없는 모델 metadata를 표시할 수 있다. 목록 표시는 로그인
완료가 아니며 로그인 필요 상태에서는 채팅 요청을 차단한다.
버전 라벨은 현재 선택 목록의 해석 결과이며 서버가 실제 생성에 사용한 모델의 증거는 아니다.
과거 대화의 저장된 모델 ID를 오늘의 alias 버전으로 소급해서 바꾸지 않는다.

### 목록이 없을 때의 안내와 공식 참고 목록

빈 목록을 모두 “모델 없음”으로 처리하지 않는다. frontend의 presentation helper는 native 상태에
따라 CLI 설치 필요, 상태 확인 필요, 로그인 필요, 서비스 중지, 설치 모델 없음, 목록 조회 미완료를
구분한다. 이 Mac에서 Grok/Agy의 실제 목록이 없는 원인은 해당 CLI 미설치이며 앱의 모델 선택
지원 자체가 없다는 뜻이 아니다. 설치/상태 확인 후 실제 CLI 목록을 다시 조회한다.

Grok/Agy의 실제 모델 목록이 비어 있을 때만 별도의 읽기 전용 “공식 문서 참고 모델”을 표시한다.
2026-10-07 확인한 공식 문서 예시는 Grok의 `grok-build`·`grok-4.7`2개와 Agy의
`gemini-3.8-flash-high`·`gemini-3.8-flash-medium`,
`gemini-3.7-flash-high`·`gemini-3.7-flash-medium`,
`gemini-3.6-flash-high`·`gemini-3.6-flash-medium`, `gemini-3.1-pro-high`7개다.
이는 최신/전체/현재 계정에서 실행 가능한 목록이라는 보장이 아니며 설치·계정 사용 가능 여부
미확인을 명시한다. 참고 목록에서 모델을 선택하거나 추론 강도를 설정하지 않는다.

참고 항목은 frontend 표시 데이터만 사용한다. `provider.models`, 선호 저장, 실제 선택 옵션,
`model`/`reasoningEffort` 요청, 지원 단계 계산에 합치지 않는다. 실제 `provider.models`가
하나라도 있으면 참고 목록을 숨기고 실제 선택 목록을 표시한다. 공식 문서 링크는 안내이며
CLI 설치·로그인이나 앱 권한 변경을 자동 실행하는 동작이 아니다. 이런 예외 상태의 안내는
모델 팝업/설정에 두고 입력창 아래 정상 상시 설명을 되살리지 않는다.

## 추론 강도

명시 모델을 선택하면 해당 모델의 실제 지원 증거만 채팅과 설정에 표시한다. 저장 schema는
`none/minimal/low/medium/high/xhigh/max/ultra`를 허용하지만 모든 공급자에8단계를
제공하는 의미가 아니다. 공급자별 허용 enum 및 설치 CLI help와 교차 확인한다. 기본 강도는
지원 목록 안의 실제 metadata가 있을 때만 표시하며 미확인 기본 강도를 하드코딩하지 않는다.
CLI 기본 모델은 추론 강도도 기본값만 사용한다. 구형 host/metadata 없는 모델에 임의 강도
옵션을 만들지 않는다.

| 공급자 | 모델별 지원 증거 | 명시 인자 |
|---|---|---|
| Codex | `supported_reasoning_levels[].effort`, 유효한 `default_reasoning_level` | `--config`, `model_reasoning_effort="<effort>"` |
| Claude Code | initialize의 `supportsEffort:true`와 `supportedEffortLevels`, 설치 help의 실제 effort choices | `--effort <effort>` |
| Grok | 실제 목록 ID와 아래 확인된 정확한 모델 계약, 설치 help의 `--effort` | `--effort <effort>` |
| Agy | 같은 base의 실제 suffix/label 일치 variant 행, 설치 help의 `--effort` | `--effort <effort>` |
| Ollama | 이번 연동에 지원 metadata 없음 | 강도 인자 없음 |

Claude는 low/medium/high/xhigh/max 중 metadata와 설치 help가 모두 지원하는 단계만
허용한다. `defaultEffortLevel`도 지원 목록 안에 있을 때만 사용한다. aliases fallback에는
effort metadata가 없으며 Haiku 등의 지원을 이름으로 추정하지 않는다.

Grok의 확인된 정확한 계약은 `grok-4.7`/`grok-4.6`의 low/medium/high/xhigh,
`grok-4.5`의 low/medium/high다. 실제 목록에 있는 ID에만 적용하고 다른 모델은 미확인
단계를 제공하지 않는다. 이 제한은 공식 reasoning 계약을 사용하는 최소 어댑터이며 향후
안정적인 metadata-only ACP 목록으로 모델별 지원을 직접 읽을 수 있을 때 재검토한다.
`--effort`를 사용하며 현재 공식 CLI의 `--reasoning-effort` alias와 구분한다.

Agy는 `-low`/`-medium`/`-high` suffix와 표시명의 `(Low)`/`(Medium)`/`(High)`가
일치하는 실제 행을 같은 base로 묶고 그 가족에서 확인된 단계만 제공한다. 누락된 medium
등을 생성하지 않는다. 선택한 원래 slug는 유지하고 명시 effort만 별도로 전달한다.
Grok/Agy의 기본 강도는 목록으로 확인하지 못했으므로 `None`이며 서버/CLI 설정의 기본값을
앱에서 추정하지 않는다.

저장한 강도가 지원 목록에서 사라지거나 목록을 확인할 수 없으면 선택과 경고를 유지하고
질문을 차단한다. 사용자가 기본값 또는 지원되는 강도로 바꿔야 다시 보낼 수 있다.
공급자·모델을 바꾸면 해당 모델의 선호를 가져오므로 다른 모델의 높은 강도가 섞이지 않는다.
앱 시작/CLI 업데이트가 저장된 모델을 자동으로 최신 모델로 바꾸지 않는다.

명시 강도는 요청 `reasoningEffort`로 전달하고 같은 조사 루프의 모든 라운드에서 공급자별
별도 argv로 사용한다. null/누락은 override를 생략한다. native는 공급자별 enum·명시 모델을
검증하며 비Codex는 질문 시작 help 재확인 결과의 실제 effort 지원 여부도 검사한다. 모델별
지원 metadata는 UI 선택에 사용하고 최종 모델/강도 조합은 CLI/서버도 검사한다. 거부되면 선택을 유지한 채
옵션 확인 오류를 안내하며 자동 fallback하지 않는다. 강도가 높으면 응답 시간이 늘 수 있고
현재 Codex 한 라운드 제한120초·조사 전체10분·취소 기능은 유지한다.

## 실행·안전 경계

빈 optional 선택은 request `model:null`이다. 기본값은 기존 앱의 격리 실행 인자를 따른다.
특히 Codex의 `--ignore-user-config`는 유지하므로 사용자의 전역 model 설정을 읽어
앱 선호로 가져오는 동작이 아니다. 명시 ID는 ASCII영숫자로 시작하고
영숫자·`._:/-`만 허용하며160자 이하이다. Claude 카탈로그가 반환하는 정확한 마지막
`[1m]` suffix 한 번만 예외로 허용한다. 임의 괄호/중복 suffix/공백/제어문자/셸 문법은
허용하지 않으며 frontend/native가 같은 규칙을 적용한다. 셸 문자열이 아닌 별도 argv로 전달한다.
질문 요청의 모델 값은 해당 조사 루프 전체에서 유지한다. 모델 거부 시 모델/계정 접근권
오류로 안내하고 기본 모델로 자동 전환하지 않는다. 응답의 모델 표시는 요청한 선택을
나타내며 응답 `reasoningEffort`도 요청 override의 echo이다. 서버가 실제로 해석한 숨은
모델 버전이나 내부 추론 실행량을 증명하지 않는다.

Codex의 read-only sandbox·규칙/사용자 설정 격리와 Claude의 tools/hooks/MCP 격리,
기존 앱 기능 dispatcher·전송 상한·동의·실행 직전 검증은 유지한다. 이 UI는 외부 MCP
클라이언트의 모델/추론 설정이나 계정/로그인, 파일 삭제 권한을 변경하지 않는다.

Claude의 일반 chat 격리 인자는 그대로 유지한다. metadata 조회만 기존 격리에
`--safe-mode`·`--permission-prompts none`·빈 system prompt·stream-json 옵션을 더하고,
tools/hooks/MCP/session 저장을 비활성화하며 자식의 `CLAUDECODE` 환경 표시를 제거한다.
자동 업데이트도 끈다. 질문 없는 조회가 사용자/계정 정보를 반환해도 모델 필드만 추출하고
raw 인증 로그를 UI에 노출하지 않는다.

Grok은 `--tools ""`만으로 deny-all이 되지 않으므로 명시적 `--deny "*"`를 함께 전달한다.
`dontAsk`·subagent/web search 비활성화·headless 옵션과 이 deny 옵션을 help 필수 검사에
포함한다. Agy의 기존 `--print`·`--sandbox` 계약은 유지한다. 두 CLI는 `models` command
광고 여부도 검사해 알 수 없는 command가 대화 prompt로 실행되는 일을 피한다.

macOS/Linux 실행 파일 탐색은 Grok 공식 설치 위치인 `~/.grok/bin`도 후보에 포함한다.
기존 PATH/명시 후보의 우선순위는 유지하며 이 fallback 추가로 설치·로그인·셸 설정을 바꾸지 않는다.
실행 파일 발견은 호환성·모델 조회·계정 접근권 검증과 별개다.

## 검증 범위

현재 Mac의 Claude Code2.1.291에서 질문 없는 실제 initialize metadata 응답은 확인했다.
Grok/Agy CLI는 이 Mac에 없어 실제 목록/응답/계정 접근권을 검증하지 않았다. 이 구조와
합성/단위 검사가 실제 생성·서버의 강도 적용·Windows 런타임을 증명하지 않는다. 최종 통합 및
설치형 진행 상태는 아래2026-10-07 QA를 따른다.

## 근거

- 코드: `assistant_provider.rs`, `assistant_model_catalog.rs`, `external_program.rs`, `assistantModelPreference.ts`, `useAssistantModelPreference.ts`, `AssistantComposerControls.tsx`, `AssistantModelPicker.tsx`, `AssistantModelSettingsPanel.tsx`, `AssistantView.tsx`.
- [Compact composer 계약](../design-refs/2026-10-07-experience-compact-composer.md)
- [다중 공급자 검증 기록](../qa/2026-10-07-multi-provider-model-selection.md)
- [검증 기록](../qa/2026-10-06-cli-model-selection.md)
- [CLI 업데이트·추론 검증 기록](../qa/2026-10-06-cli-reasoning-selection.md)
- [Codex CLI 공식 명령](https://learn.chatgpt.com/docs/developer-commands?surface=cli)
- [Claude Code 모델 설정](https://code.claude.com/docs/en/model-config)
- [Claude Code CLI reference](https://code.claude.com/docs/en/cli-reference)
- [Claude Agent SDK initialize control protocol](https://github.com/anthropics/claude-agent-sdk-python/blob/main/src/claude_agent_sdk/_internal/query.py)
- [Claude Agent SDK ModelInfo·resolvedModel 타입](https://unpkg.com/@anthropic-ai/claude-agent-sdk@0.3.291/sdk.d.ts)
- [Grok CLI reference](https://docs.x.ai/build/cli/reference)
- [Grok 설치 안내](https://docs.x.ai/build/overview)
- [Grok 모델 선택 설정](https://docs.x.ai/build/settings)
- [Grok headless scripting](https://docs.x.ai/build/cli/headless-scripting)
- [Grok reasoning 모델 계약](https://docs.x.ai/developers/model-capabilities/text/reasoning)
- [Antigravity CLI 설치 안내](https://antigravity.google/docs/cli/install/)
- [Antigravity CLI headless](https://antigravity.google/docs/cli/headless)
- [Antigravity CLI reference](https://antigravity.google/docs/cli/reference/)

#tags: cli모델, multi-provider, 추론강도, compact-composer, 설정공유, arch:012
