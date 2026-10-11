# 채팅 응답·결과 타임라인 검증

2026-10-11 · source: codex · main a4759b1 이후 개발본.
사용자 요청: 답변이 오지 않고 이전 조회 카드가 하단에 계속 남는 문제 수정.

## 실제 관찰과 수정

설치본 v1.7.0의 마지막 사용자 질문 3개에 답변이 없었으며 응답 형식 오류를 표시했다.
메인 앱은 살아 있고 해당 앱의 진행 중 Codex subprocess는 없었다. 파싱 전 원문/로그가
삭제되는 기존 계약 때문에 잘못된 출력이 plain text인지 다른 JSON 오류인지는 확인하지 못했다.
소스의 plain-text 표시 지시와 JSON protocol 충돌을 없애고 message 내부의 표시 규칙으로 한정했다.

완전한 단일 JSON 코드펜스만 정규화하고 같은 strict allowlist를 적용한다. 형식 실패 시
원래 질문과 실제 앱 근거를 이용해 분석 전용 복구를1회만 시도한다. raw 출력은 재전송하지
않으며 총5모델호출·4행동·600초 모델 조사 예산을 늘리지 않는다. 지원을 확인한 Codex에만
null-action schema를 적용한다. 인증 실패/빈 출력/timeout/cancel은 이 자동 복구 대상이 아니다.
시작한 파일 query의 hard wall-clock 종료 보장은 기존과 마찬가지로 하지 않는다.

카드는 전체 turns 뒤가 아니라 해당 assistant turn 바로 뒤에 연결한다. 과거 근거와
현재 조작 권한을 분리하고 새 질문을 이전 검토의 승인/취소로 해석하지 않는다. 완료한
검토의 조작 UI는 제거하되 실제 처리 요약과 항목별 실패/부분 결과는 완료 위치에 남긴다.
오류도 저장된 질문 뒤 BroomSweepy 메시지로 남기며 명시적 retry는 원래 질문을 중복 저장하지
않는다. 취소·세션·권한/workspace revision 변경이나 새 질문 뒤에는 이전 retry를 실행하지 않는다.
수동 폴더 검사·검색·드라이브 탐색도 이전 retry를 무효화한다.
상세 근거16개·live 결과16개·화면160턴으로 제한하고 전체 payload를 메시지 DB에 저장하지 않는다.

## 검증 기록

- Rust 전체 desktop lib release/ARM64: PASS — 291 passed, 5 ignored, jobs1/cold cache.
- 실제 Codex 일반 질문/analysis schema 왕복: PASS — opt-in `live_codex_general_answer_and_analysis_schema_contract` 1 passed. 실제 CLI/모델 선택·일반 질문 strict envelope 및 null-action schema를 검증했다. 합성 빈 workspace이고 dispatcher/파일 작업은 실행하지 않았다.
- TypeScript/locale check: PASS. 전체 frontend tests: 132 passed. Production build: PASS(기존 500kB chunk 경고 유지).
- Synthetic production-component browser: PASS — 오류→명시 retry→성공, 사용자 질문 중복 없음(질문1/모델요청2/실행0). 새 질문 뒤 기존 카드가 이전 assistant turn 안에 남고 권한 부족 카드는 펼쳐진다.
- 범위 변경 회귀: 실패 뒤 수동 파일 검사 시 retry disabled, 원래 사용자 턴1/모델요청1/실행0 유지. 세션·권한·최신 질문·workspace revision guards는 helper tests와 독립 정적 리뷰로 확인했다.
- 합성 검토: 새 질문 뒤 과거 앱 삭제 검토에 실행 버튼 없음, 미확인 상태 보존, native cancel 호출0. 별도 현재 검토에서 아니오를 누르면 완료 위치에 취소 요약만 남고 예/아니오 버튼0/실행0.
- 합성 지연/취소: 대기 status와 경과 시간/취소 버튼 노출. 취소 후 오류 메시지, retry disabled, busy 해제, 실제 실행0. 첫 관찰은 10초 mock이 먼저 완료되어 취소하지 못했으므로 두 번째 요청을 실제로 취소해 판정했다.
- Rust fmt 및 diff whitespace: PASS.
- 독립 정적 리뷰: PASS — strict parser/복구/취소·epoch/turn anchor/payload 상한. 수동 workspace 변경 후 retry 재사용 finding을 수정하고 재검토했다. 남은 확정 P0/P1/P2 finding 없음.

