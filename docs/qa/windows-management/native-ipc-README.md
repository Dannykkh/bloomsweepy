# Mac 호스트의 Mac·Windows 설정 실제 IPC 검증

2026-10-20 23:44 KST · macOS 26.4 (25E246), arm64.

최신 생산 설정 모듈 5개를 수정 없이 별도 임시 crate에 복사하고, 실제 Wry WebView의 JavaScript → Tauri IPC로 조회·거부 경로를 검증했다. 정상적인 Finder 변경 ID는 보내지 않았으며, Windows 실행을 시도하는 하네스를 Windows에서 실행할 수 없도록 Apple Silicon macOS 호스트 검사를 먼저 수행한다.

## 실제 결과

- 최종 실행 session `60482`, 아래 `cargo run --offline --release` 명령 exit `0`.
- **15/15 case 통과**, 거부 요청 각각 직후 Finder 설정 5개 및 격리 저널·`-wal`·`-shm` 불변 **12/12**.
- Windows 조회는 `supported:false`, `platform:"macos"`, 설정·대상 목록 모두 빈 배열, OS 메타데이터 null을 반환했다.
- 로컬 main의 `folder_options` 요청과 알 수 없는 ID는 OS-first 검사에서 거부됐다. secondary 창은 생산 main-window guard에서, remote main은 Tauri ACL에서 거부됐다.
- 기존 Mac 거부 8건도 다시 통과했다. canary가 조회된 허용 목록에 없음을 확인한 뒤에만 요청하며, 복원 ID는 유효한 hex가 아니다.
- 실행 전후 생산 모듈 5개와 임시 사본의 `cmp` 일치 및 SHA256을 확인했다. 임시 lock의 registry 의존성 439개 모두 저장소 lock과 버전·checksum이 일치했다. 실행은 offline이며 의존성을 추가하지 않았다.
- 독립 하네스에서 사용하지 않는 Mac AI inspection helper(`gate`, `inspect_mac_settings`)에 한정된 dead-code 경고 2개가 있었다. 이를 숨기기 위해 생산 소스를 변경하지 않았다.

| 확인 범위 | case 수 | 결과 |
|---|---:|---|
| 실제 Finder 설정 5개 조회·빈 이력·저널 불생성 | 1 | PASS |
| Mac에서 Windows 조회 unsupported·빈 목록 | 1 | PASS |
| Windows 연결: 로컬 OS 차단·secondary·unknown ID·remote ACL | 4 | PASS |
| Mac invalid canary·창 출처·복원 ID·요청 schema·remote ACL | 8 | PASS |
| 마지막 Finder 5개·저널·sidecar 상태 불변 | 1 | PASS |

Windows native adapter는 이 호스트에서 `#[cfg(windows)]`로 제외된다. 성공한 unsupported 조회는 reader가 호출되면 panic하는 `UnsupportedReader`를 통과했고, 로컬 연결 응답은 launcher closure보다 앞선 OS-first 거부 문구였다. 따라서 Windows reader/launcher 호출 0은 이 실제 응답과 플랫폼 분기 소스로 확인한 것이며, 생산 코드에 계측 counter를 삽입한 결과는 아니다.

## 증거

- `native-ipc-result.jsonl`: 실제 stdout JSON 28줄을 값 변경 없이 보존했다. case 15줄, 요청별 무변경 관찰 12줄, 최종 summary 1줄이다. 컴파일 로그를 성공 결과로 변환하거나 mock 응답을 넣지 않았다.
- `native-ipc-probe.rs.txt`: 실행한 최종 하네스 원문. Wry runtime과 기본 Builder를 사용한다. `mock_context`는 독립 config/assets/ACL container만 제공하며 MockRuntime·mock invoke를 사용하지 않는다.
- `native-ipc-Cargo.toml.txt`: 독립 crate manifest. 제품 manifest에는 추가하지 않았다.

