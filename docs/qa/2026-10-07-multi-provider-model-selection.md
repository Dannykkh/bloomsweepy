# 다중 CLI 모델·추론 선택 QA

2026-10-07 KST · macOS Apple Silicon · 현재 개발본, 공개 v1.7.0과 구분.
아래 PASS는 명시한 관찰에만 적용하며 실제 생성·Windows 런타임·계정 접근권에 확대하지 않는다.

## 요청과 구현 계약

사용자 요청은 Codex 참조의 compact composer, 입력창 아래 설명 제거,
Claude Code·Grok·Antigravity(agy)의 최신 이용 가능 모델/추론 선택, Settings 공통 설정이다.

- 정상 입력부/팝업은 현재 선택과 조작만 표시한다. 오류·미지원·stale 복구 안내는 유지한다.
- 모델 목록은 실제 설치 CLI에서 조회한다. 목록 갱신으로 저장된 모델을 자동 변경하지 않는다.
- 모델별 지원 증거와 CLI 옵션을 함께 확인하고 임의 Ultra/기본 강도/미존재 variant를 만들지 않는다.
- 채팅의 compact controls와 Settings의 기존 Picker는 동일한 공급자·모델별 선호를 사용한다.
- 기본값은 인자 override 생략이다. 모델/강도 응답 표시는 요청값 echo이며 실제 서버 적용의 증명이 아니다.
- 기존 진행/취소·격리 실행·MCP·파일 검토/삭제 동의·권한 정책은 변경하지 않는다.

정본 연결: [Experience Contract](../design-refs/2026-10-07-experience-compact-composer.md),
[CLI 모델 선택 구조](../architecture/assistant-model-selection.md).

## 현재 확인 완료

### 합성 compact UI — 메인의 선행 관찰

자체 assistant-tools fixture만 사용했다. 실제 CLI 질문·개인 파일 조회·파일 작업은 없다.
다음 PASS는 공급자 확장 전 compact 컨트롤의 관찰이며 이후 통합 검증을 대신하지 않는다.

| 검사 | 상태 | 관찰/한계 |
|---|---|---|
| Range End/Home·모델 목록 키보드·Enter | PASS | 실제 지원 단계 이동, picker 조작으로 질문 오발송0 |
| Escape·외부 닫기 | PASS | popup 닫힘·trigger focus 복귀·초안 보존 |
| 모델별 선호·기본값 reset·reload | PASS | 모델 전환 시 해당 선호 복원, null override와 명시 강도 구분 |
| Busy·취소 | PASS | 모델/강도 선택 비활성화, 취소 후 복구 |
| 목록 실패·stale·구형 host | PASS | 저장값 보존·명시 복구·미지원 값 전송 차단 |
| 760×600·390×844 | PASS | 가로 overflow 없음; 최소 창 popup clipping 수정 후 재확인 |
| 정상 설명 제거 | PASS | 1280×820 fixture에서 아래 안내와 정상 popup 설명 제거 확인 |

관찰 이미지: [compact composer](../design-refs/2026-10-07-compact-composer/desktop.jpg).
이는 합성 화면 증거이며 설치본·실제 모델 응답·계정 접근권 증거가 아니다.

### 실제 Claude CLI — 질문 없는 metadata 조회

설치된 Claude Code를2.1.147에서2.1.291로 업데이트했다. 같은 Mac의 실제 CLI에
공식 SDK initialize control request만 보내고 모델 metadata 응답을 확인했다.
user message0·assistant message0·result message0이다. LLM 질문/생성이나 파일 작업은 하지 않았다.
인증·계정 raw 출력은 UI/이 문서에 저장하지 않았다.

| CLI 응답의 모델 ID | 추론 metadata | 표시 계약 |
|---|---|---|
| `default` | low·medium·high·xhigh·max | 앱의 CLI 기본값과 중복 모델 행을 만들지 않음 |
| `opus` | low·medium·high·xhigh·max | 실제 metadata의 지원 단계만 사용 |
| `claude-fable-5[1m]` | low·medium·high·xhigh·max | 정확한 context suffix ID 보존; 일반 임의 괄호 허용과 구분 |
| `sonnet` | low·medium·high·xhigh·max | 실제 metadata의 지원 단계만 사용 |
| `haiku` | effort metadata 없음 | 단계 추정/추가하지 않음 |

