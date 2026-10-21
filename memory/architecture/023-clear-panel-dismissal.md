# Clear 재질과 비활성 앱 외부 클릭 닫힘

status: SUPERSEDED
superseded-by: [[024-legible-status-glass]]
supersedes: [[022-native-status-panel]]
date: 2026-10-21
source: codex
tags: macos26, clear-glass, outside-click, inactive-app, no-input-history
evidence: conversations/2026-10-21-menu-bar-popover.md#new-panel-rejection-and-recovery (사용자 턴 시각·세션 ID 미제공, 같은 날 명시적인 실제 실패 확인)
alternatives: local-plus-resign-only — 처음부터 inactive인 앱에서는 외부 클릭에 새 resign이 없어 실제 실패; UnderWindowBackground-only — 실제 사용자 화면에서 여전히 회색 판; whole-window-alpha/nested-blur/tint — 글자 흐림·불투명도·비용을 더하므로 기각; global-key-monitor — 접근성 신뢰와 범위를 늘리고 필요 없어 제외.
depends-on: [[007-native-window-material]]
sources: docs/qa/menu-bar-2026-10-21/review.md; https://developer.apple.com/documentation/appkit/nsglasseffectview; https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/EventOverview/MonitoringEvents/MonitoringEvents.html
files: apps/desktop/src-tauri/src/menu_bar/status_panel.rs, apps/desktop/src-tauri/src/menu_bar/macos.rs, apps/desktop/src-tauri/src/menu_bar.rs, apps/desktop/src-tauri/Cargo.toml
reopen-when: 실제 화면/외부클릭 재실패, 권한 요구, 비동기 닫힘 역전, 새 theme/OS/SDK, pre26의 재질 요구 변경 시 다시 검증한다.
last_verified: 2026-10-21 (공식SDK26.5/현재AppKit0.3.2 API확인, Clippy·실제모듈8tests PASS; Clear 후보 production/실화면 수락은 QA 별도 기록, 정적 API로 성공 추정 금지)

대체 이유: Clear 빌드4m28s 설치/서명/DB3개보존 및 실제 외부 Chrome 창 클릭 닫힘·Esc·Open·Quit·재실행은 통과했다. 그러나 사용자가 “너무 투명해서 글자가 잘 안 보임”으로 가독성 실패를 확인했다.024는 Clear만 Regular로 교체하며023의 표시중 마우스 처리/세대/오류전파는 그대로 계승한다. Clear가 회색 판 문제를 제거한 것이 이 텍스트 UI의 수락을 뜻하지 않는다.

320×370/배치/집계는 유지한다. macOS26+는 plain NSView content를 한 NSGlassEffectView.Clear의 contentView로 넣고 tintColor=None, cornerRadius16을 쓴다. 이전 OS는 한 UnderWindowBackground effect로만 fallback하며 같은 투명함을 약속하지 않는다. OS 접근성·창 전체 alpha는 바꾸지 않는다.

보이는 동안만 local/global left/right/otherMouseDown을 관찰한다. 전역 callback은 event 인자를 전혀 읽거나 저장하지 않고 닫기만 한다. 키 이벤트·좌표·앱정보·사용이력은 수집하지 않는다. local은 패널/상태버튼을 제외하고 원본 이벤트를 전달하며 own-app resign observer는 CmdTab을 보완한다. close는 세대를 무효화하고 두 token을 take/borrow해제 후 unregister한다. 이전 세대 callback은 재개방 창을 닫지 못한다.

monitor 등록 실패는 창을 닫고 Result 오류를 반환하며 설정→열기는 기존 비동기 설정과 같은 spawn_blocking/timeout 패턴으로 실제 실행 결과를 기다린다. 새 권한 요구를 자동 승인하거나 OS 설정으로 해결하지 않는다. 공개 릴리스는 변경하지 않는다. 원래 파일/키 개인정보 범위를 확대하지 않는 닫힘 전용 구현이며, 문서상 마우스 monitor와 키의 접근성 요구는 구별한다. 실제 권한/클릭 도착 여부는 이 Mac 검증이 필요하다.
