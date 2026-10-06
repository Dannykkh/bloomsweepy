# CLI 모델 선택 — 현재 요청과 검증 기록

date: 2026-10-06
source: codex
scope: 현재 모델 선택 요청만. 사용자 턴 시각/session UUID 미제공. 날짜 전체 로그 복구·정제 아님.

## 현재 요청

사용자: “그리고 cli연결할때, 뭔가 그게 있어야 하지 않나? 모델선택? 옵션에서하거나, 아니면 지금이 코덱스앱에서 입력창에 있는 모델선택하는 ui 를 그대로 쓴다던지?”

결정: Codex 앱의 비공개 UI를 가져오는 것이 아니라 익숙한 모델 선택 동작을 기존 BroomSweepy
입력창·설정에 맞춘다. 같은 Picker와 공급자별 공유 선호를 사용하며 CLI 실행에도 반영한다.
권한·로그인·CLI 전역 설정은 바꾸지 않는다. 계정 접근권은 목록 제공과 구분한다.

## 구현·관찰 — 2026-10-06 오전 검증

- Codex/Claude 선택 모델 또는 null 기본값, Ollama 설치 모델; Grok/Antigravity는 현재 연동 기본값 전용.
- 신규 선호 단위8개, 전체 프런트74개, ARM64 native225개/3ignored, TypeScript PASS.
- 합성 화면: 선택 모델 실제 request 값, CLI 기본 null, 공급자 분리, 설정↔채팅, reload,
  목록 실패/stale선택, 내장 목록 출처, Ollama 설치 모델, busy 설정 새로고침 복구 확인.
- 390×844에서 입력과44px 모델 선택이 보이고 가로 overflow 없음. Pretendard 로드·14px·키보드2px focus 확인.
- 기존 native-shaped file review 확인 생략 합성 회귀: 명시 요청1회→mock 실행1회. 실제 파일 이동 없음.
- 설정 busy 스냅샷이 refresh 자체를 잠그던 P2를 리뷰로 찾고 loading만 refresh를 막도록 수정.
- 첫 설치형 모델 목록이 비어 있던 실측: Codex0.153.4 live catalog354470B/list4/hide3,
  bundled517840B/list6/hide5. parser의 show_ui 전용 허용이 원인. list/show_ui 두 형식을 명시 허용하고
  hide/unknown은 계속 제외하도록 수정, 재검증 진행. 환경·인증·JSON 파싱 문제 아님.

## 잠금 해제 후 설치형 검증 — 2026-10-06 10:16–10:19 KST

사용자: “풀었어. 계속 진행하자.”

- CUA로 기존 앱 정상 종료 후 최종 visibility/44px 수정 ARM64 앱 설치.
  설치 binary SHA256277c38d… 일치, deep/strict 서명 검증 PASS. 데이터 폴더 무교체.
- 기존 Claude Code/sonnet 복원 후 Codex로 변경. 실제 공개 모델4개를 메뉴에서 확인.
- GPT-5.6-Luna 선택 후 파일 작업 금지 단문 질문2회에 실제 답변 완료:
  “모델 연결 확인 완료”, “17 곱하기 19는 323입니다.”
- Settings 공유, ⌘Q 완전 종료/main process 부재 후 재실행 Codex/Luna·자체 대화12개 복원.
- Remember·추가 확인 생략 등 기존 권한 유지, 보존29B 파일2개 그대로. 새 삭제 없음.
- 원본 앱과 초기 모델 앱은 recoverable backup에 보존. 최신 앱은 AI 화면으로 열어 둔다.
- 응답 모델 라벨은 요청 echo다. argv 실측은 선행 login status만 포착했으므로 exec 모델 인자를
  직접 포착했다고 보고하지 않는다. 별도 인자 전달은 기존 소스/argv 단위 검사 근거다.

## 후속 요청 — 추론 강도와 CLI 업데이트

사용자: “그런데, 추론강도는 안나오네? 그리고 최신이6까지 나왔는데5.6이네? …”
이어서 “수정 다 하자. 업데이트 하자”.

현재 선택5.6-Luna는 이전 연결 테스트에서 직접 선택한 값이며 앱에 하드코딩된 기본값이
아님을 설명했다. 공식 Codex CLI를0.153.4→0.160.1로 업데이트하고 실제 공개7개와
모델별 지원/default를 확인했다. 기존 model preference를 유지하면서 채팅/설정 공용
추론 선택과 provider/model별 저장·요청·CLI override를 구현했다. CLI 전역 설정과
삭제 권한은 유지한다. 자동frontend85/native229 및 독립 리뷰·합성 UI 완료, 실제 새
설치형 검증은 [추론 QA](../docs/qa/2026-10-06-cli-reasoning-selection.md)에 별도 기록한다.

새 ARM64 앱 설치·서명/해시 검사 PASS, 기존5.6-Luna/대화14개 복원 후 UI에서6.1-Sol+
중간 명시 선택. 실제exec 인자의 model/effort 및 격리 플래그를 직접 확인했고 연결 문구
응답·대화16개·설정 공유·권한 보존 PASS. 마지막 screenshot/완전 재실행 전에 Mac이
자동 잠금되어 UI가 중단, 사용자 잠금 해제를 요청했다. 아직 재시작 복원 PASS로 보고하지 않는다.

## 경계

합성 QA는 실제 CLI 응답 검증이 아니다. 계정별 전체 모델·Windows·장시간 메모리 검증은 별도다.
이번 요청의 커밋·푸시·공개 릴리스는 하지 않는다. 사용자 자료·보존 테스트 파일·기존 권한·로그인 설정을 삭제/변경하지 않는다.

#tags: 모델선택, cli연동, 공급자분리, 카탈로그, 설정동기화, arch:012
