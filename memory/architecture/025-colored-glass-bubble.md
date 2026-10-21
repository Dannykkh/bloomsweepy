# 색상과 꼭짓점을 복원한 유리 말풍선

status: CURRENT
supersedes: [[024-legible-status-glass]]
date: 2026-10-21
source: codex
tags: macos-menu-bar, glass-bubble, persistent-color, background-alpha
evidence: conversations/2026-10-21-menu-bar-popover.md#colored-bubble-acceptance (세션 ID·원래 턴 시각 미제공; 사용자 실제 첨부와 네이티브 미리보기 응답)
alternatives: whole-window-alpha — 글자와 조작부까지 흐려져 제외; Clear — 사용자 가독성 실패; plain-Regular — 실제 회색·무색·꼭짓점 소실로 기각; private-popover-view-patching — 비공개 내부 구조에 의존하므로 제외. 일반 Regular로 복귀하려면 사용자 방향 변경/호환성 실패 근거가 필요하다.
depends-on: [[007-native-window-material]]
sources: docs/design-refs/2026-09-07-experience-macos-menu-bar.md; docs/qa/menu-bar-2026-10-21/review.md; https://developer.apple.com/documentation/appkit/nsview/alphavalue
files: apps/desktop/src-tauri/src/menu_bar/macos.rs, apps/desktop/src-tauri/src/menu_bar/surface.rs, apps/desktop/src-tauri/src/menu_bar/gauge.rs, apps/desktop/src-tauri/src/menu_bar/status_panel.rs
reopen-when: 다른 배경/OS에서 가독성·재질·꼭짓점·inactive색·닫힘 실패, 접근성 옵션의 열린 중 실시간 반영 요구가 생기면 재검증한다.
last_verified: 2026-10-21 (실제 소스 컴포넌트 미리보기 사용자 수락·AX progress값·12tests·release Clippy·combined production4m50s/서명 PASS; Mac잠금으로 설치/실제앱재검증 대기)

320×370 본문은 보존하고 상단14pt 꼭짓점을 더한다. 단일 Regular 배경(empty contentView)의 alpha만0.82, 컨트롤은 sibling/full-opacity다. root CAShapeLayer 말풍선 mask와 미세 테두리, 화면 clamp 후 icon center에 맞는 beak을 사용한다. mask clipping이 네이티브 굴절 광학까지 다시 계산한다고 주장하지 않는다. Reduce Transparency는 열 때 재조회해alpha1이며 OS설정을 바꾸지 않는다. 구버전 단일 UWB fallback은 동일재질을 약속하지 않는다.

NSProgressIndicator의 inactive 무색화를 피하려고 read-only NativeGauge를 사용한다. RAM blue–cyan/CPU violet–blue/disk teal,6pt track와 정확한 수치·AX0–100값을 제공한다. 타이머·애니메이션·추가WebView·시스템스캔이 늘지 않으며 기존10초 단일 집계와 표시중 local/global close-only·generation·오류전파는 유지한다.
