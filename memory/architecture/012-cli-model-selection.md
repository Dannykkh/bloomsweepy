# 공급자별 CLI 모델·추론 선택과 공용 선호

status: CURRENT
date: 2026-10-06
source: codex
tags: cli-model-selection, reasoning-effort, provider-preference, local-storage, model-catalog
evidence: conversations/2026-10-06-cli-model-selection.md#현재-요청 (사용자 턴 시각·session UUID 미제공; 현재 요청만 기록)
alternatives: 설정 전용 — 대화 중 변경 불편으로 제외; 화면별 선호 — 동기화/공급자 ID 혼용 위험으로 제외; 고정 Codex 모델 이름/desktop cache — 설치 CLI와 버전·목록 불일치 때문에 제외; 계정 접근 실패 시 기본 모델 전환 — 사용자의 모델·비용 의도를 바꾸므로 제외. 공급자가 검증된 catalog/선택 API를 바꾸면 재검토.
depends-on: [[004-app-tool-investigation]], [[008-conversation-workbench]]
sources: docs/architecture/assistant-model-selection.md; docs/qa/2026-10-06-cli-model-selection.md; docs/qa/2026-10-06-cli-reasoning-selection.md; https://learn.chatgpt.com/docs/developer-commands?surface=cli; https://learn.chatgpt.com/docs/developer-settings; https://code.claude.com/docs/en/model-config
files: apps/desktop/src-tauri/src/assistant_provider.rs; apps/desktop/src/lib/assistantModelPreference.ts; apps/desktop/src/hooks/useAssistantModelPreference.ts; apps/desktop/src/components/AssistantModelPicker.tsx; apps/desktop/src/components/AssistantModelSettingsPanel.tsx; apps/desktop/src/views/AssistantView.tsx; apps/desktop/src/views/SettingsView.tsx
supersedes: none
reopen-when: CLI catalog/args 형식 변경, 계정별 catalog 제공 방식 변경, 다른 provider 추론 선택 지원 요청, 실제 공급자별 선호 혼용/자동 fallback 또는 높은 강도 timeout 실측 발생.
last_verified: 2026-10-06 (이전 모델-only0.153.4/Luna 응답·재시작 PASS. 후속CLI0.160.1/공개7개·metadata 실측, TSC·85frontend·229native/3ignored·합성 render·독립 review PASS. 새 ARM64 설치본9b799c1f…/deep strict PASS,6.1-Sol+medium 실제 exec argv/응답·Settings공유 확인. 후속 완전 재시작은 Mac잠금으로 NOT RUN. 응답 라벨/강도는 요청 echo이며 서버 resolved version 독립 증거 아님)

입력창 하단과 설정은 공용 Picker와 useSyncExternalStore 기반 공급자별 local preference를
사용한다. 현재 선택 provider와 안전 ID, optional provider/model별 강도만 저장하며 권한·계정·질문·계획은 넣지 않는다.
기존 provider/Ollama키는 읽기 migration, 새version1이 우선한다. 저장 실패는 현재 선택
유지와 경고다. optional 빈 선택은 null/모델 인자 생략, required Ollama는 설치 모델이다.

Codex/Claude CLI help의 --model 지원을 확인하고 별도 argv로 전달한다. Codex catalog는
상태 갱신에만5초→bundled3초, 각출력1MiB/최대64개 visible list|show_ui/표시 metadata만
반환한다. 첫 native 검증에서 실제CLI가 list를 반환하는데 show_ui만 허용해 목록이 비었음이
확인되어 두 형식만 명시 허용한다. hide/unknown은 제외한다. 계정 사용권은 목록과 별도이며
모델 오류/stale ID에 자동 대체하지 않는다. Claude 별칭의 실제 버전은 추정하지 않는다.

추론 선택 후속: CLI0.160.1 업데이트에서 공개7개와 지원/default metadata를 실측했다.
명시 Codex 모델만 지원 enum8개까지 표시하고 기본값/null은 override를 생략한다.
version1에 optional reasoningEfforts map을 확장하며16개/4KiB 상한에서 현재 선택을
보존한다. 모델/공급자 전환 시 그 모델 선호만 적용하고, stale/목록 실패는 선택을 보존한
경고와 전송 차단 후 명시 reset/reselection으로 복구한다. 모든 조사 라운드에 별도
--config model_reasoning_effort 인자를 사용하고 globalconfig/격리/권한은 그대로다.
매 요청마다 catalog를 조회하지 않는다. CLI가 최종 조합을 검증하며 오류 시 fallback하지
않는다. 높은 강도라도 기존 한 라운드120초 제한은 유지한다. 응답 강도는 요청 echo다.

Grok/Antigravity는 현재 앱 연동 기본값 전용이며 공급자 전체의 영구 한계를 뜻하지 않는다.
모델 선택은 기존 Codex/Claude 실행 격리와 앱 검사·공개·확인 생략 opt-in을 확대하지 않는다.
