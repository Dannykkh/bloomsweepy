# 선택적 권한 기억 검증

2026-10-05, Mac ARM64/8GiB, main 개발본. GitHub v1.7.0에 공개한 기능은 아니다.

## 실행 결과

- `cargo check -p bloomsweepy-desktop --lib`: PASS.
- `cargo test -p bloomsweepy-desktop --lib` (jobs1, incremental0): 218 PASS, 0 fail, 3 기존 opt-in native 테스트 ignored. 신규 권한 저장13 + store 복원1을 포함한다.
- `npm run check`, `npm run test:all`: PASS, frontend53.
- `cargo test -p bloomsweepy-core --lib`: 67 PASS/15 FAIL. 모두 색인 시작의 최소2048MiB 디스크 보호장치에 막혔다. 당시 여유1.6GiB. 권한 코드 실패로 위장하거나 보호장치를 우회하지 않았다.
- rustfmt 첫 시도 edition2021은 기존 let-chain 구문으로 실패. 프로젝트에 맞춰 edition2024로 실행 완료.

## 의미 있는 회귀

Session은 DB 없이 현재 실행에만 적용. Remember 네 권한/정확한 루트/설정/승인 시각 roundtrip.
끄기와 Session 전환이 디스크에 반영되며 current-run 허용과 next-run 허용을 구분한다.
없는 폴더 제거 후 재생성되어도 자동 재승인하지 않는다. 변경 링크와 잘못된 설정은 복원
제외. 손상/unknown field/version/과대 record/공유 Unix 파일/DB 링크는 거부한다.
새 허용 저장 실패는 적용하지 않으며 철회 실패는 전용 DB 제거로 과거 허용 복원을 막는다.
제거마저 실패하면 명시 경고와 Remember 상태를 유지해 다음 변경에서도 저장/폐기를 재시도한다. 저장 keys에는 plan/nonce/token/operation이 없고 새 store에는
pending review/실행/operation authority가 생기지 않는다.

## UI 합성 검증

Production Settings와 채팅 popup, 파일 작업/AI 전송0인 격리 fixture 사용.
Session → Remember는 꺼진 권한을 켜지 않음. 저장 오류는 기존 Session 선택 유지 + alert.
1280/760/390 폭에서 가로 overflow 없음, select/buttons44px, Pretendard 실제 로드.
Popup 선택 동일, Escape 닫기 + 원래 연결 버튼 focus 복귀 확인. viewport 원복.
Frontend design 스킬 기준으로 기존 토큰/전체 폭/14px/44px/대비를 유지했다.
설치형 WKWebView의 기본 select 외형이 높이를 줄이는 차이를 발견해 appearance를 명시하고
44px 높이·기존 토큰 기반 화살표를 적용했다. 최종 설치형 화면에서 기존 버튼과 같은
높이로 표시되고 긴 옵션도 잘리지 않는 것을 확인했다.

## 설치형 검증

ARM64 production build(최종2분34초), app bundle, ad-hoc codesign/deep strict 검증과
`/Applications/BroomSweepy.app` 교체 완료. 일반 배포용 notarization은 아니다.

- Remember(모든 허용 OFF) → ⌘Q → inventory running=false → 재실행 → Remember 복원 PASS.
- Session으로 복귀 → ⌘Q → running=false → 재실행 → Session/모든 허용 OFF PASS.
- 위 두 검증 후 최종 보정 빌드 중 사용자가 Remember/시스템·앱 조회ON/정리 검토ON을
  선택한 상태가 관찰됐다. 이 선택을 바꾸지 않고 최종본으로 교체했으며 완전 종료/재실행 후
  두 허용ON과 Remember 복원 PASS. 검색OFF/검사OFF 보존. 에이전트가 두 허용을 켜지 않았다.
- 자동 시작ON, 메뉴 막대 메모리 표시ON, DockerOFF, 한국어 설정 그대로 보존.
- 실제 전용 DB8192bytes/Unix mode0600. 새 앱과 bundle 본체 SHA256 일치:
  `6e438be95fbc4193e09f280575db1aeabacf3e22e07d2ccc290cf913de7612a0`.
- 이전 앱 복원용 백업: `/private/tmp/broomsweepy-permission-install-backup-qhyZno/BroomSweepy.app`.
- 로컬 증거: `docs/ui-audit/screenshots/2026-10-05-permissions-remember-restored-native.png`,
  `docs/ui-audit/screenshots/2026-10-05-permissions-session-final-native.png` (ignored).
  최종본/현재 선택 증거는 `docs/ui-audit/screenshots/2026-10-05-permissions-final-native.png`.

실제 개인 파일 공개 허용은 자동으로 켜지 않는다. 네 grant의 실제 저장/복원은 temp-dir
Rust 테스트로 확인했다. native 앱에서는 먼저 모두 꺼진 상태의 정책 저장/복원을 확인했고,
마지막 설치에서는 사용자가 선택한 시스템 조회ON·정리 검토ON의 재시작 복원도 확인했다.
Windows 런타임·실제 공급자 전송·장시간 자원 검증: NOT RUN — 이번 요청 범위 밖.
Fixture 탭과 직접 시작한 Vite 정리. 현재 선택 그대로 Settings 화면을 남겼다.
최종 자원 확인 때 여유633MiB/swap사용1850MiB. 자기 빌드의 static archive92MiB와
사용하지 않는 dylib0.4MiB만 제거했다(다음 Cargo build로 재생성 가능). 설치 앱/bundle,
rlib 캐시/백업은 보존. 사용자 파일/색인/기존 대화 삭제 없음, 빌드 폴더 전체 정리 없음.

#tags: permission-tests, local-persistence, fail-closed, arch:009