Metadata-only 조회: PASS. 기본 추론 강도 필드는 확인되지 않았으므로 하드코딩하지 않는다.
앱 Rust의 실제 metadata-only probe/parser opt-in 검사도 PASS(1개,1.45초).
첫 실행은 로그인 상태와 metadata 가용성을 혼동한 검사 전제 때문에 실패했다.
로그인되지 않은 상태에서도 capability checks와 질문 없는 metadata는 가능하므로 분리 후 통과했다.
이 결과는 실제 생성·모든 계정 모델 접근권·서버 강도 적용을 의미하지 않는다.
alias가 서버에서 해결한 실제 모델 버전도 이 응답만으로 확정하지 않는다.

## 공급자별 연동 근거와 실행 미확인

| 공급자 | 목록/명시 선택 계약 | 추론 선택 계약 | 이 Mac의 실제 CLI 검증 |
|---|---|---|---|
| Codex | 기존 CLI catalog·별도 `--model` | 기존 모델 metadata·`model_reasoning_effort` override | 설치본 catalog·GPT-6.1-Sol/중간·Settings 공유 PASS; 이번 생성 NOT RUN |
| Claude Code | initialize metadata, 실패 시 공식 aliases·`--model` | metadata와 설치 CLI `--effort` 교차 확인 | metadata-only·설치본 모델/강도·Settings/재시작 PASS; 생성 NOT RUN |
| Grok | `grok models` 실제 행·`--model` | CLI effort 옵션 확인 후 정확한 모델별 확인 단계 | CLI 미설치: 실제 목록/실행 NOT RUN |
| Antigravity(agy) | `agy models` 실제 slug/label·`--model` | 같은 base의 실제 variant 행과 `--effort` | CLI 미설치: 실제 목록/실행 NOT RUN |
| Ollama | 기존 `ollama list` required 계약 | 이번 변경에서 강도 없음 | 새 생성 검사 NOT RUN |

Grok의 확인된 정확한 모델 계약은 `grok-4.7`/`grok-4.6`에서
low·medium·high·xhigh, `grok-4.5`에서 low·medium·high다. 실제 목록에 있는 ID만 표시하며
그 외 모델의 지원 강도는 추정하지 않는다. CLI 목록 자체에 없는 모델을 공식 문서만 보고 추가하지 않는다.

Agy는 실제 목록의 slug suffix와 표시명 `(Low)/(Medium)/(High)`가 일치하는 variant를
같은 base로 묶는다. 확인된 variant만 강도로 표시하고 누락된 medium 등을 생성하지 않는다.
공식 IDE 모델 페이지를 CLI의 계정별 목록으로 대신하지 않으며, 미확인 JSON schema도 가정하지 않는다.

## 최종 통합 검사

- `npm run check`, `npm run test:all`:108/108 PASS. `npm run build` PASS.
- `cargo test -p bloomsweepy-desktop --lib --release --target aarch64-apple-darwin`:
  264 PASS/4 opt-in ignored. 새14 parser tests 및 help·argv·실행 args·strict suffix·auth배너 회귀 포함.
- 출력 cap의 합성1MiB 생성 검사가 병렬 부하에서1초 timeout과 경쟁했다. 목적은 scheduling
  속도가 아닌 크기 상한이므로 검사 예산만5초로 바꾸고 전체 재실행 PASS. 제품8초/1MiB cap은 유지.
- CUA의 `?multi-provider` fixture:Claude fable[1m]/xhigh, Grok4.7/xhigh,
  Agy Pro-high/high 선택값이 실제 mock request에 정확히 전달됨을 관찰. 실제 AI 전송0/파일 실행0.
- Grok4.5에는 xhigh가 없고 Agy Pro에는 medium이 없는 것을 UI에서 확인. provider 전환 시
  이전 provider의 override가 섞이지 않으며 Settings의 low→high 변경과 reload 복원 PASS.
- 독립 리뷰P2 수정: model-selection unsupported + 저장 ID + effort override 없음에서도
  ID/경고/전송 차단 유지. Settings에서 명시 기본값 reset 후 입력/전송 복구, 자동 질문0 PASS.
- 최종 다중 provider760×600/390×844에서 popup clamping/44px controls/가로 overflow PASS.
  실제 폰트 loaded/check·computed 대비·렌더 비평은 [critique](../design-refs/2026-10-07-critique-compact-composer.md).
