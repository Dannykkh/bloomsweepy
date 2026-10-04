# Gotchas - 주의사항, 함정

> MEMORY.md 키워드 인덱스에서 이 파일로 연결됩니다.

---

### x64 Node와 arm64 Rust가 섞인 Mac의 Tauri 패키징
`tags: node-x64, rust-arm64, explicit-target, application-management-pending`
`date: 2026-09-07`
`source: codex`

- 이 Mac에서 Node는 x64, Rust host와 실행 파일은 arm64. target 생략 시 컴파일 성공 후 x86_64 sidecar가 없다는 패키징 실패 발생.
- `apps/desktop`에서 `CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=1 APPLE_SIGNING_IDENTITY=- npm run tauri -- build --bundles app --target aarch64-apple-darwin`으로 번들 성공. 공증되지 않은 로컬 ad-hoc 빌드이며 별도 설치 후 검증 필요.
- 번들은 `target/aarch64-apple-darwin/release/bundle/macos/BroomSweepy.app`. `/Applications/BroomSweepy.app`은 아직 교체하지 않음. 신규 앱 관리·명시적 파일 열기는 조사만 완료, 삭제 범위 논의 중.
- **참조**: [이어받기](../.claude/handoffs/2026-09-07-082046-application-management-pending.md)

### 자원 오류는 건너뛴 문서가 아니라 작업 실패로 구분
`tags: worker-failure-rollback, os-resource-errors, mount-root-guard`
`date: 2026-09-07`
`source: codex`

- worker 미설치/실행/시간/할당/IPC 실패를 문서 손상으로 합치면 재색인 generation 정리에서 이전 문서도 삭제된다. typed Infrastructure→WorkerFailure로 전체 rollback, 알려진 파서 오류만 Document로 건너뛰게 했다. 기존 FTS/재시도 회귀 있음.
- ENOMEM/EMFILE/ENFILE도 일반 접근 오류로 무시하면 불완전 색인을 성공 확정할 수 있다. 스트리밍 탐색에서 자원 오류를 종료로 분류하고 열린 iterator를 즉시 회수한다. 실제 자원을 고갈시키지 않는 합성 오류 테스트 사용.
- 폴더 내부 device만 검사하면 선택한 폴더 자체가 마운트 루트인 경우가 빠진다. 검사 루트와 선택 폴더 device도 read_dir 전에 비교한다.
- 소규모 core 프로세스 최고 RSS19.4MiB·반복 전후 안정은 설치 앱/WebKit/CLI 전체나 과거 OOM 원인을 입증하지 않는다. [상세 검증](../docs/qa/2026-09-07-low-resource-folder-actions.md).

### macOS 메모리 초과 제보와 대규모 검사 최고 메모리 위험
`tags: memory-pressure, unbounded-walker, sqlite-temp-memory, pdf-peak`
`date: 2026-09-06`
`source: codex`

- 사용자가 메모리 초과 후 앱 종료를 제보. 종료 앱·당시 작업은 미확인. 신규 Bloom OOM 기록은 찾지 못했고, 현재 호스트28.4MiB/WebContent149.4MiB 및 작은 앱 저장소만으로 과거 종료를 부정하지 않음.
- 기존 HEAD의 jwalk0.8.1 큐는 unbounded이며 워커4개 제한이 대기 항목 바이트를 제한하지 않는다. SQLite temp_store=MEMORY 및 PDF 전체 추출 후 truncate도 피크 위험. 직접 사고 원인으로 확정하지 않음.
- 반복 검색은 최신 결과 교체, 색인은 path upsert+이전 generation 삭제이므로 매번 무한 행 누적은 아님. 다만 전체 RAM/색인 바이트 예산이 없다. 4096파일·128문서 전후RSS 테스트는 실행 중 피크나 WebKit을 입증하지 않는다.
- 코드 수정·OOM 재현·사용자 데이터 삭제 없이 진단만 수행. 다음 우선순위는 bounded 탐색, RAM/DB 예산, PDF 격리 및 작은 fixture peak 계측.
- **참조**: [메모리 위험 진단](../docs/qa/2026-09-06-memory-risk-audit.md)

### 빈 폴더 작업 검증의 Serde 및 디렉터리 링크 수 함정
`tags: serde-unit-variant, root-link-count, empty-folder-tools, mock-not-live`
`date: 2026-09-06`
`source: codex`

