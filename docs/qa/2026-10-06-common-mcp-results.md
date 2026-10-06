# 공통 파일·MCP·결과 왕복 검증

2026-10-06 · source: codex · 사용자 “보완하자”. 기준 소스9e9d775 이후 개발본.

## 실제 발견과 변경

이 Codex 채팅의 기존 설치 MCP 연결/디스크/CPU·RAM/앱 조회는 실제 응답했다.
app_capabilities는16KiB 제한에 걸려 계약 전체가itemsOmitted로 내려왔다.
내장 JSON과 외부 MCP는 같은 앱 엔진의 입구지만 일반 파일 작업은 내장 전용이었다.

정본에서 작은 전체 discovery index와 ID별 상세를 파생하며 전송 상한은 유지한다.
파일10행동은 typed 요청/dispatcher를 공유하고 기존 native envelope는 alias로 유지한다.
외부 작업은 승인 루트/epoch·독립 workspace·작업 번호·상태/실제 결과로 연결한다.
대기/실패 뒤에는 분석 전용 응답 한 번을 허용하고 모델 오류도 앱 근거/검토를 보존한다.
최종 실행은 main 앱 일회용 확인이며 모델/MCP의 execute/approve/권한 변경은 없다.

## 소스·합성 검증

- 공통 계약/MCP lib: 28+15개 PASS. 정본24ID·파일10행동·각 상세와 discovery의16KiB 상한 회귀 포함.
- native lib: 246개 PASS, 3개 opt-in ignored. 공유 파일 코어·native/external 격리·권한epoch/ABA·실제 결과 projection·scripted CLI terminal 왕복·완료 이벤트 작업 종류 회귀 포함.
- frontend: TypeScript PASS, 96개 PASS, production build PASS. 기존 500KiB JS chunk 안내는 남음.
- 독립 리뷰 후 수정: 빈 폴더 조회 실패도 앞선 결과를 보존, native/common alias 반복 차단, production과 같은 prompt builder 시험, 오래된 외부 모달의 상태 불변 취소, I/O worker lease 유지.
- 실제 browser fixture: 아니오 초기 포커스/Escape 취소(조회·재준비·실행0/취소1), 부분 실패 요청1/이동0·항목별 실패, 대상 변경 오류·재시도 금지·닫기, malformed 검토 실행 버튼 없음, 최소760×600에서 대상/예·아니오/취소 접근 가능 확인. 실제 파일 이동 없는 합성 UI다.
- 합계385개 PASS(43+246+96). Rust fmt·diff check PASS.
- Clippy: 새 코드 collapsible_if3건 수정 후 PASS. all-targets의 기존 permission_settings.rs items_after_test_module 경고만 명시적으로 제외(`-D warnings -A clippy::items_after_test_module`). 무예외 strict 통과로 보고하지 않는다.
- 개인 자료 삭제, 휴지통 비우기, 앱 제거, Windows, 장시간 메모리: NOT RUN.

## 실제 설치·왕복 검증

ARM .app 빌드 및 이 맥의 /Applications/BroomSweepy.app 개발 설치를 실행했다.
Tauri bundle 첫 서명 검사는 실패했으며 기존 entitlements로 ad-hoc 재서명 후
`codesign --verify --deep --strict` PASS. 공개 서명·notarization이나 GitHub 릴리스 갱신은 아니다.
기존 설치본은 /private/tmp/broomsweepy-common-mcp-backup-TsW1zC/BroomSweepy.app에 보존했다.