- `git diff --check`, Experience Contract validator PASS. 새 dependency 없음.

## 설치형과 미실행 범위

| 검사 | 현재 상태 | 완료 기준 |
|---|---|---|
| Native catalog parser·help gate·argv·suffix 회귀 | PASS | 위264개 전체 검사 및 실제 Claude opt-in 구분 |
| 공급자 확장 후 frontend 전체 검사 | PASS | 위108개 및 build/typecheck |
| Claude/Grok/Agy 합성 UI·Settings 동기화 | PASS | 위CUA 실제 mock request/공유/복원 관찰 |
| 최종 ARM64 build·설치본·서명·재시작 | PASS | 빌드/설치 해시 일치·deep strict 서명·정상 종료/재실행·실제 UI/설정 보존 확인 |
| Grok/Agy 실제 CLI metadata·생성 | NOT RUN | 이 Mac에 해당 CLI 설치/설정 필요; 설치되지 않은 동작을 PASS로 보고하지 않음 |
| Claude 실제 LLM 생성 | NOT RUN | 이번 metadata-only 검사와 별개이며 자동 비용 발생 검사를 하지 않음 |
| Windows 런타임·계정별 전체 모델 | NOT RUN | 해당 환경/계정 검증 필요 |
| 장시간 메모리·FPS·OS 스크린리더 | NOT RUN | 짧은 UI/metadata 검사와 별도 |

현재 작업의 검증에서는 실제 파일 삭제·휴지통 비우기·새 로그인·파일 권한 확대가 없다.
Claude CLI 업데이트가 변경한 설치 방식 정보와 기존 npm 잔여 설치는 메인의 설치 기록에서 구분한다.

### 실제 설치본 관찰 — 2026-10-07 01:29 KST

ARM64 단일 job/non-incremental `.app` 빌드 exit0, ad-hoc 서명과 deep strict 검증 PASS.
빌드와 `/Applications/BroomSweepy.app`의 실행파일 SHA-256은 동일하다:
`104b9988a2fa0585a8b4f3837e18515e5e8653c46c8851efe20b55d7ba76583d`.
이전 앱은 `/private/tmp/broomsweepy-compact-models-backup-WUvlLK/BroomSweepy.app`에 보존했다.
공개 릴리스/버전 변경이 아닌 로컬 개발본1.7.0 교체다.

- Claude 실제 catalog의 Opus/Fable/Sonnet/Haiku4행과 기본값 행을 팝업에서 확인했다.
  Opus 기본 강도 미확인 상태에는 지원5버튼, 매우 높음 선택 후에는 실제5단계 range가 표시된다.
  Settings에 Opus/매우 높음이 반영되고, Settings 기본값 reset이 채팅에 복원된다.
- Codex는 CLI 준비됨, GPT-6.1-Sol/중간으로 Settings→채팅 공유·range 표시 PASS.
  실제 질문은 보내지 않았다. 원래 Claude/opus/빈 effort 선호로 되돌렸다.
- ⌘Q 후 앱 프로세스 부재(exit1)를 확인한 뒤 같은 설치 경로에서 다시 실행했다.
  기존 대화11개, Claude/Opus/CLI 기본값, 기억 유지·추가 확인 생략1·시스템 조회1·검색off·정리 검토on이 보존됐다.
- 현재 Claude CLI는 로그인 필요다. metadata 조회 성공과 계정 로그인을 혼동하지 않는다.
  최신 native 설치 방식으로 업데이트됐으며 기존 npm 잔여 설치는 삭제하지 않았다.
- 읽기 전용 최종 독립 리뷰에서 이번 변경의 확정적 blocker 없음. 기존 Oct6 MCP 변경은 범위 제외.

실제 화면 증거: [Claude 추론 팝업](../design-refs/2026-10-07-compact-composer/installed.jpg),
[Codex 추론 팝업](../design-refs/2026-10-07-compact-composer/installed-codex.jpg).
이 이미지는 합성 fixture가 아닌 실제 설치본이며, 질문 응답·서버 적용 증거는 아니다.

## 후속: Claude 최신 버전 라벨 — 2026-10-07

