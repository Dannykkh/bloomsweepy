# 정리 후보 트리 — macOS 설치형 검증

2026-10-05 07:02 Asia/Seoul · source: codex · 현재 개발본 · 맥 설치형 트리 흐름 통과

## 현재 승인과 범위

직전 구현 완료 보고의 다음 단계인 맥 설치형 검증을 사용자가 “진행하자”로 승인했다. 새 개발본 빌드·기존 앱 백업/교체·직접 만든 임시 자료의 실제 OS Trash 테스트를 수행한다. 개인 자료, 기존 휴지통 항목, 다른 앱, 시작 프로그램/보안/공급자 설정은 변경하지 않는다. 커밋·푸시·릴리스는 이번 요청이 아니다.

## 빌드와 저용량 준비

ARM64 Mac의 최초 여유는 약2.4GiB였다. CARGO_BUILD_JOBS=1/CARGO_INCREMENTAL=0을 사용한다. 명시적 target 빌드를 중단하고 기존 host cache를 사용하는 동일 ARM64 빌드로 전환했다. 동봉 MCP1.7.0과 문서 worker의 버전/heap-budget 검사가 완료됐다.

빌드 중1.9GiB에서 이번 실행 전에 없었던 ignored `target/aarch64-apple-darwin` 중간 캐시297MiB만 제거해2.2GiB로 회복했다. sidecar는 이미 src-tauri/binaries에 복사됐고 현재 main host 빌드는 다른 target/release를 사용한다. 삭제한 캐시는 재빌드할 수 있다. 기존 소스·target/debug·설치 앱·rollback은 제거하지 않았다.

본체 release 컴파일은6분45초에 완료됐다. 기본 패키징은 x86_64 sidecar를 찾으며 실패했다. 실제 Node `process.arch`는 x64이고 Rust 실행 파일은 arm64였으므로 default bundler target과 Rust host가 달랐다. ARM64 실행 파일을 명시적 target의 출력 위치에 준비하고 `tauri bundle --target aarch64-apple-darwin`으로 동일 파일을 패키징했다. 전체 재컴파일이나 Node/의존성 교체를 하지 않았다.

첫 strict signature 검사는 linker의 ad-hoc 서명만 있어 bundle resource seal이 없다는 오류를 반환했다. 동봉 도구 둘과 앱을 기존 entitlements로 로컬 ad-hoc 서명하고 `codesign --verify --deep --strict`를 통과했다. Apple 공증/공개 DMG 검증이 아니다. 세 실행 파일 모두 Mach-O arm64이며 MCP1.7.0·document-worker heap-budget 검사를 통과했다.

기존 앱은 정상 종료 후 `/private/tmp/broomsweepy-tree-install-backup-3Nh8NQ/BroomSweepy.app`으로 보관하고 새31MiB bundle을 `/Applications/BroomSweepy.app`에 설치했다. 설치 파일과 빌드 파일의 `cmp`, strict/deep signature 검사가 통과했다. 이전 rollback 앱·데이터·저장 대화는 지우지 않았다. 최종 df 여유는 약2.3GiB이며 삭제209B로 늘어난 공간이라고 해석하지 않는다.

설치 binary SHA-256: `fb43ed8d4de842abf28213a1c2b9d8a93d7e2a7c938fb04ac3b77e4e2f6c8840`. 보관한 이전 binary: `e832fe1d31af3fb34ad5ab87c5bff19d015edef1285163adb19d234617a8ceb5`. 버전 표시는1.7.0 유지이며 GitHub 배포 파일/태그는 변경하지 않았다.

## 직접 만든 테스트 범위

루트: `/private/tmp/broomsweepy-native-tree-p09HiN`. 이번 작업이 생성한 자료이며 개인 파일이 아니다.

| 항목 | 바이트 | 목적 |
|---|---:|---|
| old-project/notes/KEEP.txt | 69 | 부모 선택 뒤 하위 해제; 반드시 유지 |
| old-project/notes/draft.txt | 75 | 선택 휴지통 대상 |
| old-project/archive.txt | 77 | 선택 휴지통 대상 |
| disposable-whole/part.txt | 57 | 전체 폴더 이동의 하위 확인 체크 |
| protected-group/.git/config | 64 | 미전개 보호 하위 때문에 부모 전체 이동 차단 |
| Protected.app/placeholder.txt | 64 | 앱 번들 후보 체크 차단 |

보존 항목의 초기 SHA-256: KEEP `988df19d741502c2b9d286f94498732b493af35cc897efce21df5b7309b39644`, git marker `85c6883d954f4e1f20d81d1054c47cf4c46e6e462dd1bb343acaedfcee87ce9c`, app marker `fccc78b14953c2b9d5b3199d92302e4a74f217653c75c76236924e326ba8e5c5`.

