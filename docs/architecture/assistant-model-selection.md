# 내장 채팅의 CLI 모델·추론 강도 선택

현재 개발본 계약. 공개 v1.7.0 다운로드와 구분한다.

## 사용자 흐름과 저장

채팅 입력창 아래 `AI 모델`과 설정의 `AI 모델`은 같은 Picker·선호 hook을 사용한다.
공급자마다 마지막 모델을, 모델마다 추론 강도를 별도로 기억하며 공급자를 바꿔도 다른 공급자의 ID를 보내지
않는다. 질문이 실행 중이면 입력창에서 모델을 바꿀 수 없다. 설정은 현재 native busy
상태로 선택을 제한하되 상태 다시 확인 버튼은 사용할 수 있다.

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
| Claude Code | 기본값 또는 별칭 | 공식 `sonnet`·`opus`·`haiku` aliases | 선택 시 별도 `--model alias` |
| Ollama | 설치 모델 필수 | `ollama list` | 기존 `run model`, 실행 전 설치 여부 재검증 |
| Grok / Antigravity | 현재 연동은 기본값만 | unsupported | 모델 인자 없음 |

Codex/Claude의 실제 CLI help에서 `--model`을 확인하지 못하면 기본값 전용으로 표시한다.
공급자 자체가 모델을 지원하지 않는다는 일반적 주장과 현재 앱 연동의 범위를 구분한다.
구형 native host가 metadata를 보내지 않으면 Ollama만 required, 나머지는 unsupported다.

Codex 카탈로그는 상태 갱신 시에만 조회하며 각 질문의 사전 검사/라운드마다 반복하지 않는다.
live5초→bundled3초, stdout/stderr 각1MiB, visible `list`/`show_ui` 최대64개, ID·표시명과
지원 추론 enum/유효한 기본 강도만
반환한다. 모델 지침·능력 설명·desktop cache·인증 정보를 UI에 붙이지 않는다. CLI의
카탈로그 metadata 갱신은 가능하지만 사용자 CLI 전역 설정은 수정하지 않는다.

목록 제공과 계정별 사용 권한 검증은 다르다. Claude 별칭의 실제 모델 버전은 공급자가
결정한다. 목록 실패는 CLI 연결 실패가 아니며 기본값 사용/재확인이 가능하다. 저장된 ID가
목록에 없으면 “목록에서 확인되지 않음”으로 보존하고 임의로 대체하지 않는다.

## 추론 강도

Codex 모델을 명시 선택하면 같은 Picker에서 그 모델의 CLI 카탈로그가 제공한 단계만
표시한다. 카탈로그의 `supported_reasoning_levels[].effort`와
`default_reasoning_level`을 읽되 `none/minimal/low/medium/high/xhigh/max/ultra`만
중복 없이 최대8개 허용하고, 기본 강도는 지원 목록 안에 있을 때만 표시한다. 모델명을
하드코딩하지 않는다. CLI 기본 모델은 추론 강도도 기본값만 사용하며 다른 공급자/구형
host/metadata 없는 모델에 임의 강도 옵션을 만들지 않는다.

저장한 강도가 지원 목록에서 사라지거나 목록을 확인할 수 없으면 선택과 경고를 유지하고
질문을 차단한다. 사용자가 기본값 또는 지원되는 강도로 바꿔야 다시 보낼 수 있다.
공급자·모델을 바꾸면 해당 모델의 선호를 가져오므로 다른 모델의 높은 강도가 섞이지 않는다.
앱 시작/CLI 업데이트가 저장된 모델을 자동으로 최신 모델로 바꾸지 않는다.

명시 강도는 요청 `reasoningEffort`로 전달하며 같은 조사 루프의 모든 라운드에서 Codex
별도 argv `--config`, `model_reasoning_effort="<effort>"`로 사용한다. null/누락은 override를
생략한다. native는 enum·Codex·명시 모델을 검증하고, 최종 모델/강도 조합의 지원 여부는
CLI도 검사한다. 매 질문마다 카탈로그를 다시 조회하지 않는다. 거부되면 선택을 유지한 채
옵션 확인 오류를 안내하며 자동 fallback하지 않는다. 강도가 높으면 응답 시간이 늘 수 있고
현재 Codex 한 라운드 제한120초·조사 전체10분·취소 기능은 유지한다.

## 실행·안전 경계

빈 optional 선택은 request `model:null`이다. 기본값은 기존 앱의 격리 실행 인자를 따른다.
특히 Codex의 `--ignore-user-config`는 유지하므로 사용자의 전역 model 설정을 읽어
앱 선호로 가져오는 동작이 아니다. 명시 ID는 ASCII영숫자로 시작하고
영숫자·`._:/-`만 허용하며160자 이하이다. 셸 문자열이 아닌 별도 argv로 전달한다.
질문 요청의 모델 값은 해당 조사 루프 전체에서 유지한다. 모델 거부 시 모델/계정 접근권
오류로 안내하고 기본 모델로 자동 전환하지 않는다. 응답의 모델 표시는 요청한 선택을
나타내며 응답 `reasoningEffort`도 요청 override의 echo이다. 서버가 실제로 해석한 숨은
모델 버전이나 내부 추론 실행량을 증명하지 않는다.

Codex의 read-only sandbox·규칙/사용자 설정 격리와 Claude의 tools/hooks/MCP 격리,
기존 앱 기능 dispatcher·전송 상한·동의·실행 직전 검증은 유지한다. 이 UI는 외부 MCP
클라이언트의 모델/추론 설정이나 계정/로그인, 파일 삭제 권한을 변경하지 않는다.

## 근거

- 코드: `assistant_provider.rs`, `assistantModelPreference.ts`, `useAssistantModelPreference.ts`, `AssistantModelPicker.tsx`, `AssistantModelSettingsPanel.tsx`, `AssistantView.tsx`.
- [검증 기록](../qa/2026-10-06-cli-model-selection.md)
- [CLI 업데이트·추론 검증 기록](../qa/2026-10-06-cli-reasoning-selection.md)
- [Codex CLI 공식 명령](https://learn.chatgpt.com/docs/developer-commands?surface=cli)
- [Claude Code 모델 설정](https://code.claude.com/docs/en/model-config)
