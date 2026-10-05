# macOS 네이티브 제목줄과 단일 글래스 합성

status: CURRENT
date: 2026-10-05
source: codex
tags: native-glass, window-chrome, macos-install, opaque-fallback, low-resource
evidence: conversations/2026-10-05-window-glass.md#현재-요청 (사용자 턴 시각·세션 UUID 미제공; 설치형 검증은 같은 파일의 실제 확인)
alternatives: Transparent 제목줄 — 첫 설치형 렌더에서 안정적인 제목 대비를 확인하지 못해 Visible OS chrome을 선택했다. 실제 배경별 대비가 검증되면 재검토; HTML 가짜 창 버튼 — OS 제어/접근성을 복제할 이유가 없고 플랫폼별 동작을 흐린다; 추가·중첩 CSS blur — 기존 AppKit material 위에서 비용과 불투명도만 더하므로 제외.
depends-on: none — 기존 단일본의 Rust 기본/Swift UI golden master 방향을 유지하는 국소 재질 조정
sources: DESIGN.md; docs/design-refs/2026-09-04-experience-rust-swift-experience.md; docs/design-refs/2026-10-05-impl-log-window-glass.md
files: apps/desktop/src/main.tsx; apps/desktop/src/App.css; apps/desktop/src-tauri/tauri.macos.conf.json; apps/desktop/tests/tauriConfig.test.ts
reopen-when: Tauri/AppKit·배포 flavor가 바뀌거나 MAS/네이티브 material 없는 호스트를 추가할 때 material gate를 재검토한다. 실제 wallpaper·OS 접근성 설정에서 대비/성능 문제가 확인되면 opacity와 fallback을 다시 검증한다.
last_verified: 2026-10-05 (이 맥 ARM64 설치형 Visible 제목/버전/창 버튼·화면 재열기, 프런트51개·config2개·빌드·ad-hoc 서명; Windows 실행 및 장시간 자원 검증은 NOT RUN)

[기존 Rust/Swift 방향](../architecture.md)은 대체하지 않는다. 네이티브 material은 macOS Tauri에서만 사용하고 브라우저와 Windows는 opaque canvas다. main.tsx의 단일 host gate와 CSS 토큰을 사용하며 전체 shell은 투명, body wash는24–44%, native sidebar는28%다. native sidebar/hero는 AppKit과 중복되는 CSS blur를 적용하지 않는다. OS 제목줄은 Visible/hiddenTitle false로 제목·버전·traffic lights를 내용과 분리한다.

reduced-transparency CSS는 opaque fallback을 제공하지만 실제 OS 설정을 변경해 검증하지는 않았다. 이 결정은 모든 wallpaper 대비나 장기 메모리 사용량을 보증하지 않는다. 로컬 설치본은1.7.0 개발본이고 공개 릴리스·Apple 공증을 갱신한 것이 아니다.
