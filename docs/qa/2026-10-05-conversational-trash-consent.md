# QA — 채팅 휴지통 이동의 한 번 확인과 선택적 확인 생략

## 요구와 경계

채팅에서 검토 팝업·복수 체크·최종 버튼을 반복하는 대신 정확한 대상의 인라인
예/아니오 한 번을 사용한다. 늦게 답해도 시간만으로 만료되지 않는다. 설정에서
명시적으로 확인 생략을 허용하면 원래 사용자의 정확한 이름을 포함한 제거 명령을
앱이 재검증 후 실행한다. 기본 OFF, Session/Remember 수명은 기존 권한 저장소와 같다.

모델·MCP에는 실행/자동 승인 도구를 주지 않는다. 상담·조건부·모호한 이름·다른 경로·
복수 계획·불완전 검색과 앱 관련 데이터·프로세스 종료·Docker·영구 휴지통 비우기는
확인 생략 대상이 아니다. 빈 폴더 종류 전체를 정리하는 요청도 한 번의 질문을 유지한다.
정리 후보 트리/용량지도/시스템 정리의 별도 수명과 확인은 바꾸지 않는다.

## 자동 검사

- `npm run check`: PASS.
- `npm run test:all`: 63 PASS. 직접 인간 응답, 정확한 이름, 상담/부정/조건/대상 치환,
  단일 대기 계획, 네 locale placeholder 일치를 포함한다.
- `cargo test -p bloomsweepy-desktop --lib --release -j1`: 219 PASS, 3 ignored.
  시간이 없는 일회용 계획·선택/재검사/신원/inventory/kind 변경·동시 소비·변경 대상
  차단과 확인 생략 권한의 기본 OFF/Remember 왕복/Session 복원 안 함/철회/구형 JSON
  기본값을 확인했다. ignored 실제 공급자/OS Trash 검사는 이번 작업에서 실행하지 않았다.
- Experience Contract validator: PASS.
- ARM64 app-only production build: PASS. 기존 약906kB main JS 경고는 남아 있다.

## 실제 렌더 / 합성 IPC

실제 production 컴포넌트와 `assistant-tools-fixture.html`의 합성 어댑터를 사용했다.
실제 사용자 파일 이동·권한 활성화·LLM 호출은 없다. Mock 실행 횟수는 OS 이동 증거가 아니다.

- 기본 OFF + 오래된 `expiresAtUnixMs:1` 앱 계획: 재검토 모달/2개 체크 없이 대상 경로와
  앱 본체만 범위를 표시하고 예/아니오를 제공한다. 시간 경과 오류 없음.
- `아니오`: 실행0, provider query 추가 없음. 다시 준비한 계획에 `예`: 실행1,
  query1/prepare0 유지, 대기 카드 제거와 실제 합성 결과 표시.
- 파일/내용 있는 폴더 계획: 하위 포함 안내와 한 번 질문, 별도 체크 없음.
  직접 `예` 입력으로 query1/실행1. 더 이상의 공급자 응답을 기다리지 않는다.
- 확인 생략 ON: `promo-video 삭제해도 돼?`는 실행0; `promo-video 삭제해줘`는 실행1.
  `Synthetic Editor 앱 삭제하자`도 중간 질문 없이 실행1/prepare0/query1.
- 실행 전 대상 변경과 권한 철회: 각각 실행0, 명확한 실패 문구, 자동 재시도 없음.
- 합성 Settings: Remember를 골라도 새 권한 기본 OFF. 체크하면 ON 표시, 기존 수명 선택 유지.
- 1280×720 기본 화면: 인라인 대상·범위·버튼, 하단 composer 함께 확인.
- 390×844: 페이지 scrollWidth380px, 가로 overflow 없음; 긍정 버튼의 긴 문구가 줄바꿈,
  composer가 보인다. 키보드 Tab으로 긍정 버튼의 기존 두 겹 focus ring 확인.
  첫 렌더의 파일 버튼42px를 공통 질문 class로44px로 보완하고 두 버튼 모두44px 실측 확인.
- 기존 Pretendard/dark tokens 사용, 새 blur/애니메이션/의존성 없음. 앱은 고정 dark scheme이며
  light theme가 없다. 실제 OS reduced-motion 설정 변경: NOT RUN; 기존 감소 규칙 유지.

스크린샷은 `docs/ui-audit/screenshots/2026-10-05-chat-trash-question.png`에 합성 대상으로만
저장한 로컬 ignored 증거다. 공개 앱 기능·마케팅 캡처로 혼동하지 않는다.

## 설치형 확인

