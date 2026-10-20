# Windows 관리 구현 / Mac 회귀 검증

2026-10-20 · Codex · main @85620ca 이후 로컬 변경. macOS26.4/aarch64,8GiB. 설치본 교체·배포 인증이 아니다.

## 범위와 결과

Windows 설정 메뉴/고정10행 저장값 조회/공식8진입/공통 읽기 전용 `windows.inspect`를 구현했다. Mac은 기존 메뉴와 Finder5개·내구성 변경 저널을 유지한다. [구현 계약](../../plan/windows-management/implementation-contract.md), [기능 계약](../../architecture/app-capability-contract.md).

| 검사 | 실제 결과 | 판정 |
|---|---|---|
| 프론트 전체 | `npm run check`, `npm run test:all`170/170, `npm run build` exit0 | PASS |
| Windows control 계약 | control31/31; 무인자 조회만, 임의 target/쓰기 거부,27개 capability 발견/16KiB budget | PASS |
| desktop Rust lib | release323PASS/6ignored; Windows9개, 모델 projection·기존 Mac/파일/권한 회귀 포함. opt-in ignored는 실행으로 계산하지 않음 | PASS |
| 실제 Mac 읽기 smoke | `mac_settings::tests::native_catalog_readonly_smoke --exact --ignored`1/1; macOS26.4 지원=true, Finder5개 unset/None, 쓰기 없음 | PASS |
| Windows actual-source cross typecheck | `x86_64-pc-windows-gnu`에서 실제 `model.rs/native.rs`·기존 dependency21개의 버전/checksum 일치. 최종 offline check exit0/경고0 | PASS — 타입 검사만 |
| 전체 Rust workspace |490PASS/0failed/6ignored, exit0. control31+core112+통합9+앱323+MCP15. smoke는 별도 실행 | PASS |
| 실제 Wry 부정 IPC |15/15 case·거부 후12/12 불변 관찰, exit0. Finder5개/분리 journal·sidecar 무변경. Windows 빈행·targets/unsupported·local OS거부·secondary·remote ACL 확인 — [전용 기록](native-ipc-README.md), [stdout](native-ipc-result.jsonl) | PASS |
| 정적 리뷰 | reader KEY_READ/64bit, fixed target, local main origin, disclosure/cancel/budget, OS gating. 추가 actionable finding 없음 | PASS |
| 형식 | `cargo fmt --all -- --check`, `git diff --check` exit0 | PASS |

빌드에는 기존1.10MB JS(313KB gzip) 단일 chunk의500KB 경고가 남는다. 전체workspace의 기존 `assistant_files::scan_workspace/apply_workspace_action` dead_code 경고는 이번 Windows 기능의 실행 실패가 아니다. 번들 분할·장기 메모리 안정화를 검증했다고 주장하지 않는다.

## 실제 App 브라우저 회귀 (합성 IPC)

IAB localhost1431의 `workspace-navigation-fixture.html`이 실제 App/화면/핸들러를 렌더한다. 모든 IPC는 합성이며 unknown/write는 거부한다. 화면 하단에 합성임을 표시한다. Windows 데이터는 가짜 빌드26100.3915/에디션Synthetic QA, 저장값0/1/없음이다. 사용자 파일 검사/OS 설정 변경/AI 전송은 없다.

