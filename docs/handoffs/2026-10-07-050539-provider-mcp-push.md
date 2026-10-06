# 공통 MCP 결과·다중 공급자 입력창 커밋 인계

## Origin / Session Metadata

- Created: 2026-10-07 05:05:39 KST
- Project: BroomSweepy; branch main; pre-commit HEAD 9e9d775
- Source: codex
- Origin: 사용자 “핸드오프하고, 커밋 푸시”. 앞선 공통 MCP 보완, 입력창 개선, 모델/추론 선택, 설치 교체 결과를 함께 공유한다.
- Origin source: 현재 사용자 턴. 원본 세션 UUID/턴 도착 시각 미제공; 위 시각은 기록 시각이며 추정 세션 ID를 쓰지 않는다.
- Continues from: [입력창·설치 인계](2026-10-07-003212-compact-composer.md), [공통 MCP 인계](2026-10-06-200123-common-mcp-results.md). 실제 이어받은 구현 범위다.

## Current State Summary

코드·문서·기억·대화·공개 적합한 화면 증거를 커밋/푸시하는 요청이다. 기능 구현과 이 Mac의
로컬 개발 앱 교체는 완료했다. 버전은1.7.0이며 새 릴리스/태그/공증 요청은 아니다.

현재 `/Applications/BroomSweepy.app`의 실행파일 SHA-256은
`5fca6b1a6ec4ea86784f9928df61a85e00d798ab5654652c29b805056adbf5d1`.
이번 인계에서 다시 해시와 정확한 경로의 실행 프로세스를 확인했다. 앞선 설치 검증에서
기존 대화11개·권한·자동시작·트레이 설정을 보존하고 Codex/GPT-6.1-Sol/중간으로 복원했다.
이전 앱은 `/private/tmp/broomsweepy-empty-catalog-backup-8Q7RnR/BroomSweepy.app`에 보존되어 있다.

Git 준비 시점 HEAD/origin-main은9e9d775로 동일(좌우 차이0/0).
SSH 키 인증 실패 후 같은 저장소 HTTPS fetch와 push dry-run은 성공했다.
기존 origin 주소/글로벌 Git 인증 설정은 바꾸지 않았다. 이 문서는 커밋 직전 스냅샷이며,
최종 커밋 ID와 실제 원격 일치는 Git 기록 및 이 요청의 완료 응답에서 확인한다.

## Feature/Flow/Decision Snapshot

| 경계 | 입력 → 처리 → 저장 → 표시 |
|---|---|
| 내장 채팅/JSON | 인간 질문 → 제한된 모델 조사 → 공통 typed dispatcher/앱 결과 → 세션 결과·로컬 검토 → 결과 카드/모델의 분석 응답 |
| 외부 MCP | 기능 index/ID별 상세 → 선택 루트·epoch의 독립 workspace → 앱 작업 ID·상태 → 제한된 실제 목록/검토 상태 → 외부 클라이언트 분석 |
| 모델·추론 선택 | 실제 CLI catalog → 지원 모델/강도만 검증 → 공급자·모델별 공용 preference → 컴팩트 채팅 popup/Settings → 조사 전체 라운드의 명시 argv |

파일 검색·검사·용량 계산·검토/실행 검증은 앱이 맡고 모델은 조회 선택·분석을 맡는다.
내장 files envelope는 공통 요청의 호환 alias이며 외부는 내부 세션/임의 경로를 공유하지 않는다.
최종 실행/권한 설정을 모델·MCP에 노출하지 않는다. 대기/실패 뒤 분석 전용 한 라운드는
추가 행동을 하지 않고 실제 결과·준비된 검토를 보존한다. 기존4행동·48KiB 누적 상한을 유지한다.

정상 입력창 아래 설명을 제거하고 작은 모델/추론 trigger·지원 단계 slider·원형 전송을 사용한다.
Claude 버전은 실제 resolvedModel의 제한된 canonical metadata에서 식별하며 실행/저장 ID는 바꾸지 않는다.
Grok/Agy 빈 catalog에는 설치/로그인/서비스/조회 상태와 읽기 전용 공식 예시를 표시한다.
예시를 실제 선택·추론·선호·argv에 합치지 않으며 실제 목록이 있으면 숨긴다.
자동 fallback/가짜 최신 모델·지원 강도/CLI 자동 설치는 사용자의 선택·권한을 바꾸므로 제외했다.