최종 ARM64 main/문서 worker/MCP sidecar 빌드와 ad-hoc deep/strict 서명 검증 PASS.
최신 frontend `index-Ce0Ou13X.js`와 새 native 모델 계약이 번들 안에 포함됨을 확인했다.
기존 앱을 완전히 종료하고 다음 되돌리기 경로에 보관한 뒤 `/Applications/BroomSweepy.app`를 교체했다:
`/private/tmp/broomsweepy-trash-consent-backup-KdjuMa/BroomSweepy.app`.
설치된 host SHA-256은 빌드 번들과 일치:
`5134cd0b4f7fbcccb5fa7608b10251f3bc7b060b1f7a2640b3ed0c96344bce16`.
버전은1.7.0 개발본이고 notarization/공개 릴리스 변경은 아니다.
실행된 새 앱의 Settings에서 Remember, 시스템·앱 조회ON, 정리 검토ON, 검색OFF,
자동 시작ON, 메뉴 메모리ON, DockerOFF, 한국어와 검사 기준 유지 확인. 새로운 확인
생략 권한은 OFF로 표시되며 활성화하지 않았다. 제목줄/버전/traffic lights 유지.
실제 설정 캡처는 로컬 ignored `docs/ui-audit/screenshots/2026-10-05-installed-trash-permission.png`.
앱은 해당 설정 화면에 열어 둔다. 자체 합성 QA 탭3개와 Vite는 종료했고 viewport를 reset했다.

## 남은 검증

새 권한을 실제 사용자 앱에서 켜거나 실제 사용자의 파일/앱을 지우지 않았다.
새 전체 흐름의 실제 LLM→native 삭제 종단, Windows 런타임, 장시간 자원 soak는 NOT RUN.
현재 변경은 개발본이며 공개 GitHub v1.7.0 파일을 교체하거나 커밋/푸시하지 않았다.

## 실제 Codex와 native 검증 — 2026-10-05 20:10 KST

판정: **전체 완료 불가 — 두 회귀 발견.** 이번 요청은 검증이므로 구현을 수정하지 않았다.

- 실제 `live_codex_file_tool_contract`: PASS. 일반 검사/이름 지정 폴더 검토/큰 항목
  삭제 상담의 세 요청이 정해진 조회 도구로 연결됨을 확인했다(30.21초).
- `native_assistant_file_trash_synthetic_only`: PASS. 새2항목16B의 실제 macOS Trash
  이동, unselected 파일/링크 원본 보존, journal 검증(0.58초). 모델/UI 종단은 아니다.
- 설치 앱에서 새168B/6파일 격리 폴더의 `consent-default-no.txt 삭제해줘`를 Codex에
  전달했다. 정확한28B 대상과 한 번 예/아니오 표시. 직접 인간 `아니오`는 추가 모델
  왕복 없이 계획을 취소했고 원본28B를 보존했다.
- **취소 상태 회귀(실제 재현):** 선택0/계획 제거 뒤 공통 `files.workspace` 카드에
  “최종 확인 대기”가 남는다. `manageFiles`는 `appToolResults`를 갱신하지 않는다.
- **확인 생략 회귀(실제 응답 구조와 소스/계산으로 확인):** native provider는 파일
  계획에 항상 `files.workspace:review_required` 래퍼를 함께 반환한다. UI는 정확한
  파일 명령이어도 `reviews.length===0`을 요구해 자동 실행이 불가능하다.
  grant=true/정확한 명령=true/실제래퍼1→automaticTrash=false. 합성 fixture의 래퍼
  누락은 true여서 이전 합성 PASS가 이 문제를 가렸다. 실제 ON UI 실행은 NOT RUN.
- action-time 권한 활성화 질문에 답이 없어 설정을 켜지 않았다. 실제 앱 자동 삭제,
  변경 대상 차단/ON 재시작 종단, Windows/장시간 검증은 여전히 NOT RUN.
- 자체168B fixture와 빈 임시 폴더만 제거하고 완전 종료/재시작. native는 없어진
  임시 scope를 복원하지 않았고 원래 권한 필드 전부 일치: Remember/조회ON/정리검토ON/
  확인생략OFF/검색범위없음. 사용자 자료/앱은 삭제하지 않았고 Trash는 비우지 않았다.
  OS Trash 테스트16B는 복원 가능, 고립된 테스트 대화는 증거로 남겼다.
- 실제 취소 화면: `docs/ui-audit/screenshots/2026-10-05-native-consent-verification.png`.
  설치 앱은 재시작 후 Settings에 열어 둔다. 이전219Rust/63frontend는 이전 턴 결과다.