| Case | 관찰 | 판정 |
|---|---|---|
| Windows desktop1280×820 | 컴퓨터 네 번째 Windows 설정, Mac 메뉴0, 현재 메뉴1, 공식8연결·Explorer4·기본 접힌AI6 | PASS |
| 저장값·범위 | DWORD0이0으로 보이고 unset은미설정. Recall 사용자/기기 독립. AI 지원 미확인·legacy폐기 | PASS |
| `windows-case=mixed` | 값/부재/REG_SZ불일치/접근거부/미지원값9/오류가 서로 다름; false/0 추정 없음 | PASS |
| 공식 storage 진입 | 합성 요청 수1, requested 안내·변경 완료 아님 | PASS — native 창 열림 아님 |
| `open-error` | 키보드 Enter로 실제 View handler 오류 안내 확인, 실패 후 다시 요청 가능 | PASS |
| `error` | 첫조회 실패에 행0·이전값 안내 없음. 실렌더에서 발견한 문구 버그를 수정하고 unit회귀 추가 | PASS |
| `unsupported` | 행/공식 연결이 없고 비지원 안내, OS metadata미확인 유지 | PASS |
| `slow` |1600ms 합성 지연 중 실제 진행 문구·refresh비활성, 성능 화면으로 이탈 가능. 응답 후에도 성능 화면 유지·Windows view0 | PASS |
| Windows390×844 | 문서 가로 overflow0, 실제 버튼 최소44px, 공식 연결2열171px, Koreanfont로드true | PASS |
| Mac 메뉴/동선 | Mac만 Finder메뉴;5행·history·초기적용비활성. 공통 공간정리→시스템후보 pressed=true | PASS |
| Mac 앱 설정 | 일반/검사·검색/AI·권한/파일감시/개발도구/정보. 분류 전환 뒤 모델조회수1 유지 | PASS |
| Mac390×844 | overflow0, Finder5행, drawer열림 main inert / 메뉴선택후 해제 | PASS |
| 브라우저 오류 | 새 화면·전환 후 console error/warn0 | PASS |

### 캡처

- [Windows desktop](windows-desktop.jpg), [Windows narrow](windows-narrow.jpg).
- [Mac desktop 회귀](mac-regression-desktop.jpg), [Mac narrow 회귀](mac-regression-narrow.jpg).

캡처는 합성 IPC를 사용하는 **실제 프론트 렌더**다. 실제 Windows/설치본/네이티브 glass 캡처가 아니다. 디자인은 frontend-design 기준과 기존 DESIGN 토큰을 재사용했고 새 이미지·아이콘·의존성은 추가하지 않았다.

## 재현 명령

```sh
# repository root, one build worker on the8GiB Mac
CARGO_BUILD_JOBS=1 cargo test --offline --locked --release --workspace --quiet -- --test-threads=1
CARGO_BUILD_JOBS=1 cargo test --offline --locked --release -p bloomsweepy-desktop --lib mac_settings::tests::native_catalog_readonly_smoke -- --exact --ignored --nocapture --test-threads=1
```

프론트는 `apps/desktop`에서 `npm run check`, `npm run test:all`, `npm run build`. fixture query는 `platform=windows`와 `windows-case=mixed/error/unsupported/slow/open-error`. UI 조작은 Cua에서 수행했다.

Windows cross-check 하네스는 [source](native-cross-check.rs.txt)/[manifest](native-cross-check-Cargo.toml.txt)다. 임시crate에 lib.rs/Cargo.toml로 복사하고 현재 프로젝트 절대경로를 맞춘 뒤 `CARGO_BUILD_JOBS=1 CARGO_TARGET_DIR=<repo>/target cargo check --offline --target x86_64-pc-windows-gnu --manifest-path <tmp>/Cargo.toml`. 실제 실행 디렉터리 `/private/tmp/broomsweepy-windows-native-check-qmOWhw`. target std는 공식 rustup으로 준비했고 신규 프로젝트 의존성은 없다. `check_native_contract`는 cargo check이므로 실행하지 않는다. 이 검사는 Windows Tauri wrapper/전체앱/링크·installer의 성공을 증명하지 않는다.

## NOT RUN / 남은 경계

- 실제 Windows 일반/관리형 PC 레지스트리 조회·공식 설정 페이지 표시·오류·installer/전체 Tauri 빌드. Mac 교차 타입 검사로 대체하지 않는다.
- OS 설정 쓰기/관리자 상승/AI·서비스·보안/시스템 패키지 제거는 구현 범위 밖. 원복 가능하다고 주장하지 않는다.
- 새 Mac 설치본 교체·전체 product setup·실제 LLM/MCP 클라이언트 종단 대화·사용자 자료 삭제·장기 RAM soak는 실행하지 않았다.
- forced reduced-motion emulation은 선택 IAB에 노출되지 않아 NOT RUN. 새 CSS의 reduce fallback과 기존 모션 분기는 정적 확인만 했다.
- 커밋·푸시·릴리스·원격 CI는 이번 요청이 아니어서 실행하지 않았다.

#tags: windows관리, 맥회귀, 실제조회, 합성ui, 검증경계, arch:018
