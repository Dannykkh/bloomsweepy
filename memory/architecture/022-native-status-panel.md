# 단일 네이티브 유리 상태 패널

status: SUPERSEDED
superseded-by: [[023-clear-panel-dismissal]]
supersedes: [[021-native-popover]]
date: 2026-10-21
source: codex
tags: macos-menu-bar, native-glass, status-panel, local-event-lifecycle, screenshot-evidence
evidence: conversations/2026-10-21-menu-bar-popover.md#user-screen-feedback (실제 사용자 턴 시각·세션 ID 미제공; 사용자 확인과 첨부 화면은2026-10-21)
alternatives: standard-popover-appearance — DarkAqua/가벼운 글자 중간 설치본의 실제 화면이 여전히 진한 회색 판이라 기각; extra44percent-tint — 더 어두운 덧칠은 반대 방향이라 빌드만 하고 설치하지 않음; whole-window-alpha — 글자/컨트롤까지 흐려지므로 기각; second-blur-or-WebView — 단일 네이티브 재질로 충분하고 추가 비용 불필요; global-input-monitor — own-app 알림+표시 중 local mouse-down만 사용해 전역 수집을 늘리지 않음.
depends-on: [[007-native-window-material]]
sources: DESIGN.md; docs/design-refs/2026-09-07-experience-macos-menu-bar.md; docs/qa/menu-bar-2026-10-21/review.md; https://developer.apple.com/documentation/appkit/nsvisualeffectview; https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/EventOverview/MonitoringEvents/MonitoringEvents.html
files: apps/desktop/src-tauri/src/menu_bar/macos.rs, apps/desktop/src-tauri/src/menu_bar/status_panel.rs, apps/desktop/src-tauri/Cargo.toml
reopen-when: 실제 데스크톱에서 대비/재질 불만, 표시 중 monitor 정리/아이콘 toggle/다중 모니터 위치/앱 전환 잔류 회귀, 테마 선택 추가, AppKit 동작 변경 시 재검증한다.
last_verified: 2026-10-21 (release-library Clippy/-D warnings·위치6tests·ARM64 production4m30s·서명·DB3개 보존 PASS. 실제 설치형 Settings열기/Esc/메인복귀/종료/재실행 확인. 최종 직접아이콘/물리적외부클릭/desktop합성 질감은 사용자 확인 대기, 모든키보드·접근성 검증으로 확대하지 않음.)

대체 이유: 사용자가 실제 새 패널에서 “바깥클릭 안닫힘 여전히 회색판”으로 두 수락 기준 모두 실패를 확인했다. 설치형을 정상 종료하고 닫힘이 확인된 중간 NSPopover로 복원했으며3개DB/서명을 재검증했다. 단위테스트/Clippy/Esc/Open PASS가 물리적 외부 클릭·재질을 보장하지 않았다. 아래는 거절된 구현의 기록이며023이 재질과 비활성 앱 외부 클릭의 구멍을 변경한다.

기존 NSStatusItem과10초 집계 sampler는 유지하고 상태 UI만 한 NSPanel로 옮긴다. 창은 clear/nonopaque/shadow, 콘텐츠는 한 UnderWindowBackground/BehindWindow/Active NSVisualEffectView다. DarkAqua·16pt medium 제목·15pt regular 수치·14pt 의미색 라벨과320×370 배치를 보존한다. tint와 전체 창 alpha는 넣지 않는다. Native material의 접근성 fallback은 OS가 소유하며 시스템 설정을 변경하지 않는다. 창 단독 캡처는 데스크톱 배경 합성 질감의 완전한 증거가 아니다.

패널은 key가 될 수 있으나 main은 되지 않는다. 열 때 앱 활성화(14+ API와 구버전 호환 분기)를 요청하고 아이콘 아래에 visibleFrame 내 위치시킨다. Esc는 orderOut하며, 자기 앱 외부 클릭은 표시 중 local mouse-down monitor, 다른 앱 전환은 own NSApplication did-resign-active로 닫는다. 패널/상태 버튼 window 번호는 local dismiss에서 제외하고 원본 클릭을 항상 전달한다. 앱 활성화 거절/비활성화 없는 외부 메뉴까지 모든 바깥 입력을 보장하지 않는다.

한 panel·한 local token을 재사용한다. 닫을 때 token을 take한 뒤 borrow를 풀고 removeMonitor하여 동기 재진입을 피한다. token→block→panel retain cycle은 close에서 끊고, NativeMenuBar Drop은 observer제거→panel.close→status제거 순서다. callback은 shared UI RefCell을 다시 빌리지 않는다. 새 WebView·파일/프로세스 목록·AI호출·전역 입력/키 감시·사용 이력은 없다. 공개v1.8.0 설치파일/태그는 이번 로컬 수정으로 교체하지 않는다.