## 변경 화면 UI 검증

전체 제품 감리가 아닌 수정한 채팅 영역의 검증이다. 1280×820/390×844 실제 렌더에서 가로
overflow 없음과 하단 입력창 노출을 확인했다. 지원하지 않는 light theme는 N/A이다.

| 영역 | 결과 |
|---|---|
| 테마·대비 | 기존 dark 토큰 유지, 본문과 오류 텍스트 식별; 새로운 팔레트 없음 |
| 반응형 | 위 두 viewport에서 카드가 흐름 안에 있고 입력창이 화면에 남음 |
| 접근성 | 오류 role=alert, retry 높이44px, Tab 이동 시 focus outline 노출 |
| 로딩·취소 | 실제 mock 지연 status/경과 시간과 취소 후 idle 복귀 확인 |
| 입력·재시도 | 사용자 질문 중복 없음, 최신 질문·권한·workspace 경계 무효화 |
| 내비게이션 | 메뉴 순서/공유 설정/라우팅 변경 없음 |
| 폰트 | 실제 loaded font `Pretendard Variable`, 한글 글리프 표시 |
| 모션 | 추가 모션 없음, 기존 reduced-motion 정책 변경 없음 |
| 브랜드·밀도 | Agent Workbench 유지, 완료 조회 접힘/실패·권한 부족 펼침 |

합성 화면: [desktop](screenshots/2026-10-11-chat-timeline-desktop.jpg),
[narrow](screenshots/2026-10-11-chat-timeline-narrow.jpg). QA 배너/카운터는 fixture 전용이며 배포 UI가 아니다.

## 설치와 한계

ARM64 release 앱 패키지 빌드 PASS. 번들 생성 후 서명 검증은 최초에 resource seal 오류로
실패했다. 기존과 같은 ad-hoc 정책과 기존 entitlements로 MCP→document-worker→app 순서로
다시 서명하고 `codesign --verify --deep --strict` PASS 후 설치했다. 공증 배포본이 아니며
이번 요청에서 버전/릴리스/commit/push는 변경하지 않았다(v1.7.0 개발 수정본).

2026-10-11 16:23 KST `/Applications/BroomSweepy.app` 교체·실행 PASS(PID52626).
이전 앱은 `/private/tmp/broomsweepy-chat-response-install-OCEqkm/Previous-BroomSweepy.app` 및
`Replaced-BroomSweepy.app`에 보존했다. 설치된 main SHA256은
`6311753af76b4d54142a5551f62c81c64ef086bf1ab7d1f85935afc05a44e48b`로 서명한 빌드와 일치한다.
기존 main SHA256은 `4711f90679c31889e84398f4ce5373b269eb70d6e4c18410758418488bd664e4`이다.
대화 DB와 권한 DB는 종료 전/후/재실행 후 SHA256이 동일했다. 실제 설치 UI에서 기존
43메시지·선택 폴더87B·전체 접근 권한·Codex 준비됨·GPT-6.1-Sol/중간 복원을 확인했다.
설치된 MCP `--version` 및 document-worker `--check-heap-budget`도 PASS.
실제 사용자 대화의 질문을 다시 보내거나 삭제 작업을 재연하지 않았다. 설치 UI smoke,
별도 실제 CLI 왕복, 합성 카드/오류 검증을 설치본 전체 종단 테스트로 혼동하지 않는다.

개인 파일 삭제·앱 제거·휴지통 비우기·권한 확대·Windows·장시간 soak는 NOT RUN.
실제 사용자 대화 화면은 개인 정보가 섞일 수 있어 Git에 캡처하지 않는다.
합성 fixture만 UI 검증용으로 사용하며 실제 LLM/파일 작업의 증거로 표현하지 않는다.

#tags: chat-response, evidence-timeline, explicit-retry, strict-json, regression, arch:008, arch:013