- internally tagged enum의 unit variant는 deny_unknown_fields가 있어도 예상 밖 필드를 무시할 수 있었다. `ScanEmptyDirectories {}` 빈 struct variant로 바꾸고 root/command 주입 거부를 테스트했다.
- 부모 디렉터리의 링크 수는 형제 폴더를 이동하면 달라진다. root에는 안정적 FileIdentity만 비교하고 개별 후보에는 실제 비어 있음·정밀 수정 시각·신원 검증을 유지해야 두 번째 정상 후보가 오탐으로 막히지 않는다.
- Rust 205/프론트 37 테스트와 mock IPC UI 통과는 실제 Codex 및 네이티브 OS Trash E2E가 아니다. 새 흐름의 실제 모델 요청·설치·Windows 런타임은 별도 검증해야 한다. OS 경로 기반 휴지통의 TOCTOU 한계도 남는다.
- **참조**: [검증 기록](../docs/plan/conversational-empty-folders/verification.md)

### Rust/Tauri macOS 빌드 순서
`tags: rust, tauri, macos, sidecar, cargo-test`
`date: 2026-09-04`
`source: codex`

- `cargo test --workspace --all-targets` 전에 `apps/desktop`에서 `TAURI_ENV_TARGET_TRIPLE=aarch64-apple-darwin npm run prepare:sidecar`를 실행해야 한다. sidecar가 없으면 Tauri `build.rs`가 리소스 부재로 실패한다.
- Apple Silicon에서는 Rust host가 `aarch64-apple-darwin`인지 확인하고 Tauri 번들에도 `--target aarch64-apple-darwin`을 명시한다. v1.5.0은 Rust 157개 테스트와 ARM64 `.app`/`.dmg`, 설치본 bridge 호출까지 통과했다.
- **참조**: [2026-09-04 대화](../conversations/2026-09-04-codex.md)

### 설치본 AI 채팅의 CLI 호환성 문제 — 미해결
`tags: cli-discovery, authentication-status, claude-safe-mode, chat-qa`
`date: 2026-09-05`
`source: codex`

- ❌ SUPERSEDED — `superseded-by: #cli-준비-상태와-서버-인증을-분리` (앱 연동 수정; 외부 CLI 복구·재로그인은 별도 사용자 작업)

- Codex 실응답·맥락 유지·취소·기록 복원은 통과했지만 GUI 재실행 후 로그인 필요로 바뀌었다. 정상 ARM64 번들과 실행 파일이 누락된 npm 설치본이 공존하며, 실행 환경별 경로 선택 문제가 의심된다. GUI 자식 실행 경로는 직접 포착하지 못했다. 현재 상태 판정은 모든 실행 실패를 인증 필요로 표시한다.
- Claude Code 2.1.19와 2.1.147은 어댑터가 항상 전달하는 `--safe-mode`를 지원하지 않아 채팅 전송이 실패한다. 관련 자동 테스트 19개 통과만으로 설치된 CLI 호환성을 보장할 수 없다.
- 테스트 요청 범위로 원인만 확인했으며 코드·인증 설정·CLI 설치는 수정하지 않았다. 수정 시 도구/MCP 실행 차단 경계를 유지해야 한다.
- **참조**: [설치본 채팅 검증 기록](../docs/qa/2026-09-05-chat-cli-test.md)

### CLI 준비 상태와 서버 인증을 분리
`tags: cli-discovery, authentication-status, oauth-expiry, stdout-errors, chat-qa`
`date: 2026-09-05`
`source: codex`

- ✅ CURRENT — `supersedes: #설치본-ai-채팅의-cli-호환성-문제--미해결`. 앱 내부 실행 파일을 독립 CLI 설치로 보지 않는다. 독립 후보를 열거해 실행·필수 옵션을 검사하고, UI에 경로·버전과 미설치/실행 오류/호환성/인증/확인 실패를 구분한다.
- Claude의 미지원 `--safe-mode`를 제거하고 도구·MCP·사용자 설정·훅 차단을 지원 옵션으로 구성했다. 검사 출력은 익명 임시 파일로 제한하고, 실행 준비 단계도 취소할 수 있다.
- macOS에서 실제 Claude 2.1.147 요청은 서버까지 도달했지만 OAuth 토큰 만료 401로 거부됐다. `auth status` 성공은 서버 토큰 유효성 보장이 아니다. 실패가 stdout에 나올 수도 있어 양쪽 스트림을 분류하고, UI에는 원문 대신 구조화된 재로그인 안내를 보낸다.
- 외부 Codex npm 설치 손상은 남아 있다. CLI 재설치·업데이트·재로그인은 실행하지 않았으며, 실제 Claude 성공 응답은 사용자 재로그인 후 검증해야 한다.
- 최종 설치본에서 실제 요청 후 토큰 만료 안내, 로그인 필요 상태, 전송 차단을 확인했다. 자동 테스트 220개 통과. 일부 콜드 실행은 10초 검사 제한을 넘겨 다시 확인이 필요했다.
- **참조**: [CLI 상태 구분 및 인증 오류 수정 검증](../docs/qa/2026-09-05-cli-health-fix.md)

