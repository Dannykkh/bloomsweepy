# 읽을 수 있는 표준 Liquid Glass 요약창

status: SUPERSEDED
superseded-by: [[025-colored-glass-bubble]]
supersedes: [[023-clear-panel-dismissal]]
date: 2026-10-21
source: codex
tags: liquid-glass, readability, regular-material, macos-menu-bar, outside-click
evidence: conversations/2026-10-21-menu-bar-popover.md#clear-legibility-feedback (사용자 턴 시각·세션 ID 미제공; 같은 날 실제 Clear 화면의 가독성 실패)
alternatives: Clear — 실제 사용자 화면에서 너무 투명해 글자를 못 읽음; Clear-plus-fixed-dimming — 다시 고정된 진한 판을 만들거나 배경별 대비를 놓치므로 제외; UWB-only — 실제 회색 판 실패; whole-window-alpha — 컨트롤까지 흐려지므로 제외.
depends-on: [[007-native-window-material]]
sources: https://developer.apple.com/design/human-interface-guidelines/materials; docs/qa/menu-bar-2026-10-21/review.md; docs/design-refs/2026-09-07-experience-macos-menu-bar.md
files: apps/desktop/src-tauri/src/menu_bar/macos.rs, apps/desktop/src-tauri/src/menu_bar/status_panel.rs, apps/desktop/src-tauri/src/menu_bar.rs, apps/desktop/src-tauri/Cargo.toml
reopen-when: 실제 밝거나 복잡한 배경에서 가독성/재질 실패, 외부클릭/재클릭 회귀, pre26 요구·테마·SDK변경 시 재검증한다.
last_verified: 2026-10-21 (Regular Clippy/-D warnings·production7m29s·ARM64서명·DB3개보존·실제Settings열기/글자노출확인. 동일023모니터8tests/Clear외부좌표clickPASS는구분. Regular외부자동click은불확정, 실제가독성/물리적외부click사용자확인대기.)

macOS26+에서 한 NSGlassEffectView.Regular를 plain contentView의 바탕으로 사용한다. tintColor=None, cornerRadius16, 창 clear/nonopaque이며 컨트롤이나 창 전체 alpha를 낮추지 않는다. Apple HIG는 글자 많은 팝오버에는 Regular, 풍부한 미디어 위의 높은 투과도는 Clear로 구분한다. 사용자 확인은 이 선택을 실제 배경에서 다시 검증하는 근거이며 문서만으로 PASS하지 않는다. pre26은 기존 단일 native effect fallback이다.

대체 이유: 사용자가 실제 Regular 화면의 무색 게이지·꼭짓점 소실·밋밋한 회색 배경을 지적했다.025는 같은 재질을 배경-only alpha로 분리하고 말풍선·항상 색이 유지되는 게이지를 복원한다. 전체 창/글자 alpha 금지는 유지한다.

023의 표시중 local/global mouse-down close-only·세대 격리·등록오류 전파와 기존 NSApplication observer를 그대로 유지한다. 전역 키·좌표·앱정보·이력을 읽거나 저장하지 않는다. CPU/RAM/disk집계·UI배치·설정·대화DB·Windows는 이 재질 한 줄 수정으로 바꾸지 않는다. 공개 릴리스/커밋/푸시는 별도 요청 전 수행하지 않는다.