구성도 정본: [앱 조사 흐름](../flow-diagrams/app-tool-investigation.mmd).
상세 경계·대안·근거: [공통 capability 계약](../architecture/app-capability-contract.md),
[모델 선택 계약](../architecture/assistant-model-selection.md), architecture004/011/012/013.

## Implemented Features / File Ownership

- 공통 Rust contract: `crates/bloomsweepy-control/src/capabilities.rs`, `apps/bloomsweepy-mcp/src/mcp.rs`.
- 앱 실행/공개 경계: `apps/desktop/src-tauri/src/{app_tools,assistant_files,assistant_provider,control_server}.rs`.
- 공급자 모델 계약/탐색: `assistant_model_catalog.rs`, `external_program.rs`; 새 dependency 없음.
- 채팅/설정: `AssistantComposerControls.*`, `AssistantModelPicker.*`, `AssistantView.*`, 공용 preference/helper와 i18n.
- 외부 검토/설치 검사: `externalFileReview.ts`, fixture 및 frontend tests, `apps/bloomsweepy-mcp/tests/installed-common-flow.mjs`.
- 정본: desktop README, DESIGN, architecture/QA/design refs, 현재 대화, 정제 기억·인덱스 및 앞선 핸드오프.
- 작업자 읽기 전용 코드·문서/이미지 검토 소유권 반환. 쓰기·stage·커밋·푸시·최종 판정은 메인 책임.

## Verification

완료한 검사를 새로 실행했다고 보고하지 않는다. 이번 요청은 코드 기능 변경 없이 검증 기록을 묶는 작업이다.

- 공통 MCP 단계: control/MCP28+15, native246·3ignored, frontend96/TypeScript/production build PASS.
  실제 설치 stdio12도구/24기능/파일10행동, 자체3파일87B 조회·검토·인간 취소(moved0), 실제 Codex 결과 분석·같은 용량지도 PASS.
  [공통 MCP QA](../qa/2026-10-06-common-mcp-results.md). 기존 permission_settings의 Clippy 경고1종을 명시 제외한 검사이며 무예외 strict PASS가 아니다.
- 최종 입력창/공급자 단계: frontend112/check/build, native269·4opt-in ignored, Rust fmt/diff PASS.
  Claude metadata-only opt-in1은 앞선 버전 라벨 단계에서 PASS; 실제 생성/계정 접근권 증거가 아니다.
  합성760/390px 키보드·stale·busy·기본값·설정/선호 복원·실제 catalog 복귀 PASS.
- ARM build5fca6b1a… 및 ad-hoc deep strict PASS. 실제 교체/실행·Grok2/Agy7 예시·질문 차단·native 공식 링크·대화/설정 보존 PASS.
  [다중 공급자 QA](../qa/2026-10-07-multi-provider-model-selection.md). 이미지의 합성/실제 설치 범위는 문서에 구분되어 있다.
- 이번 요청: `git diff --check` PASS; 독립 읽기 전용 코드 리뷰 확정 blocker 없음.
  민감값 패턴은 대상 텍스트에서 일치0. 패턴 검사만으로 개인정보 부재를 보장하지 않아 이미지도 독립 시각 점검했다.
- `demo-assets/`의 옛 Swift 전체 데스크톱 캡처5개는 개인 대화/로컬 경로 포함으로 커밋 제외·로컬 보존한다.
  공개 후보 design refs9개는 약1.76MiB이며 build bundle/자격증명/원시 JSONL은 포함하지 않는다.
- 문서 benchmark의 stale 진행 상태와 설치 JPEG2개의 잘못된 `.png` 확장자/참조만 정정했다. 이미지 픽셀은 변경하지 않았다.
- Grok/Agy 실제 CLI 모델 조회/생성, Claude 실제 생성, Windows 런타임, 장시간 저자원/FPS/OS 스크린리더: NOT RUN.
  이 Mac 두 CLI 미설치이며 앱 교체 승인을 별도 CLI 설치/로그인 승인으로 확대하지 않는다.