사용자 후속 요청은 Opus/Fable/Sonnet 이름만이 아니라 최신 버전까지 식별하는 것이다.
누락 원인은 initialize가 제공한 `resolvedModel`을 무시하고 짧은 `displayName`만 사용한 데 있다.
canonical wire ID의 제한된 family/숫자 버전만 라벨로 추출한다. snapshot 날짜·임의 description은 버전으로 사용하지 않는다.
모델·추론 popup과 Settings는 같은 label을 표시하며 Settings tooltip도 label+실행ID를 제공한다.
실행/선호 ID와 과거 대화의 모델 ID는 변경하지 않는다.

같은 metadata-only 초기화에 startup `--model fable`을 넣으면 아래 실제 최신 alias 행도 받는다.
추가 조회·질문·set_model control·전역 모델 설정 변경은 없다. 기존8초/1MiB/격리 상한을 유지한다.

| 원래 선택 ID | 실제 CLI resolvedModel | 표시 라벨 |
|---|---|---|
| opus | claude-opus-5-5 | Opus 5.5 |
| sonnet | claude-sonnet-5-5 | Sonnet 5.5 |
| haiku | claude-haiku-4-5-20251001 | Haiku 4.5 |
| claude-fable-5[1m] | claude-fable-5 | Fable 5 (1M) |
| fable | claude-fable-5-1 | Fable 5.1 |

기존 Fable5 pinned 선택을5.1로 오표시하거나 바꾸지 않는다. 최신 행은 실제 CLI 응답에서 별도로 확인했다.
옵션에 공개된 모델과 계정 접근권·실제 생성 모델은 여전히 별개다.

- 프론트 check와108tests PASS. 현재 UI 라벨 전달/선호/ID 기본 계약 유지.
- Rust267tests PASS/4ignored. 새3개 회귀는 실제 resolved 버전/ID·추론 불변,
  잘못된 ID/description 비추론, 미래 버전을 고정 최신 매핑 없이 표시하는 계약이다.
- 실제 Claude opt-in1 PASS(2.22초): fable 행·숫자 버전 라벨, user/assistant/result0 확인.
- 프론트 생산 build PASS, JS945.21kB. 기존 chunk warning 유지.
- ARM64 최종 재빌드 exit0(3분53초), deep strict 서명·설치/빌드 해시 일치 PASS:
  `f029c0d70fb39e2d1f0576eb3014ed68196acb8ade35a4b0c9499f319a668800`.
  이전 compact 앱은 `/private/tmp/broomsweepy-model-versions-backup-zFacMw/BroomSweepy.app`에 보존했다.
- 실제 설치본의 모델 popup에 위5버전이 표시됨을 확인했다. Fable5.1 선택→실제ID fable PASS.
  [실제 모델 버전 목록](../design-refs/2026-10-07-compact-composer/installed-claude-versions.jpg).
  이 화면은 실제 설치본이며 합성 fixture/LLM 생성 응답이 아니다.
- Settings도 Fable5.1/tooltip `Fable 5.1 · fable`로 공유됨을 확인했다.
  테스트 전의 Claude/opus/default 선호를 복원하고 현재 사용 중이던 Codex/Sol/중간으로 되돌렸다.
  새 설치 실행 뒤 대화11개와 기억·확인생략1·시스템조회1·검색off·정리검토on 보존 PASS.
  실제 질문/파일 실행/새 로그인0. Claude 로그인 필요, Grok/Agy 실제 CLI 미설치 한계는 그대로다.

## 후속: 빈 Grok/Agy 목록 — 2026-10-07 약03:05 KST

사용자 “안티나 그록은 아예 사용할 모델리스트도 없네”를 실제 설치 상태와 대조했다.
PATH와 Homebrew/npm/Volta/사용자 CLI 후보에 `agy`·`grok` 실행파일이 없고,
공식 사용자 설치 위치 및 관련 Applications 후보도 없었다. 이 Mac에서는 CLI 미설치가 원인이다.
Grok 공식 `~/.grok/bin` fallback은 native 후보에서 누락되어 있어 macOS/Linux에 추가했다.
기존 후보 우선순위·절대경로 검증·중복 제거·Windows 경로는 그대로다.

