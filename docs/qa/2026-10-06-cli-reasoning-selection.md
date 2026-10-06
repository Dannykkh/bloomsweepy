# CLI 업데이트·모델별 추론 강도 QA

2026-10-06 KST · macOS Apple Silicon8GiB · 개발본, 공개v1.7.0 다운로드와 구분.

## CLI 업데이트와 목록 실측

공식 `codex update`로 기존 standalone0.153.4→0.160.1을 업데이트했다. 기존 로그인/CLI
전역 설정/모델 선호는 변경하지 않았다. 업데이트가 기존 앱의5.6-Luna 선택을 자동으로
최신 모델로 바꾸지 않는다. CLI `debug models` 결과는 ID/label/default/support만 투영해
확인하며 인증·cache·모델 지침·사용자 대화는 출력/저장하지 않는다.

| 실제 공개 모델 | 기본 강도 | 지원 강도 |
|----------------|-----------|-----------|
| GPT-6.1-Sol | low | low, medium, high, xhigh, max, ultra |
| GPT-6-Astra | medium | low, medium, high, xhigh, max, ultra |
| GPT-6-Sol | medium | low, medium, high, xhigh, max, ultra |
| GPT-6-Luna | medium | low, medium, high, xhigh, max |
| GPT-5.6-Sol | low | low, medium, high, xhigh, max, ultra |
| GPT-5.6-Terra | medium | low, medium, high, xhigh, max, ultra |
| GPT-5.6-Luna | medium | low, medium, high, xhigh, max |