### 독립 Codex CLI 설치 및 채팅 성공
`tags: codex-install, arm64, standalone-cli, chat-success`
`date: 2026-09-05`
`source: codex`

- ✅ CURRENT — 위 항목의 Codex 재설치 대기 상태를 갱신한다. 사용자 승인 후 공식 독립 설치 프로그램으로 ARM64 Codex 0.153.4를 설치했다. 공개 경로는 `/Users/dannysmacair/.local/bin/codex`, `.zprofile`에 설치 프로그램의 PATH 블록이 추가됐다.
- 기존 손상된 npm 런처와 앱 내부 0.153.3은 보존했다. 새 독립 CLI가 우선 선택되며 설치본 UI에서 경로·버전·CLI 준비됨을 확인했다.
- 실제 앱 테스트 질문에 `연결 확인` 응답 성공. 기존 대화 8개 메시지는 보존되고 테스트 질문/답변 포함 10개가 됐다. Claude 재로그인은 아직 수행하지 않았다.
- **참조**: [독립 CLI 설치 및 앱 검증](../docs/qa/2026-09-05-codex-cli-install.md)

### Codex 상세 답변과 재실행 후 맥락 복원 검증
`tags: response-quality, conversation-restore, chat-header-overlap`
`date: 2026-09-06`
`source: codex`

- 설치본에서 실제 질문 3회 성공: 요약 수치·순위·비율·범위 한계, 후속 참조, 앱 정상 종료/재실행 뒤 기억 복원을 확인했다. 총 8,351바이트·3파일·잔여 155바이트 계산 정확. 테스트 파일 해시 불변, 기존 메시지 10개 보존 후 16개가 됐다.
- 관련 자동 테스트 30개 통과, opt-in 진단 1개 기본 제외. 응답은 CLI 완료 후 한 번에 표시되며 스트리밍이 아니다. 최근 20메시지·메시지당 2,000자·총 24,000자 제한이 있어 무제한 대화 기억을 보장하지 않는다.
- 긴 대화 스크롤에서 sticky 제목과 본문이 겹치는 UI 문제를 확인했다. 이번 턴은 검증만 수행했고 UI나 앱 설치본은 수정하지 않았다.
- 제목 겹침 미해결 상태만 ❌ SUPERSEDED — `superseded-by: #채팅-제목-겹침-수정과-설치본-검증`. 위 응답 검증 결과는 유효하다.
- **참조**: [답변 품질 및 복원 검증](../docs/qa/2026-09-06-codex-response-quality.md)

### 채팅 제목 겹침 수정과 설치본 검증
`tags: chat-header-overlap, normal-flow, layout-fixture, macos-install, adhoc-signing`
`date: 2026-09-06`
`source: codex`

- ✅ CURRENT — 이전 제목 겹침 미해결 상태를 갱신. assistant 전용 클래스를 추가하여 제목을 main 스크롤의 일반 흐름으로 이동했다. 글래스 디자인·다른 화면 sticky 정책·CLI 로직·저장 대화는 유지했다.
- 실제 AppShell/CSS + 합성 메시지 fixture에서 3개 폭 × 5개 상태 15조합 겹침/수평 넘침 없음, 두 폭의 키보드 입력, typecheck·assistant 5테스트 통과. 390px는 브라우저 스트레스 검사이며 네이티브 모바일 지원 주장이 아니다.
- ARM64 앱을 재빌드해 설치. 생성 번들의 linker-only 서명은 strict resource 검사에 실패해 빌드 산출물에 local ad-hoc 서명 후 검증했다. 설치본 정상 실행, 대화 16개와 Codex 준비 상태 복원, 실제 중간/하단 스크롤 겹침 없음. 임시 입력은 전송하지 않고 제거했다.
- 기존 앱 백업: `/Users/dannysmacair/.Trash/BroomSweepy-before-chat-header-20260906-085530.app`. Claude 재로그인·스트리밍은 별도 미완료. 새 AI 요청·커밋·푸시는 없음.
- **참조**: [제목 레이아웃 QA](../docs/qa/2026-09-06-chat-header-layout.md)