모델 팝업과 Settings는 공용 presentation helper로 설치/고장/로그인/서비스/설치 모델 없음/
목록 조회 실패를 구분한다. 실제 목록이 없을 때만 공식 문서의 읽기 전용 예시(Grok 2개,
Agy 7개)와 설치 안내를 표시한다. 실제 목록·선호·추론 지원 단계·요청 argv에는 합치지 않는다.
계정 사용 가능 여부/전체 목록을 주장하지 않고 정상 목록을 받으면 예시를 숨긴다.
signed-out CLI의 실제 metadata와 optional CLI의 준비된 기본값 실행은 유지한다.

- Frontend `check`/`test:all`:112 PASS. 추가4검사는 미설치 placeholder, 원인 구분,
  signed-out metadata/unsupported 계약, 예시와 실제 모델·강도 분리다.
- Native release ARM lib:269 PASS/4 opt-in ignored. 추가2검사는 PATH 없는 공식 Grok 위치
  탐색 및 절대경로/중복/우선순위다. 기존 Claude 실제 probe는 이번 후속에서 재실행하지 않았다.
- CUA 합성 `?multi-provider&provider-state=notInstalled`:두 공급자의 전송/실제 선택 차단,
  stale ID 경고와 명시 reset, 설치 안내·별도 예시, Settings 공유 PASS. 실제 AI/파일 작업0.
- 760×600 및390×844:팝업 clipping/가로 overflow 없음, Agy7행 bounded scroll·설치 링크
  keyboard 접근 PASS. 예시 link focus 2px, 링크44px, 실제 폰트 로드 확인. primary link의
  초기 대비4.46:1을 기존 text token17.41:1로 수정했다. 새 전역 모션/의존성 없음.
- 정상 `?multi-provider`로 복귀한 Agy fixture에서 실제 모델5개+기본값 선택지6개,
  참고 section0을 관찰했다. Mock queries/executions/preparations0 유지.
- 최종 읽기 전용 독립 리뷰 blocker 없음. `cargo fmt --all -- --check`/`git diff --check` PASS.
- ARM `.app` build exit0(3분45초), frontend JS954.59kB/CSS196.59kB. 기존 chunk warning 유지.
  새 bundle의 ad-hoc deep strict 서명 PASS. 실행파일 SHA-256:
  `5fca6b1a6ec4ea86784f9928df61a85e00d798ab5654652c29b805056adbf5d1`.
- 이 단계 당시 설치 교체/실제 새 UI·native 공식 링크/재시작은 **NOT RUN — Mac 잠금**.
  `/Applications/BroomSweepy.app`는 이전 f029c0d7… 그대로이며 실제 프로세스도 실행 중이다.
  사용자에게 잠금 해제 및 두 CLI 설치 여부를 비동기로 요청했으나 아직 응답이 없다.
  강제 종료·CLI 설치·로그인·AI 질문·파일 실행·권한 변경·commit/push는 하지 않았다.

관찰 증거는 합성 QA 화면이다: [Grok 미설치 안내](../design-refs/2026-10-07-compact-composer/grok-uninstalled.jpg),
[Agy 미설치 안내](../design-refs/2026-10-07-compact-composer/antigravity-uninstalled.jpg).
위의 이전 설치 PASS와 이번 새 bundle의 빌드 PASS를 설치형 동작 PASS로 혼동하지 않는다.
실제 CLI `models`/생성/계정 접근권과 Windows 런타임은 여전히 NOT RUN이다.

## 후속: 사용자 요청에 따른 설치 교체 — 2026-10-07 KST

사용자 “교체하자” 후 Mac 잠금이 풀린 것을 실제 CUA 화면으로 확인했다.
설치 대상은 위에서 빌드한 새 ARM bundle이며 CLI 설치/로그인은 이번 요청에 포함하지 않는다.

- 기존 앱에서 Codex/GPT-6.1-Sol/중간과 권한 기억·추가 확인 생략1·시스템 조회1·검색off·
  정리 검토on·자동 시작on·메뉴 막대 메모리on·Docker off를 확인했다. 기존 대화는11개였다.
- CUA ⌘Q 정상 종료 뒤 정확한 앱 프로세스 부재(exit1)를 확인했다. 실행 중 덮어쓰지 않았다.
- 기존 앱을 `/private/tmp/broomsweepy-empty-catalog-backup-8Q7RnR/BroomSweepy.app`로
  이동해 보존했다. 백업의 실행파일은 이전 f029c0d7…와 일치하며 삭제하지 않았다.