이는 이 CLI/계정에서 관찰한 목록이며 모든 계정의 사용권이나 숨은 서버 모델 버전을 보장하지 않는다.
공식 [CLI update/debug 명령](https://learn.chatgpt.com/docs/developer-commands?surface=cli)과
[일회 실행 config override](https://learn.chatgpt.com/docs/developer-settings)를 확인했다.

## 자동 검사와 독립 리뷰

- frontend `npm run check` PASS, `npm run test:all`85PASS(선호19개, 추론 회귀11개 신규).
- native provider ARM64 release -j1:35PASS/2ignored.
- 전체 `cargo test -p bloomsweepy-desktop --lib --release --target aarch64-apple-darwin -j 1`:
  229PASS/3ignored. opt-in CLI 진단·live계약·nativeTrash는 이 검사에서 미실행.
- native tests: 구형 request 기본값, enum/공급자/모델 검증, metadata bounds/default,
  별도 argv·격리 보존, 모델/강도 거부 시 fallback/raw output 노출 없음.
- frontend tests: 구형v1/레거시, 모델별·공급자별 강도, 기본값/null, stale/catalog 실패,
  저장 실패,16개 상한에서 현재 선택 보존, invalid enum 검증.
- 독립 읽기 전용 소스 리뷰: actionable finding 없음. 리뷰 결과는 런타임 검증과 구분한다.
- rustfmt·git diff 검사 PASS. ARM64 app-only production build PASS(3m43s).
  기존928.79kB JS 청크 경고는 남음, 새 의존성 없음.

## 합성 UI — 실제 AI·파일 작업 없음

CUA로 자체 Vite1420/assistant-tools-fixture를 검사했다.

| 시나리오 | 실제 관찰 |
|----------|-----------|
| Fast+medium 질문 | mock request fixture-fast/medium, 합성 응답 완료 |
| Fast→Deep→Fast | Deep은 기본medium·max까지만, Fast의medium 복원 |
| Settings→chat→reload | 변경max 공유·재로드 복원 |
| CLI 기본 모델 | 강도 disabled/default, request model·effort null |
| Claude 전환 | 기존sonnet 복원, 강도 기본값만 disabled; Codex 강도 혼용 없음 |
| 목록 실패 | 저장한max 유지·warning·전송 차단, 명시 default reset 후 질문 가능 |
| 지원 축소 | 저장ultra를 유지하되 stale option/warning·전송 차단 |
| busy chat/settings | 두 선택기 disabled, 설정 refresh enabled→다음 status로 복구 |
| old host | 저장 강도 경고·전송 차단→default reset 후 두 picker disabled/질문 가능 |

390×844 리플로우 후 composer와 두44px 선택기가 보이고 scrollWidth380/clientWidth380으로
가로 overflow 없음.14px·Pretendard 실제 로드 확인. 키보드 Tab이 추론 선택기로 이동하고
2px outline이 나타남. 고정 다크/기존 reduced-motion 규칙 유지, 새로운 모션/blur/자산 없음.
로컬 ignored 증거: docs/ui-audit/screenshots/2026-10-06/reasoning-synthetic-390.png.
한/영/일/중 옵션·기본값 라벨 전환을 실제 화면에서 확인했고 일본어/중국어 글리프가
정상 렌더링되었다. fixture 표시 언어는 한국어로 복원했다.

## 범위 한정 UI 감사

frontend-design의 기존 Agent Workbench/effect budget0·Web Interface Guidelines를 적용했다.
이 점수는 변경 선택기 범위의 렌더 리뷰이며 앱 전체 인증이나 계측 수치가 아니다.

| 영역 | 점수/10 | 근거·한계 |
|------|---------|-----------|
| 테마 |9| 기존 고정dark/material·글자/option 대비 유지 |
| 반응형 |9|390/desktop reflow, composer·두44px 선택기 유지 |
| 접근성 |8| label/name/description/stale alert·keyboard2px outline; OS스크린리더 NOT RUN |
| 로딩/성능 |8| bounded catalog/storage·busy refresh 복구; 장시간/FPS NOT RUN |
| 폼 UX |9| 모델별 선호·default·stale 보존/명시 reset |
| 내비게이션 |9| 기존 Settings/chat 위치·공유선호 |
| 타이포/간격 |9|14px·Pretendard 로드·4locale·좁은 화면 wrap |
| 애니메이션 |10| 새 모션 없음·기존 reduced-motion 유지 |
| AI Slop |9| 필요한 모델+추론 두 선택기, 새 장식/효과 없음 |

가중8.85/10(B). 저장 실패/권한은 별도 자동 검사와 native 결과로 구분한다.

## 설치형 검증

14:13–14:16 KST, 새 ARM64 번들을 /Applications에 설치했다. 기존 앱은 CUA ⌘Q 정상 종료와
main process 부재 후 `/private/tmp/broomsweepy-reasoning-update-backup-Tej4i8/BroomSweepy.app`
으로 보존했다. 데이터 폴더는 교체하지 않았다.

초기 Tauri bundle은 linker ad-hoc signature만 있어 deep/strict resource 검사가 실패했다.
설치 전에 프로젝트의 기존 entitlements로 app bundle을 다시 ad-hoc 서명한 뒤 검사 PASS.
최종 bundle/설치 실행파일 SHA256 일치:
`9b799c1f7f47f49283fe5b364e8bde8d2e93c21145baa3c05676467da4c94a15`.
ARM64·설치본 deep/strict PASS. Apple 공증·공개 릴리스로 보고하지 않는다.

- 첫 시작에서 기존 Codex/5.6-Luna·기본중간·자체 대화14개 복원 PASS. 자동 최신 전환 없음.
- 실제 모델 메뉴의 공개7개와6.1-Sol default낮음 표시 확인. UI에서6.1-Sol/중간 명시 선택.
- 파일 조회·검사·삭제를 금지한 자체 연결 질문에 실제 “추론 설정 연결 확인 완료” 응답.
  경고/저장 실패 없이 대화16개·입력창/선택기 재활성화.
- app 자식 `codex exec`를 제한된90초 관찰에서 직접 포착했다. raw argv/질문/경로는 출력하지
  않고 model=gpt-6.1-sol, effort=medium, readOnly/ignoreUserConfig/ignoreRules/neverApproval
  모두true만 투영했다. UI 요청 echo와 별도로 실제 전달 인자를 확인한 증거다.
- Settings도 Codex/6.1-Sol/중간으로 공유 PASS. 처음 metadata 조회 중의 일시적 경고는
  상태 완료 후 사라지고 정상 선택이 복원되었다.
- Remember·확인 생략·시스템/앱 조회·정리 검토 ON, 자동 시작·메뉴 사용량 ON, Docker OFF 유지.
- 보존 파일 consent-cancel.txt/keep-this.txt 각29B 유지. 이번 테스트에서 파일 작업 없음.
- 마지막 설정 screenshot/⌘Q 재실행 검사 도중 Mac이 자동 잠금되어 CUA가 중단했다.
  새 설치 앱은 계속 실행 중이며, 잠금 해제를 요청했다. 추론 선택의 새 빌드 재시작 복원은
  아직 NOT RUN — 자동/합성 저장 검사와 실제 새 설치 시작 복원을 이것의 증거로 확대하지 않는다.

로컬 ignored 증거: reasoning-native-model-catalog.png, reasoning-native-response.png
(docs/ui-audit/screenshots/2026-10-06/). 잠금 이후 screenshot은 생성하지 못했다.

## 남은 경계

Windows 런타임, 다른 공급자의 실제 응답, 계정별 전체 모델/강도 조합, 장시간 RAM/FPS,
OS스크린리더는 NOT RUN. 높은 강도도 기존 한 라운드120초 제한을 따르며 장시간 완료를
보장하지 않는다. 응답 model/reasoningEffort는 요청 echo다. 파일 삭제/휴지통 비우기/로그인/
권한/CLI 전역 설정 변경, commit/push/release는 이번 요청에서 하지 않는다.

#tags: 추론강도, cli업데이트, 카탈로그, 맥설치, ui검증, arch:012