## Session Memory Review

- Root: 실제 `.git`, `docs/handoffs`, `memory`, `conversations`는 확인된 프로젝트 내부 디렉터리다. 쓰기는 절대경로를 사용했다.
- Architecture preflight: MEMORY/index에서004/011/012/013 실제 본문과 연결 근거 확인. 닥터 마지막 방문2026-09-29로30일 미도래이며 이번 재실행은 SKIPPED.
- 관련 기존 결정은 변경/대체하지 않았다. 입력창·MCP·설치 증거가012/013 및 현재 대화·QA에 연결됨을 확인했다.
- `build_anchor_index.py --file MEMORY.md`는 일치 의존 항목 없음으로 반환; 실제 기억 본문과 Git 공유006을 직접 확인했다. MEMORY는 마지막 업데이트 날짜만 갱신해4994B 상한을 유지한다.
- 원본 세션 UUID 미제공으로 신규 관찰/gotcha 전체 정제와 기존 backlog/offset 변경은 하지 않았다.
- Self-improvement candidate: entry012/013, reusable=yes, relation=기존 결정 보완, evidence=verified, candidate=defer, target=none.
  범위에 맞는 프로젝트 전용 스킬이 없고 같은 근거의 후보를 재생성하지 않는다. 전역 스킬/규칙은 변경하지 않는다.
- Component map N/A: `codemap/component-map.json` 없음. 자동 생성 코드맵은 수동 수정하지 않았다.
- Handoff validator READY: 필수 섹션·placeholder·민감값 검사 PASS. 목록형 구현을 non-feature로 분류하고, 명시적으로 부재인 component-map을 깨진 참조로 센다. `supersedes:none`/세션 UUID 미제공 경고도 남는다. 실제 결정 대체가 없으므로 가짜 slug/ID로 숨기지 않는다.

## Immediate Next Steps / Resume

1. 메인에서 공개 후보만 stage → staged diff/민감자료 제외 확인 → 커밋 → 같은 저장소 main으로 non-force HTTPS push → 원격 HEAD와 로컬 HEAD 일치 확인. 사용자 승인 범위다.
2. 푸시 후 다음 구현자는 Git 로그/이 인계 및 연결 QA에서 재개한다. Mac 앱 재빌드/재교체·이미 통과한 전체 테스트 반복은 필요 없다.
3. 실제 Grok/Agy 설치/로그인/서버 모델·강도, Claude 생성은 별도 사용자 선택과 환경으로 검증한다.
4. Windows·장시간8GiB/낮은 디스크 soak는 별도 환경에서 진행한다. 현재 PASS를 그 범위까지 확대하지 않는다.
5. 버전 변경/태그/GitHub Release는 이번 요청에 없으며 하지 않는다.

## Important Context / Environment / Safety

- 8GiB ARM Mac, 앞선 측정 디스크 약1.9GiB/100%; 이번 중복 Cargo/build 실행 없음. 기존 build/dev 세션은 종료됐다.
- 앱을 정상 실행 상태로 유지했다. 앱 데이터·권한·대화/사용자 파일 삭제나 OS 휴지통 비우기는 이번 작업에 없다.
- 원래 origin은 SSH이며 그대로 유지한다. HTTPS는 같은 `Dannykkh/bloomsweepy`의 명시 URL로 이 요청에서만 사용한다. force push/remote 재설정/글로벌 credential 변경 없음.
- 커밋에서 제외한 `demo-assets`는 삭제하지 않는다. 그 밖의 기존 사용자 변경도 그대로 보존하며 build/원시 상태는 기존 ignore를 따른다.
- 이후 UI를 이어가면 `cua.rewriteDocumentation()`을 먼저 읽는다. 쉘 UI 자동화/개인 대화 캡처를 하지 않는다.

#tags: 핸드오프, 커밋푸시, 공통mcp, 다중공급자, 설치검증, arch:004, arch:011, arch:012, arch:013