### Apple Silicon의 Intel Claude CLI가 10초 상태 검사를 초과함
`tags: claude-arm64, rosetta, startup-latency, macos-soak, oauth-expiry`
`date: 2026-09-06`
`source: codex`

- Claude 2.1.147 설치 파일이 x86_64였으며 `--version`에 AVX 경고와 12.64초 지연. 같은 버전의 공식 ARM64 바이너리로 교체하고 manifest SHA-256·서명을 검증했다. 최초 6.42초, 재측정 0.77/0.06초. 앱의 10초 제한은 바꾸지 않았다.
- 공개 symlink는 유지. 이전 Intel 바이너리는 `/Users/dannysmacair/.Trash/claude-2.1.147-x64-before-arm64-20260906-094804`에 보존. 로컬 CLI 준비와 서버 인증은 별개: 실제 질문은 여전히 만료 인증 안내, 기존 대화는 17개 메시지로 보존.
- macOS 전용 4,096파일/128문서 1,000회 반복 회귀, 취소/재시작·8GiB 희소파일·링크 순환·중복 검증을 추가하고 통과했다. 161.36초의 제한된 검사이며 하루 종일/실제8GiB읽기/모든Mac 검증이 아니다. 전체 일반 테스트221개 통과.
- 최종 완료는 Claude 브라우저 로그인과 성능 화면 정상 종료 실기 검증 대기. 백그라운드 CUA에서 초기 성능 측정이 진행되지 않아 미검증으로 남겼다. 계측 hook의 hidden 상태 polling 생략이 원인 후보이며 실제 제품 결함으로 확정하지 않았다.
- **참조**: [macOS 완료 검증](../docs/qa/2026-09-06-macos-completion.md)

### macOS 검증 범위 변경과 클라우드 하위 순회 누락
`tags: cloud-exclusion, jwalk, codex-common-flow, macos, scan-safety`
`date: 2026-09-06`
`source: codex`

- 위 항목의 Claude 로그인 대기는 사용자 지시로 취소됨. Claude는 Windows에서 검증했다고 사용자가 설명했으며, 맥에서는 Codex를 공통 채팅 흐름의 기준으로 사용한다. 다른 공급자의 인증/실행 성공을 Codex 성공에서 추론하지 않는다.
- 기존 cloudVolumePolicy는 대시보드 볼륨 표시만 숨겼고 Rust 하위 순회에는 클라우드 제외가 없었다. macOS Library/CloudStorage·Mobile Documents가 전체 드라이브 스캔에 들어갈 수 있다. jwalk process_read_dir에서 자식 작업 예약 전 가지치기하고 5개 스캔/색인 진입점과 직접 root 선택에 공통 규칙을 적용 중이다.
- 진행 중이던 실제 전체 검사는 앱의 검사 중단 버튼으로 취소 완료했다. 디스크 약 240MB로 빌드 공간 부족; target/debug/incremental 약5.7GB만 제거해도 되는지 사용자에게 요청했다. 클라우드 다운로드가 디스크 부족의 원인인지는 확인하지 않았다.

### 클라우드 제외 수정본 macOS 설치 완료
`tags: cloud-exclusion, macos-install, scan-safety, compiler-cache, codex-common-flow`
`date: 2026-09-06`
`source: codex`

- 사용자 승인 후 정확한 target/debug/incremental 컴파일 캐시만 제거하고 ARM64 앱/sidecar 빌드·서명·설치 완료. 이전 앱은 휴지통의 BroomSweepy-before-cloud-exclusion-20260906-1205.app에 보존. 빌드 후 파일시스템 여유4.2GiB.
- 수정 후 전체 Rust193+frontend35=228테스트 통과, full-workspace clippy/format/typecheck/build 통과. 설치본에서 가짜 Google Drive/iCloud/OneDrive 파일을 제외하고 로컬2개22B만 집계, 클라우드 root 직접 선택 차단을 확인했다. 실제 클라우드/전체드라이브를 재검사하지 않았다.
- 자체 테스트 파일5개와 임시 폴더만 제거하고 앱 재실행. 기존 대화17개 보존, Codex CLI 준비 상태로 선택 복원, 대시보드 대기. 모든 공급자/임의 미러링 폴더/Windows 실기 검증을 완료했다고 확대 해석하지 않는다.
- **참조**: [클라우드 제외 QA](../docs/qa/2026-09-06-cloud-scan-exclusion.md)