- 새 bundle을 `/Applications/BroomSweepy.app`에 복사했다. 설치본 deep strict 서명 PASS,
  빌드/설치 실행파일 SHA-256 일치 PASS:
  `5fca6b1a6ec4ea86784f9928df61a85e00d798ab5654652c29b805056adbf5d1`.
  독립 정적 검토에서 앱/MCP/document-worker 모두 arm64, 이 Mac 실행 blocker 없음.
  서명은 ad-hoc이며 개발자 공증·외부 배포 검증이 아니다. 버전 표시는 개발본1.7.0을 유지한다.
- CUA로 새 설치 경로에서 실행했고 실제 앱 창을 확인했다. Settings에서 Grok2개/Agy7개
  읽기 전용 예시·공식 링크·CLI 설치 필요·모델/추론 select 비활성화를 확인했다.
  Agy 예시는 bounded scroll로 표시되며 실제 선택 옵션으로 합쳐지지 않는다.
- 실제 Grok 채팅의 모델 popup에서 같은 예시2개·설치 안내·질문 입력/전송 비활성화 PASS.
  popup의 공식 안내 클릭이 native opener를 거쳐 Safari의 정확한
  `https://docs.x.ai/build/overview`를 여는 것을 확인했다. 이 검증 탭만 닫고 기존 Safari 탭은 보존했다.
- 기존 대화11개·위 권한/자동 시작/메뉴 막대/Docker 설정 보존 PASS. 검증 중 공급자만 바꿨고
  모델/강도/권한·CLI 전역 설정은 변경하지 않았다. 마지막에 Codex/GPT-6.1-Sol/중간으로
  복원했고, 실제 채팅의 같은11개와 입력 준비 상태를 다시 확인한 채 앱을 열어 두었다.
- 설치 후 실제 화면은 [Grok Settings](../design-refs/2026-10-07-compact-composer/installed-grok-empty.jpg),
  [Agy Settings](../design-refs/2026-10-07-compact-composer/installed-antigravity-empty.jpg).
  이는 합성 QA가 아닌 `/Applications`의 새 설치본이다.

이로써 위의 Mac 잠금으로 미뤄진 앱 교체/설치형 UI 확인은 해소했다. 새 소스 변경은 없어
앞선112frontend/269native 검사를 재실행하지 않았다. 실제 새 AI 질문·파일 실행·삭제0,
새 로그인·권한 변경·Grok/Agy CLI 설치0, commit/push/release0이다.
Grok/Agy 실제 모델 목록·생성·계정 접근권, Claude 실제 생성, Windows·장시간 검사는 계속 NOT RUN이다.

## 공식 근거

- [Claude Code 모델 설정](https://code.claude.com/docs/en/model-config)
- [Claude Code CLI reference](https://code.claude.com/docs/en/cli-reference)
- [Claude 모델·버전 목록](https://platform.claude.com/docs/en/models/overview)
- [Claude Agent SDK ModelInfo 타입](https://unpkg.com/@anthropic-ai/claude-agent-sdk@0.3.291/sdk.d.ts)
- [Grok CLI reference](https://docs.x.ai/build/cli/reference)
- [Grok 공식 설치 안내](https://docs.x.ai/build/overview)
- [Grok 공식 모델 설정 예시](https://docs.x.ai/build/settings)
- [Grok headless scripting](https://docs.x.ai/build/cli/headless-scripting)
- [Grok reasoning 모델 계약](https://docs.x.ai/developers/model-capabilities/text/reasoning)
- [Grok remote model config source](https://github.com/xai-org/grok-build/blob/main/crates/codegen/xai-grok-shell/src/agent/remote_config/manager/mod.rs)
- [Grok ACP model metadata source](https://github.com/xai-org/grok-build/blob/main/crates/codegen/xai-grok-pager/src/acp/model_state.rs)
- [Antigravity CLI headless](https://antigravity.google/docs/cli/headless)
- [Antigravity 공식 CLI 설치](https://antigravity.google/docs/cli/install/)
- [Antigravity CLI reference](https://antigravity.google/docs/cli/reference/)
- [Antigravity CLI changelog](https://antigravity.google/docs/changelog)

#tags: multi-provider, cli모델, 추론강도, compact-composer, 검증범위, arch:012
