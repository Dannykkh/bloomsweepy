# macOS 설치형 삭제 안전·모델 복원 검사

2026-10-06 18:56–19:09 KST · source: codex · Apple Silicon8GiB.
현재 사용자 “설치형 검사까지 완료하자”. 시작 소스 `9e9d775`, 릴리스/태그 생성 아님.

## 빌드·교체 신원

`CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 npm run tauri build -- --target aarch64-apple-darwin --bundles app`
PASS. 기존 ARM64 release 캐시를 재사용했고 native compile2m48s, frontend JS929.29kB의
기존 chunk 경고는 유지된다. MCP/document sidecar 버전 및 heap-budget 자가검사도 준비
스크립트를 통과했다. 새 source 수정은 없고 이전 frontend91/native229·3ignored를 다시
실행한 것으로 보고하지 않는다.

빌드 로그의 최종 `index-B-Wgh7wi.js`는 SHA256
`416226fc7e035638a27f3093f40bc3d6173728e326431f434416d8e9ab2331b6`.
tauri.conf의 frontendDist와 `generate_context!`로 앱에 포함되는 최종 소스이다.
동일한1.7.0 화면 표시만으로 새 build를 식별하지 않았다.

프로젝트의 기존 entitlements로 ad-hoc 재서명 후 준비 번들/설치본 deep+strict PASS,
arm64 실행파일 SHA256 둘 다
`08faa1acb50352687b1642ae79e1580d7987892bc8105bd81afba5aa12827a8a`.
실행 경로 `/Applications/BroomSweepy.app/Contents/MacOS/bloomsweepy-desktop` 확인.
Apple 공증이나 공개 릴리스 설치 검사로 확대하지 않는다.

CUA ⌘Q와 main process 부재 후 기존9b799c1f… 앱을
`/private/tmp/broomsweepy-guard-install-backup-O8tnSG/BroomSweepy.app`에 이동 보존하고
새 번들을 설치했다. 이전 backup들과 외부 MCP 프로세스는 정리하지 않았다.
첫 실행 전 권한·대화·developer-tools DB는 교체 전후 hash가 동일했다.
앱 사용 중 정상 저장으로 DB 자체 hash가 달라지는 것과 권한 의미 변경은 구분한다.
검사 시작 디스크3.2GiB 여유, 설치 뒤 약3.2–3.3GiB. 사용자 자료·전역 CLI 설정·로그인 교체 없음.

## 자체 fixture와 실제 관찰

새 `/private/tmp/broomsweepy-installed-guard-e2e-04sVQi`에29B 파일 세 개만 만들었다:
conditional-cancel.txt, direct-auto.txt, keep-control.txt. 개인 자료는 검사하지 않았다.
native 폴더 선택창은 경로 직접 입력으로 정확한 폴더/세 파일을 확인한 후 Open했다.
AX 행 클릭이 다른 backup으로 이동한 시도는 Open하지 않았고, clipboard 실패 뒤
settable 경로 필드로 확인해 진행했다. 잘못된 폴더를 검사한 것으로 보고하지 않는다.

| 설치형 시나리오 | 결과와 실제 근거 |
|----------------|------------------|
| 첫 시작 보존 | 이전 자체 대화16개 복원, 새 폴더87B/3파일만 검사. Codex/6.1-Sol/중간 유지 |
| 확인 생략ON + 조건부 요청 | `문제가 없으면 conditional-cancel.txt 삭제해`를 실제 Codex에 전송. 정확한1대상29B native 계획 및 최종 확인 대기/예·아니오 카드 준비, 자동 이동 안 함 |
| 인간 아니오 | composer에 `아니오` 입력. 로컬 BroomSweepy 취소, 선택0개, 활성 확인 카드와 자체 대기 결과 제거, 대화4개 저장 |
| 취소 원본/저널 | 세 파일각29B/총87B 유지, action journal8079B로 실행 기록 증가 없음 |
| 완전 재시작 | 첫 새 host PID84931 ⌘Q 뒤 부재→재실행 PID85789. Settings와 chat 모두 Codex/6.1-Sol/중간, 자체 대화4개/취소 문구 복원. 과거 native 계획/버튼 복원 없음 |
| 권한 보존 | Remember, 확인 생략ON, 시스템 조회ON, 정리 검토ON, 자동시작/메뉴 사용량ON, DockerOFF. readonly DB projection도 앞의4grant 의미 일치 |
| 정확한 직접 명령 자동 이동 | **PENDING — 새 자체 direct-auto.txt29B만 이동하는 정확한 사용자 승인 요청 중. 아직 전송/이동하지 않음** |

조건부 검사에서 LLM 설명만 본 것이 아니라 실제 native 계획이 준비된 화면을 확인했다.
취소는 `humanTrashDecision`의 로컬 분기/앱 표시와 즉시 계획 제거를 함께 확인했다.
CLI argv probe는 질문 완료 뒤라 새 실행을 포착하지 못했다. 이전 argv 관찰 PASS를 새
관찰로 대체하지 않으며, 라벨/강도는 서버 resolved version의 독립 증거가 아니다.
독립 문서 교차검토 PASS/findings 없음. 재시작 검사는 취소한 계획이 재등장하지 않은
범위이며 미취소 계획의 재시작을 별도로 시험한 것은 아니다.

## 증거·남은 경계

ignored local screenshots, `docs/ui-audit/screenshots/2026-10-06/`:
guard-native-conditional-review.png, guard-native-cancelled.png,
guard-native-test-targets.png, guard-native-restart-settings.png, guard-native-restart-chat.png.

직접 명령 검사는 사용자 승인 후 이 새 파일만 질문→새 계획→자동1/1 이동→원본 부재/
저널/나머지58B 보존으로 검증해야 한다. 휴지통 비우기·실제 앱 제거·내용 있는 폴더 자동
이동·Windows·장시간 RAM/FPS/OS스크린리더는 NOT RUN. 현재 앱은 새 자체 대화로 실행 중이다.
이 후속 QA/기억은 로컬 갱신이며 현재 요청에 새 commit/push/release는 포함하지 않았다.

#tags: 설치형검사, 조건부삭제, 취소검증, 재시작복원, 맥설치, arch:011, arch:012