## 두 회귀 수정 및 실제 ON 종단 — 2026-10-06 07:01 KST

판정: 발견한 두 회귀는 수정 및 이 맥의 실제 설치형 검증 완료. 전체 플랫폼/장시간 검증 완료를 뜻하지 않는다.

- 파일 자동 실행은 자체 native `files.workspace:review_required` 하나의 출처·검토 준비·
  freshScan·revision 일치를 확인한다. 나머지 결과는 completed여야 한다. 별도/복수 검토,
  불완전/다른 revision/권한 필요/상담 요청은 자동 실행하지 않는다.
- 취소·직접 실행·선택 변경은 해당 파일/빈 폴더 revision의 자체 대기 래퍼만 제거한다.
  취소는 대화에 기록하며, 관련 없는 memory 검토가 남는 합성 회귀도 확인했다.
- 실제 native 래퍼를 fixture에 추가했다. 이전의 래퍼 없는 합성 PASS는 실제 응답을
  충분히 반영하지 못한 검사였으며 위2026-10-05 결과는 역사적 기록으로 보존한다.
- `npm run check`: PASS. `npm run test:all`: **66 PASS**. 기존63에 native 응답 형태,
  다중/오래된/불완전 근거의 자동 실행 차단, 자체 래퍼만 정리하는3검사를 추가했다.
- production 컴포넌트 합성 UI: ON→질문 없이 실행1; OFF→아니오 실행0/추가 query0 및
  대기 카드 제거; 새 요청→예 실행1; file+memory 검토→자동 실행0, file취소 후 memory
  검토 보존. changed-target/permission-revoked는 실행0, 오류 표시, 자동 재시도 없음.
- `npm run tauri build -- --target aarch64-apple-darwin --bundles app`: PASS.
  ARM64 sidecar/문서 worker 및 production frontend `index-QsmxU81u.js` 포함.
  기존907.48kB main JS chunk 경고는 남는다. ad-hoc deep/strict signature PASS.
- 이전 앱을 `/private/tmp/broomsweepy-consent-fix-backup-63GsJb/BroomSweepy.app`에
  보관하고 `/Applications/BroomSweepy.app` 교체. host SHA-256은 번들과 일치:
  `b6cf38985c1058fdfc7454897f8a6e85d0a011d1d5e61e1dbdaa1791791fdce8`.
  버전1.7.0 개발본이며 notarization/공개 릴리스/commit/push는 하지 않았다.
- 확인 생략 Remember/ON을 유지한 채 새 격리 대화를 선택했다. 새 자체3파일87B뿐이며
  기존 개인 대화를 이 검증 요청에 포함하지 않았다.
- 실제 Codex에 `consent-cancel.txt를 휴지통으로 이동하기 전에 검토 목록만 보여줘.
  아직 실행하지 마.` 요청. 정확한29B 대상의 한 번 질문이 나왔다. 인간 `아니오`는
  추가 모델 왕복 없이 취소 메시지/계획 제거/자체 대기 래퍼 제거, 완료된 조회 근거 보존.
- 사용자가 `/private/tmp/broomsweepy-consent-fixed-e2e-qiarwz/consent-auto.txt`29B
  **이 테스트 파일만 허용**한다고 답한 뒤 `consent-auto.txt 삭제해줘`를 실제 Codex에 보냈다.
  추가 예/아니오 없이 재검증→요청1/이동1 결과와 정확한 대상 경로를 실제 설치 앱에서 확인.
  stat에서 auto파일 부재, `consent-cancel.txt`/`keep-this.txt` 각각29B 보존을 확인했다.
  휴지통은 비우지 않았고 사용자 자료/앱을 삭제하지 않았다.
- 실제 증거: `docs/ui-audit/screenshots/2026-10-06-native-cancel-fixed.png`,
  `2026-10-06-native-auto-trash-fixed.png`. 합성 증거:
  `2026-10-06-trash-consent-fix-synthetic.png`. 모두 로컬 ignored이며 공개 마케팅 화면이 아니다.
- Rust 소스는 이번 턴 변경하지 않았다. 이전219Rust/3 ignored 전체 suite 재실행은 NOT RUN;
  native ARM64 컴파일과 실제 Codex→native Trash 종단은 위처럼 별도 확인했다.
  Windows/장시간 메모리 soak/실제 내용 있는 폴더·앱 본체 자동 이동은 NOT RUN.
- 나머지 테스트2파일58B, 고립된 대화와 이전 앱 백업은 보존한다. 확인 생략ON을 유지하고
  앱은 성공 결과에 열어 둔다. 자체 QA browser와 Vite만 종료한다.