### Rust UI v1.6.0 릴리스와 공개 캡처
`tags: release-1.6.0, screenshots, https-push, adhoc-dmg`
`date: 2026-09-06`
`source: codex`

- 사용자 요청으로 Rust 기본 설명의 README 네 언어와 실제 React 컴포넌트 기반 예시 캡처20장, 개발 전용 release-preview를 공개. 실제 파일·AI 답변으로 오해하지 않도록 표시했다. 개인 QA·레거시 영상 자산·영상 인증 파일은 커밋하지 않았다.
- e4f3ad8(core), cb6cf47(desktop), 378ec22(docs), 5fa55a7(release)와 annotated v1.6.0을 main에 푸시. SSH 인증은 실패했지만 시스템 credential helper의 정상 HTTPS 인증을 사용했다. 큰 이미지 pack의 HTTP400은 명령 범위 http.version=HTTP/1.1, http.postBuffer=52428800으로 해결했으며 원격 설정이나 기존 기록을 강제 변경하지 않았다.
- ARM64 앱+DMG1.6.0 빌드, strict/deep 서명, DMG 무결성, DMG 내부 실행파일의 원본 일치 및 helper1.6.0 확인. GitHub release383467664에 DMG와 SHA256SUMS 업로드; 서버 digest가 로컬과 일치. DMG SHA256: 7e28fb4f73727718cdeed6429d9d47920c8862e0132a865d7225f13a9632524e.
- 로컬 회귀 Rust193+frontend35 통과, clippy/format/typecheck/build 통과. macOS는 ad-hoc 서명이며 미공증, Intel 바이너리 없음. Windows CI34012147538/macOS CI34012147541은 게시 시 진행 중이며 새 Windows 설치 파일은 미첨부. /Applications 설치본은 앞서 설치한1.5.0 클라우드 수정본을 유지했다.
- **참조**: [v1.6.0 릴리스](https://github.com/Dannykkh/bloomsweepy/releases/tag/v1.6.0)

### 공통 Tauri 기능과 OS별 JSON 설정의 Windows 빌드 충돌
`tags: release-1.6.1, tauri-config, windows-ci, private-api`
`date: 2026-09-06`
`source: codex`

- v1.6.0 macOS CI는 성공했지만 Windows CI34012147538은 cargo clippy의 tauri-build 설정 검증에서 실패했다. 공통 Cargo tauri 의존성은 macos-private-api를 선언하고, 일치하는 app.macOSPrivateApi는 Mac 전용 JSON에만 있어 Windows에서 불일치했다.
- 설치된 tauri-build2.6.3/src/manifest.rs는 공통 의존성 선언을 먼저 검사한다. 공통 tauri 의존성을 남긴 채 기능만 target별로 옮기는 수정은 Mac 검증을 깨뜨릴 수 있다. 공통 JSON에 기능 플래그를 맞추고 창 투명도·vibrancy는 Mac 오버라이드에만 유지한다.
- 수정738cd1d와 버전8c7250b 및 annotated v1.6.1을 main에 푸시했다. Windows34012634057(Rust195+frontend37,6 opt-in 제외)와 macOS34012633956(Rust193+frontend37,1 opt-in 제외) CI 최종 성공. 로컬1.6.1 앱/DMG 빌드·서명·패키지 일치 확인도 통과했다.
- 14:15KST에 release383473561을 공개했다. ARM64 DMG, Windows x64 EXE·MSI, 통합 SHA256SUMS 모두 업로드하고 서버 digest를 로컬과 대조했다. Mac DMG SHA2561427f047bcd0d0d185f6062a25c0c0b4e9b122b9fc2f76d9fb1d0a960d681f64. v1.6.0 태그/파일은 그대로 보존; Mac은 ad-hoc/미공증, 설치본은 교체하지 않았다. CI 성공이 Windows 실제 GUI 설치·NTFS 실볼륨·휴지통 조작 검증을 의미하지 않는다.
- **참조**: [수정 커밋](https://github.com/Dannykkh/bloomsweepy/commit/738cd1d)
- 후속19:12KST: 사용자 요청으로 기존 앱을 휴지통 BroomSweepy-before-v1.6.1-20260906-191140.app에 옮기고 검증된 릴리스 DMG에서 /Applications에1.6.1을 새로 설치했다. 앱/CLI버전·서명·실행파일 일치, 한국어 대시보드 실행 확인. 설정/대화 저장15파일은 실행 전 교체 전후 SHA256이 동일했다. 위의 '설치본 미교체'는14:15 게시 당시 상태이며 현재는1.6.1 설치 완료다.

### v1.7.0 정식 공개 준비와 DMG 포장 대체
`tags: release-1.7.0, stable-release, hdiutil-fallback, ci-artifacts`
`date: 2026-09-07`
`source: codex`

- 사용자가 실제 Codex 종단·장시간 메모리 검증 미완료를 안내받은 뒤 정식 공개를 선택했다. 해당 한계와 앱 용량/설치일/정렬 미구현을 README4언어·공개 릴리스 문서에 명시했다.
- main에 be47f5d(core), 7408b19(desktop), ac9e747(docs), 3c499cb(release)를 HTTPS로 푸시. CI Windows34092133320/macOS34092133345 진행 중이며 이 기록 시점 태그/릴리스는 아직 미생성.
- Tauri 앱+helper1.7.0 ARM64 빌드 성공 후 bundle_dmg.sh만 실패. 동일 서명 앱과 Applications 링크를 임시 staging에 복사해 hdiutil UDZO로 DMG 생성. 무결성·마운트 내부1.7.0·strict/deep서명·두 helper버전·heap 진단 통과. Mac DMG SHA25663eec1136fd5a49312f6885c5c3ff06aa7baa075bd85a7d2aab0307742f1cb9a.
- 설치된 /Applications 앱은 교체하지 않았다. 새 스크린샷도 만들지 않고 기존 v1.6.0 이미지를 참고용으로 표시했다. 개인 QA/영상 인증/도메인 사전 변경은 공개 커밋에서 제외했다.
- **참조**: [.claude/handoffs/2026-09-07-154524-release-v1.7.0.md](../.claude/handoffs/2026-09-07-154524-release-v1.7.0.md)
- 완료18:17KST: Windows 초기 CI에서 verbatim drive prefix 메타데이터 조회와 AppData 내부 action fixtures, 파일 열기 canonical path 거부를 발견. 2d5cb20/b44eb14 및 문서·포맷 후속을 포함한 최종 a2d055ec2150310154289e095c5369b115826142에 annotated v1.7.0을 푸시했다. Windows34100523574/macOS34100523551 모두 최종 성공. 보호 정책은 유지하고 로컬 drive prefix만 정규화했다.
- GitHub release383963327 정식 공개(draft=false, prerelease=false). Mac DMG 최종 SHA256ca42dde7aa213270fb75e8ded2ab1338bbb081df8c1f514a6269061175b1dddd, Windows MSI4f0c311fe8b78303438fc8e73dfbf4e19bfbb955cd1bb69f49ae15bbd1cd9319, EXE8bba795beeb249d2f893aa51a782ad8d3f319fbd731e6a2e9165b8da83aa08f8. 3설치파일+SHA256SUMS 서버 digest/size 일치 검증. 이전 릴리스는 보존, 설치된 Mac 앱은 이번 요청에서 교체하지 않았다.

### 배포용 allocator 자가검사의 allocation elision
`tags: release-allocator-probe, allocation-elision, heap-budget`
`date: 2026-09-07`
`source: codex`

- `std::alloc::alloc` 결과를 검사하고 즉시 해제하던 자가검사는 debug PASS/release exit1을 재현했다. Rust GlobalAlloc 계약은 불필요한 할당 제거/성공 가정을 허용한다.
- 진단만 전역 `ALLOCATOR`의 구체 alloc/dealloc 메서드를 직접 호출하도록 수정. release helper 경계2개와 자가검사 PASS. allocator 정책/128MiB budget은 바꾸지 않았으며 전체 RSS 보장은 아니다.
- prepare-sidecar에서 버전과 heap rejection 진단을 모두 통과해야 번들을 준비하도록 연결했다.
- **참조**: [앱 관리/설치 QA](../docs/qa/2026-09-07-application-management-open-actions.md)
