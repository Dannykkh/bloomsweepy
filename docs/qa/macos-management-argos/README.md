# 격리 네이티브 IPC 감리 증거

2026-10-20 · Argos 재감리. 제품 소스/설치 앱을 바꾸거나 실제 Finder 설정을 변경하지 않고 생산 명령 경로의 negative를 검증했다.

## 실제 결과

- 최종 session51486, `cargo run --offline --release`, exit0.
- 기본 TC10/10, 거부8건 각각 직후 설정5개+audit journal 무변경8/8.
- Wry2.11.5의 실제 WebView JS→Tauri IPC. `mock_context`는 테스트용 config/assets/ACL container에만 사용했고 Runtime은 MockRuntime이 아니라 Wry다. Builder도 mock_builder가 아닌 default다. mockIPC/임의 invoke key/remote 권한 강제 부여 없음.
- 첫 조회 정상 성공, main canary가 생산 definition까지 도달, secondary는 생산 main-window guard 거부, remote main은 upstream Tauri ACL 거부.
- source schema의 domain 추가/expectedValue 누락/enabled 문자열 거부 확인.
- 모든 mutation ID는 허용 목록에 없는 값 또는 invalid hex. 정상 OS 변경 성공 테스트는 아님.

파일:

- `native-ipc-results.json`: 실제 stdout JSON만 파싱해 보존한 응답·무변경 관찰.
- `native-ipc-probe.rs.txt`: 최종 실행한 하네스 원문. 제품 모듈을 수정하지 않는다.
- `native-ipc-Cargo.toml.txt`: 사용한 독립 임시 crate manifest. 앱 의존성에 추가하지 않았다.

## 대상 지문

| 생산 소스 | SHA256 (임시 사본과 동일) |
|---|---|
| apps/desktop/src-tauri/src/mac_settings.rs | a19a7ef59a269aec84dc33a81b2cbc970646da5f95b5235d472d826149c64668 |
| apps/desktop/src-tauri/src/mac_settings/native.rs | be8948495ba35c13d0174685301eaddd3c03cffed6a84355359c3301c866c38a |

## 재현 준비

macOS26.4 ARM64, 기존 Rust dependency cache를 사용했다. 프로젝트 루트에서 임시 디렉터리를 새로 만들어 아래 파일을 배치한다.

```text
<fresh-temp>/Cargo.toml             ← native-ipc-Cargo.toml.txt
<fresh-temp>/Cargo.lock             ← 프로젝트 Cargo.lock
<fresh-temp>/src/main.rs            ← native-ipc-probe.rs.txt
<fresh-temp>/src/mac_settings.rs    ← 생산 파일 그대로 복사
<fresh-temp>/src/mac_settings/native.rs ← 생산 파일 그대로 복사
```

파일 hash 일치를 확인한 뒤 기존 프로젝트 target을 재사용해 다음을 실행한다. 경로는 실제 확인한 절대경로로 지정한다.

```sh
CARGO_BUILD_JOBS=2 \
CARGO_TARGET_DIR=/absolute/path/to/BroomSweepy/target \
cargo run --offline --release --manifest-path /absolute/path/to/fresh-temp/Cargo.toml
```

소스 트리의 상위 mac_settings.rs에 외부 `#[path]`만 붙이면 child native.rs 해석이 달라질 수 있어, 위 원래 모듈 구조를 보존해야 한다. 최초 컴파일 실패는 이 테스트 준비 문제였고 제품 수정으로 해결하지 않았다.

120초 프로세스 watchdog, 요청/페이지 로드 각10초, 제한된 loopback 응답만 사용한다. worker가 끝나면 감사 process만 종료한다. 별도 nonce app identifier로 journal 경로를 분리하고, 없던 journal은 생성되지 않아야 한다. 임시 crate와 감사 실행 파일은 증거 보존 후 정리한다. 공유 컴파일 dependency cache는 광범위 삭제하지 않는다.

이번 실행 후 임시 crate184KiB와 감사 package의 실행/중간 파일9개21.9MiB를 정리했다. 위 source/manifest/JSON 증거로 다시 생성할 수 있으며, 기존 제품 실행 파일·설치 앱·사용자 자료·공유 dependency cache는 삭제하지 않았다. 문서에 적힌 `/tmp` 실행 경로는 과거 실행 기록이며 현재 존재하는 파일 링크가 아니다.

## 한계

제품 전체 setup/CSP/capability 정책과 설치 패키지를 실행한 것이 아니다. 실제 정상 Apply/Restore 성공, 실제 기록 소유권/중복/경합 업무 흐름은 NOT RUN이며 기존 fake-store/native-domain 시험과 구분한다. 이 하네스 단독으로 API 보안 전체 PASS나 설치 앱의 모든 기능 정상 판정을 하지 않는다. 상태 관찰 범위는 고정5개 stored settings와 audit journal이며 전체 OS/파일시스템 무변경을 보장하지 않는다.

#tags: 아르고스, 네이티브ipc, wry, 상태불변, 검증하네스, arch:016
