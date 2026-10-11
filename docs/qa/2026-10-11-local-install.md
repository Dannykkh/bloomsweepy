# 이 Mac의 ARM64 개발본 설치 검증

2026-10-11 11:22 KST / macOS26.4 Apple Silicon8GiB / source worktree main f1feac1 + 기존 dirty 변경.
사용자 요청: “설치하자”. 공개 릴리스 업데이트가 아니라 현재 개발본의 로컬 설치다.

## 결과

`/Applications/BroomSweepy.app` 교체·실행 PASS. 기존 대화38메시지와 설정을 복원했다.
version은 여전히1.7.0이므로 버전 문자열만으로 새 빌드를 구분하지 않는다.
최종 설치 executable SHA256:
`4711f90679c31889e84398f4ce5373b269eb70d6e4c18410758418488bd664e4`.

## 빌드 / 설치

- 실행 명령: `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 CARGO_PROFILE_RELEASE_DEBUG=0 npm run tauri build -- --target aarch64-apple-darwin --bundles app`, cwd `apps/desktop`.
- 첫 desktop release7m40s, 팝업 수정 후 desktop release3m03s PASS. 매번 단일 Cargo만 실행.
- sidecar2개 release, TypeScript/Vite production build PASS.
- 경고: 기존 큰 JS chunk(~1008.71kB)와 unused Rust wrapper2개. 경고를 실패나 수정 완료로 숨기지 않는다.
- `Info.plist` lint/identifier/version/executable 확인. main·MCP·document-worker 모두 Mach-O arm64.
- 최초 bundle의 resource seal 검사는 실패했다. 기존 entitlement를 사용하여 MCP→worker→app 순
  ad-hoc 서명 후 `codesign --verify --deep --strict` PASS. notarization/DMG 생성은 하지 않았다.
- 준비된 staging을 ditto로 복사·검증하고 기존 앱을 백업한 뒤 교체했다. 설치 실행파일과 빌드 실행파일 `cmp` PASS.
- MCP/worker `--version`1.7.0, document-worker `--check-heap-budget`→`heap-budget-rejection-ok` PASS.
- 원래 앱: `/private/tmp/broomsweepy-install-backup-jVoGhr/BroomSweepy.app`.
- 첫 설치 개발본도 같은 backup 디렉터리의 `BroomSweepy-first-install.app`에 보관했다.
- 임시 staging2개는 앱 이동 후 빈 디렉터리만 제거했다. 원래 앱·사용자 데이터는 삭제하지 않았다.
- 설치 main 실제 경로 `/Applications/BroomSweepy.app/Contents/MacOS/bloomsweepy-desktop` 확인.
  최종 시작 PID29481. 기존 external MCP PID10077/18831은 종료하지 않았다.
- 마지막 df 여유 약2.2GiB. cache/user data/다른 앱을 정리하지 않았다.

## 데이터 보존

앱 종료 상태에서 시작 전·첫 복사 후·첫 실행/⌘Q 후·최종 복사 후 두 SQLite hash가 모두 일치했다.
`~/Library/Application Support/com.broomsweepy.desktop` 내용은 읽어 추출하거나 수정하지 않았다.

| 저장소 | SHA256 |
|---|---|
|assistant-sessions-v1.sqlite3|`7ae76f48b4bcfa5f1d364dc2f60779c422ece2f614a40c76ed16dc43abf8ed26`|
|permission-settings-v1.sqlite3|`0554008e1dd1dfaf9d4d283fefb0eebaff75adab1baccd0e5698e31f604adfe7`|

hash 동일은 이 두 main DB 파일의 해당 시점 비교다. 모든 파일/WAL의 불변이나 장시간 resource 안정성까지 보장하지 않는다.
startup recovery/권한 재검증은 정상적으로 내부 저장을 갱신할 수도 있다.

## 실제 설치 UI / 회귀 수정

첫 설치에서 권한·모델 popup은 AX에 열림으로 나오지만 실제 screenshot에는 보이지 않았다.
Escape/재열기/창 Raise/좌표 클릭 후에도 동일했다. 합성 Chrome QA만으로 발견하지 못한 native 회귀다.

`apps/desktop/src/views/AssistantView.css`의 `.assistant-dock`에 `z-index:1`을 추가하여
composited transcript와 dock/popup의 stacking 순서를 명시했다. geometry/포커스/권한 로직은 바꾸지 않았다.
수정 후 release를 다시 만들어 교체한 뒤 **두 popup이 실제 픽셀에 표시됨**을 확인했다.
WebKit 내부 구현 원인을 instrument한 것은 아니며, 동일 설치 상태의 한정 CSS 변경 전후 관찰이다.

| 설치형 확인 | 결과 |
|---|---|
|기존 대화 복원|38메시지·기존 선택 세션 표시 PASS, 새 질문 전송 없음|
|설정/입력창 권한 공유|`확인 후 실행`, `이 컴퓨터에서 기억` PASS|
|완전 종료/다시 시작|⌘Q main 부재→수정 설치본 재실행→같은 현재값/대화 표시 PASS|
|권한 popup|조회만·확인 후 실행·전체 접근, confirm 체크, 화면 경계 내 표시 PASS|
|모델 popup|GPT-6.1-Sol·중간·지원 range가 실제 표시됨 PASS|
|Escape|두 popup 닫힘, 각각 trigger로 포커스 복귀 PASS|
|CLI 상태|Codex CLI 준비됨·저장 모델/강도 확인 PASS. 서버 인증/응답 성공은 미검증|
|기존 옵션|file-change review OFF/선택 폴더 없음, 자동 시작 ON, RAM 제목 ON 유지. 스위치 조작 없음|
|권한 확대/파일 작업|실행 안 함. 모드/외부 grant/OS FDA/문서 공개를 켜지 않음|

UI screenshot은 사용자 대화가 포함될 수 있으므로 Git에 복사하지 않고 로컬 임시 경로에만 보관했다:
`/private/tmp/broomsweepy-install-proof-eZPBVa/{permission-popup,model-popup,installed-settings}.jpg`.
마지막에 popup을 닫고 대화 화면으로 돌려 앱을 실행해 두었다.

## NOT RUN / 남은 범위

실제 새 mode 저장/전체 접근 grant·전체 볼륨 검색·native IPC 경쟁상태·CLI 질문 종단/서버 인증·
사용자 파일 휴지통 이동·Windows·네트워크/USB 실장치·장시간 RAM/disk soak는 NOT RUN.
기존 [개발 QA](2026-10-11-chat-access.md)의 단위/합성 결과와 이 설치 smoke를 합쳐 전체 종단 성공으로 표현하지 않는다.
서명은 로컬 ad-hoc이며 공개 배포/Apple 공증/보안 경고 우회 검증이 아니다. 커밋·푸시·릴리스 없음.

#tags: installed-qa, arm64, data-preservation, webkit-popup, chat-access, arch:015