- 새로 시작한 설치형 MCP stdio: initialize → tools/list12 → discovery24기능 → files.workspace 상세10행동 PASS. 실제 JSON15707B/11012B, 16KiB 미만이며 기능 누락·truncated 없음.
- opt-in harness [installed-common-flow.mjs](../../apps/bloomsweepy-mcp/tests/installed-common-flow.mjs): 정확한 승인 테스트 루트를 먼저 검증한 뒤 largest의 running/작업ID/완료/status·실제3파일87B 및 review_named의 review_required/deleted:false를 확인. 모델 결과에 전체 루트·로컬 plan ID 없음. 실행·승인 요청은 없다.
- 메인 앱 UI에서 외부 검토 대상 conditional-cancel.txt29B와 예/아니오 표시 확인. 아니오로 취소 후 workspace/status는 cancelled·movedCount0·selectedCount0·reviewPrepared:false. 취소를 harness가 자동 검증했다고 주장하지 않는다.
- 이전 외부 검토창을 열린 채 새 검사를 완료한 뒤 이전 창의 아니오를 눌러도 새 조회 결과(inspected)는 보존됨. 오래된 모달 취소의 상태 불변 계약 실측.
- 실제 내장 Codex(GPT-6.1-Sol/중간): “이 폴더에서 가장 용량이 큰 항목은 뭐야? 앱으로 새로 검사해서 이름과 실제 용량을 비교해줘. 삭제나 삭제 검토는 하지 마.” → 세 파일 각각29B/합계87B·읽기 오류 없음·삭제/검토하지 않았다는 응답과 앱 결과 카드 확인. 합성 CLI가 아닌 실제 모델 응답이다.
- 최신 native 큰 항목 결과에서 같은 결과 용량지도 →87B·직계3·하위 파일3·빈 폴더0·29B세 타일 확인. 외부의 후속 검사로 저장 맵 generation이 교체되면 이전 native 지도는 만료 안내 후 재검사가 필요하다.
- Remember 권한·정확한 테스트 루트·GPT-6.1-Sol/중간·저장 대화 복원 확인. 세 테스트 파일 각29B와 action-journal8079B는 유지됐다. 실제 이동은 실행하지 않았다.

실제 MCP 검사는 파일 작업 완료 이벤트가 저장공간 검사 실패 알림으로 잘못 분류되는
오류도 발견했다. control-scan-completed에 실제 작업 kind를 넣고 legacy storageScan도
같은 factory를 사용하도록 수정했다. 6종류×3종료상태 회귀 PASS.
해당 마지막 수정도 ARM host 재빌드·설치·strict 서명 검사 PASS.
최종 설치본에서 다시 MCP largest3파일87B → 검토 → 아니오 → cancelled/movedCount0을
실측했고 대시보드에 잘못된 검사 실패 알림이 없었다. 재실행 후7개 저장 메시지와
GPT-6.1-Sol/중간도 복원됐다. 실제 모델 질문 왕복은 직전 개발 설치본에서 확인했으며
최종 변경은 완료 이벤트 kind뿐이다. 마지막 설치 후 모델 질문을 다시 전송하지는 않았다.

최종 설치 SHA256:

- host: e8eb908f347a7ff7f6ea52d68928fd139d876a4a34c2c761d365837ae575c798
- MCP helper: b17bcbfe7cd4de9b851399afa8306dfabb63caa2e6ac75deaed325cd075c58d0

기존에 실행 중인 MCP sidecar는 옛 DTO를 유지할 수 있다. 기존 사용자 연결은 강제 종료하지
않았고, 위 실측은 새 설치 helper를 새 stdio 프로세스로 시작했다. 외부 클라이언트에서는
연결을 재시작한 뒤 새 기능을 사용해야 한다.

600초는 모델 조사 예산이며 진행 중인 앱 파일 검사에 대한 hard wall-clock 제한이 아니다.
외부 상태 관찰 최대4회는 모델 행동 계약이지 새 서버 rate-limit 구현이 아니다.
취소/닫기는 현재 계획과 정확히 일치할 때만 그 계획을 변경하며 오래된 유효 ID는 새 상태를 변경하지 않는다.

이 문서는 커밋/푸시/공개 릴리스 또는 모든 기능의 설치형 성공을 주장하지 않는다.

#tags: 공통계약, mcp, json, 상태응답, 설치형검증, arch:004, arch:013
