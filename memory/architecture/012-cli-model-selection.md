# 공급자별 CLI 모델·추론 선택과 공용 선호

status: CURRENT
date: 2026-10-07
source: codex
tags: cli-model-selection, reasoning-effort, provider-preference, local-storage, model-catalog
evidence: conversations/2026-10-06-cli-model-selection.md#현재-요청; conversations/2026-10-06-installed-safety-verification.md; conversations/2026-10-07-compact-multi-provider-composer.md#현재-요청 (사용자 턴 시각·session UUID 미제공; 현재 요청만 기록)
alternatives: 설정 전용 — 대화 중 변경 불편으로 제외; 화면별 선호 — 동기화/공급자 ID 혼용 위험으로 제외; 고정 Codex 모델 이름/desktop cache — 설치 CLI와 버전·목록 불일치 때문에 제외; 계정 접근 실패 시 기본 모델 전환 — 사용자의 모델·비용 의도를 바꾸므로 제외. 공급자가 검증된 catalog/선택 API를 바꾸면 재검토.
depends-on: [[004-app-tool-investigation]], [[008-conversation-workbench]]
sources: docs/architecture/assistant-model-selection.md; docs/qa/2026-10-06-cli-model-selection.md; docs/qa/2026-10-06-cli-reasoning-selection.md; docs/qa/2026-10-06-installed-trash-safety.md; https://learn.chatgpt.com/docs/developer-commands?surface=cli; https://learn.chatgpt.com/docs/developer-settings; https://code.claude.com/docs/en/model-config
files: apps/desktop/src-tauri/src/assistant_provider.rs; apps/desktop/src-tauri/src/assistant_model_catalog.rs; apps/desktop/src-tauri/src/external_program.rs; apps/desktop/src/lib/assistantModelPreference.ts; apps/desktop/src/lib/assistantComposerOptions.ts; apps/desktop/src/hooks/useAssistantModelPreference.ts; apps/desktop/src/components/AssistantComposerControls.tsx; apps/desktop/src/components/AssistantModelPicker.tsx; apps/desktop/src/components/AssistantModelSettingsPanel.tsx; apps/desktop/src/views/AssistantView.tsx; apps/desktop/src/views/SettingsView.tsx
supersedes: none
reopen-when: CLI catalog/args 형식 변경, 계정별 catalog 제공 방식 변경, 공식 참조 문서의 selector/버전 예시 변경, 다른 provider 추론 선택 지원 요청, 실제 공급자별 선호 혼용/자동 fallback 또는 높은 강도 timeout 실측 발생.
last_verified: 2026-10-07 (빈Grok/Agy 상태/참고 목록:112frontend/269native·4ignored·합성760·390/실제catalog복귀 PASS. 사용자 교체 요청 후 정상종료/기존앱백업/ARM5fca6b1a… 설치·deep strict·해시일치·실제실행 PASS. 실제Settings의Grok2/Agy7 읽기전용 예시·선택비활성화, Grok채팅popup/질문차단/native공식링크 PASS. 기존대화11개·권한/자동시작/트레이설정보존, 원래CodexSol/중간 복원. 두CLI 미설치/실제생성·새로그인·파일실행0/Windows NOT RUN. 단계별 근거는 QA 및 아래 historical/followup 기록)
historical_verification: 2026-10-06 (이전 모델-only0.153.4/Luna 응답·재시작 PASS. 후속CLI0.160.1/공개7개·metadata 실측, TSC·85frontend·229native/3ignored·합성 render·독립 review PASS. 당시9b799c1f… 설치본6.1-Sol+medium 실제 argv/응답·Settings공유 PASS, 오후재시작은 Mac잠금으로 당시NOT RUN. 후속guard ARM64설치본08faa1ac…/deep strict PASS, ⌘Q/process부재→재실행 뒤 chat/Settings6.1-Sol/중간·자체취소대화4개·기존권한 복원PASS로 미완료 재시작 해소. 새 argv probe는 요청 완료 후라 포착못함. 라벨/강도는 서버 resolved version 독립 증거 아님)
followup_verified: 2026-10-07 01:57 KST (resolvedModel 버전 라벨·동일initialize --model fable 최신행 조회,108frontend/267native·4ignored/metadata opt-in1 PASS. ARM f029c0d7…/deep strict·설치해시일치,실제Opus5.5/Fable5.1/Sonnet5.5/Haiku4.5·기존Fable5(1M) 구분/Settings tooltip·IDfable PASS. 원래선호/당시CodexSol·중간/대화11개·권한보존,질문·파일실행0. 위104b9988은 앞선 compact phase, f029c0d7은버전라벨phase 증거이며 후속교체현재5fca6b1a…)

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

2026-10-06 당시 Grok/Antigravity는 앱 연동 기본값 전용이었으며 영구 한계로 정의하지 않았다.
모델 선택은 기존 Codex/Claude 실행 격리와 앱 검사·공개·확인 생략 opt-in을 확대하지 않는다.

## 2026-10-07 사용자 범위 확대

정상 설명을 제거한 컴팩트 composer와 Settings 기존 Picker는 같은 선호를 공유한다.
실제 지원 단계만 native range로 배치하며 default가 미확인일 때 지원 버튼으로 선택을 시작한다.
새 timer/Canvas/blur/의존성 없이 정적 브랜드 track 한 곳만 사용자 참조 예외로 허용한다.