| 대상 | SHA256 |
|---|---|
| `apps/desktop/src-tauri/src/mac_settings.rs` | `16b895f5ec44930946fad950a55be908deeab2d82651056dec3b7da68435efe3` |
| `apps/desktop/src-tauri/src/mac_settings/native.rs` | `be8948495ba35c13d0174685301eaddd3c03cffed6a84355359c3301c866c38a` |
| `apps/desktop/src-tauri/src/windows_settings.rs` | `0a4f90ce33b5faa2e36aebc0ec8fa653aca6588e17306f1f7dd75a2509aaadc6` |
| `apps/desktop/src-tauri/src/windows_settings/model.rs` | `7d1c848db67deb99c022547c66df5c108349b4017db89d6a25a947caddc404f3` |
| `apps/desktop/src-tauri/src/windows_settings/native.rs` | `663e8d82c44225b2fe89a0f20c2a93a54cafb7dccd648d2e289c4c2c74b12d8f` |
| `native-ipc-probe.rs.txt` | `6ed9a50979864765338bc4a0fd25a452b6606ca08511d9ca28145be3be54ae5f` |
| `native-ipc-Cargo.toml.txt` | `d5aa966231c8a9153dbf368d67dda1ba7c5eb2c4787398cdb7fdba326d826b30` |
| `native-ipc-result.jsonl` | `990813b6137049d49e8082c23c1af9cb057554179791cf9c7bbb752c8190c2c0` |

## 실행 및 재현

실제 사용한 임시 crate는 `/private/tmp/broomsweepy-windows-ipc-probe-TrE8WY`이며 기존 프로젝트 target을 재사용했다. 공유 target lock 대기를 허용해 다른 Rust 검증과 동시에 컴파일하지 않았다.

```sh
CARGO_BUILD_JOBS=1 \
CARGO_TARGET_DIR=/Users/dannysmacair/Documents/git/BroomSweepy/target \
cargo run --offline --release \
  --manifest-path /private/tmp/broomsweepy-windows-ipc-probe-TrE8WY/Cargo.toml
```

새 실행은 프로젝트 루트에서 고유 임시 디렉터리를 만들고 원래 child-module 구조를 유지해 다음처럼 배치한다. 각 생산 사본이 위 지문 또는 새로 확인한 실행 대상과 byte-identical인지 먼저 확인한다.

```text
<fresh-temp>/Cargo.toml                    ← native-ipc-Cargo.toml.txt
<fresh-temp>/Cargo.lock                    ← 프로젝트 Cargo.lock
<fresh-temp>/src/main.rs                   ← native-ipc-probe.rs.txt
<fresh-temp>/src/mac_settings.rs           ← 생산 파일 그대로 복사
<fresh-temp>/src/mac_settings/native.rs    ← 생산 파일 그대로 복사
<fresh-temp>/src/windows_settings.rs       ← 생산 파일 그대로 복사
<fresh-temp>/src/windows_settings/model.rs ← 생산 파일 그대로 복사
<fresh-temp>/src/windows_settings/native.rs ← 생산 파일 그대로 복사
```

별도 nonce app identifier로 제품 저널과 분리한다. 프로세스 watchdog은 120초, 각 IPC·페이지 로드는 최대 10초이며 remote 검사에는 임시 loopback 서버만 사용한다. 실패해도 설정을 자동 복원하거나 정상 변경 요청으로 전환하지 않는다. 종료 시점 임시 crate는 236 KiB, 전용 release 실행 파일은 약 11 MiB였다. 공유 dependency cache·사용자 자료는 정리하지 않았다.

## 확인하지 않은 범위

- Windows 실기 레지스트리 조회·설정 페이지 실행·설치형 전체 검증: **NOT RUN**.
- Mac 정상 Apply/Restore 성공, 일반 사용자 저널/승인·채팅 전체 흐름: 이 하네스에서는 **NOT RUN**.
- Mac에서 unknown target은 OS 검사에서 먼저 차단되므로 Windows allowlist 파싱 실기 검증으로 해석하지 않는다.
- 전체 제품 setup/CSP/capability 구성을 로드한 것이 아니므로 설치 앱 전체 보안·기능 PASS를 뜻하지 않는다.
- 무변경 관찰은 고정 Finder 설정 5개와 분리 저널·sidecar에 한정된다. 전체 OS·파일시스템 무변경을 보장하지 않는다. WebView의 자체 런타임 캐시 생성과 사용자 자료 수정은 구분한다.

#tags: windows-settings, macos-settings, native-ipc, wry, readonly, negative-test