실제 실행 대상은 draft75B + archive77B + disposable-whole57B, 중복 없는3대상209B였다. 아래 검토 UI·실행 결과·원본 부재·휴지통 파일 메타데이터와 저널이 일치했다.

## 실제 설치 앱 결과

CUA로 `/Applications/BroomSweepy.app`의 native 창을 조작했다. 합성 browser adapter가 아니라 tauri://localhost의 실제 IPC/파일 검사/휴지통 작업이다.

| 흐름 | 확인 결과 |
|---|---|
| 지도 검사→트리 | 임시 루트406B·직계4개·하위 파일6개. 같은 검사 결과에서 실제 트리 생성 |
| 보호 후보 | Protected.app checkbox disabled. 미전개 protected-group을 선택해 검토하면 `.git` 보호 오류로 계획 생성 거부 |
| 부모 상속 | old-project 선택 후 notes/draft/KEEP/archive를 펼쳐도 checked 유지 |
| 하위 제외 | KEEP를 해제하면 old-project와 notes는 mixed. 최종 목록에 부모 둘과 KEEP 없음 |
| 전체 폴더 함께 선택 | disposable-whole을 추가해3대상209B. 실제 정확한3경로 표시, nested ack 전 실행 disabled |
| 취소 | Cancel 기본 포커스→취소 후 검토 버튼으로 복귀, 선택 유지. 모든 원본과 KEEP 해시 동일, 이 테스트의 journal 항목0 |
| 실제 이동 | 새 검토→ack→버튼. 이동3/209B·실패0·생략0, 결과 포커스 확인 |
| 원본/휴지통/저널 | 이동된 원본3개 부재. `.Trash/archive.txt`77B·`draft.txt`75B·`disposable-whole/part.txt`57B 존재. operation `1791150929081-72950-1`의3개 moved와 completed209B 기록 |
| 보존 | KEEP69B와 부모 notes, 두 보호 marker 유지. 세 SHA-256 모두 초기값과 동일 |
| 재검사 | result의 다시 스캔으로 새 트리 생성, 이동 대상 없음·선택0. KEEP 표시. 트리 하위를 펼친 뒤에도 지도는 임시 루트 전체 범위 유지 |
| AI 카드 진입 | 임시 루트로 새 대화1개 생성→로컬 파일·폴더 검사→실제 측정 카드→session/revision 출처 트리. 자동 선택0, 같은 로컬 루트 새 검사도 성공. 외부 모델 요청0 |
| 재실행 | 정상 종료→프로세스 종료 확인→재실행. dashboard 최근 정리에3개/209B 기록과 이전 기록 함께 유지 |
| 외부 권한 | 동봉 MCP status 성공, bridge idle. inspection/scan/cleanup/files/documents 외부 권한은 기존 closed 상태 유지 |

Finder 휴지통 작업 후 임시 루트와 부모에 `.DS_Store`가 생겨 재검사 총량은197B가 아니라18.2KB로 표시됐다. 실제 생성된 파일을 반영한 값이며 앱이 stale406B를 유지한 것이 아니다. OS가 만든 메타데이터를 추가로 삭제하지 않았다.

호스트 RSS의 관측값은 약82–117MiB였고 재실행 후 약100MiB였다. 소수 시점의 호스트 측정일 뿐 WebView를 포함한 전체 프로세스 트리나 장시간 메모리 안정성 보장은 아니다. native 화면 이미지를 도구에서 확인했지만 캡처 파일은 저장하지 않았다.

## 경계와 NOT RUN

휴지통 파일의 내용 해시를 별도로 읽으려 했으나 macOS가 Operation not permitted로 거부했다. 파일 존재/크기·원본 부재·native 결과·저널은 확인했고, OS 보호를 풀거나 접근 권한을 추가하지 않았다. 휴지통 내용 해시 및 실제 복원은 NOT RUN이다. 휴지통을 비우지 않았으므로 테스트3대상은 OS 휴지통에 남아 있다. 나머지 임시 보존 자료/테스트 대화와 rollback 앱도 유지한다.

외부 LLM 조사/응답 종단, AI 출처 트리 이동 뒤 stale revision refresh의 전체 종단, Windows/장시간 메모리 soak/전체 integration, future-drop worker lease/main 외 Webview 거부/실제 만료 타이밍/큰 페이지 부하는 NOT RUN이다. 이번 검증이 이 맥의 모든 기능 완료나 공개 릴리스 검증을 뜻하지 않는다. 제품 소스는 바꾸지 않아 이전 Rust320·프런트51 회귀를 재실행하지 않았고, 현재 소스를 포함한 release 컴파일/프런트 빌드와 설치형 실동작을 추가했다.