Claude control initialize(질문 없음)에서 실 모델/effort를 읽고 CLI help의 광고된 choices와
교차 확인한다. 구형/실패는 aliases fallback이며 추론을 추정하지 않는다. 기본 강도는 actual
metadata가 없으면 null이다. [1m] 단일 suffix만 frontend/native ID 예외로 허용한다.
후속 버전 표시: CLI `resolvedModel`의 검증된 canonical family/숫자 버전을 라벨로 사용한다.
metadata-only startup --model fable로 현재 alias 행을 함께 받아 기존 pinned Fable5와5.1을 구분한다.
질문·추가 probe·전역 선택 변경은 없고 실행/저장 ID는 그대로다. 임의 description이나 고정 최신
매핑으로 버전을 만들지 않으며 오늘의 alias 버전을 과거 대화에 소급하지 않는다.
Grok은 실제 models 목록과 정확한 확인 ID별 단계만, Agy는 실제 same-base variant만
표시한다. 기본값은 인자 생략, 명시는 --model/--effort를 모든 조사 라운드에 전달한다.
Codex의 --config 경로는 그대로다. Grok 빈 tools의 None 동작 때문에 --deny '*'를 필수
광고/실행 인자로 추가한다. metadata와 계정 접근권·실제 서버 강도는 별개다.

설치본/구형 host가 모델 선택을 지원하지 않는데 저장 ID가 있으면 표시·경고·전송 차단을
유지하고 사용자 명시 reset으로만 default를 보낸다. 원래 빈 default는 계속 허용한다.
alternatives(추가): 모든 공급자에 enum8개 — CLI/model 계약 위반으로 제외; 고정 최신 모델
나열 — 실제 설치/계정 목록과 불일치로 제외; Grok ACP 확장 — 현재 작업에는 검증된
metadata-only 응답 계약이 없어 known exact matrix로 제한, 안정된 schema 실측 시 복귀.
sources(추가): docs/qa/2026-10-07-multi-provider-model-selection.md; docs/design-refs/2026-10-07-experience-compact-composer.md; https://github.com/anthropics/claude-agent-sdk-python/blob/main/src/claude_agent_sdk/_internal/query.py; https://docs.x.ai/build/cli/reference; https://www.antigravity.google/docs/cli/headless/

## 2026-10-07 빈 카탈로그와 참고 목록의 경계

목록 없음과 CLI 미설치/호환성/인증/서비스 상태를 구분한다. 이 Mac의 Grok/Agy CLI는
실제로 미설치이며 desktop 앱 설치를 CLI 설치로 간주하지 않는다. Grok의 공식 사용자
설치 디렉터리 `~/.grok/bin`을 macOS/Linux fallback에 포함하고 기존 후보 우선순위를 보존한다.

빈 Grok/Agy 목록에 공식 문서의 읽기 전용 모델 예시와 설치 링크를 별도로 제공한다.
실제 `provider.models`와 선택 옵션·선호·추론 단계·실행 인자에는 합치지 않으며 정상 목록이
하나라도 있으면 예시를 숨긴다. 최신/전체/계정 접근권을 보장하지 않고 설치·계정 미확인을 표시한다.
이는 고정 최신 목록을 실행 catalog로 대신하는 방식과 다르며 기존 실제 목록 정본 결정은 유지한다.
정상 채팅 아래 반복 설명을 복원하지 않고 예외 상태의 팝업/Settings에만 안내한다.
로그인되지 않은 건강한 CLI가 주는 실제 metadata와 준비된 optional 기본값 실행은 유지한다.

alternatives(빈 목록): 아무 목록 없이 기본값만 표시 — 미설치를 정상 기본값으로 오해해 제외;
공식 예시를 실제 선택 목록에 합치기 — 설치/계정/추론 지원을 보장하지 않아 제외;
CLI 자동 설치/로그인 — 별도 설치 의사를 받지 않았고 계정 설정을 넘겨짚을 수 없어 제외.
evidence(후속): conversations/2026-10-07-compact-multi-provider-composer.md#후속-grokagy-빈-목록 (약03:05 KST 관찰; 원본 턴 시각·session UUID 미제공).
sources(참고): https://docs.x.ai/build/overview; https://docs.x.ai/build/settings; https://antigravity.google/docs/cli/install/; https://antigravity.google/docs/cli/headless/
reference_last_verified: 2026-10-07 (공식 예시Grok2/Agy7·CLI 미설치 확인.112frontend/269native·4ignored/합성760·390/정상catalog복귀 PASS. ARM새bundle5fca6b1a…/deep strict PASS;Mac잠금으로 새 설치·실제 UI/재시작 NOT RUN, 이전f029c0d7… 유지. 실제CLI생성·설치·로그인0)

install_followup: 2026-10-07 (사용자 교체 요청 후 Mac잠금 해제, 기존f029c0d7… backup-8Q7RnR 보존, 현재/Applications 새5fca6b1a… 설치·실제UI 확인으로 위 NOT RUN 해소. 실제CLI 설치/계정로그인/생성 검증으로 확대하지 않는다)
evidence(설치): conversations/2026-10-07-compact-multi-provider-composer.md#새-설치본-교체; docs/qa/2026-10-07-multi-provider-model-selection.md#후속-사용자-요청에-따른-설치-교체--2026-10-07-kst
