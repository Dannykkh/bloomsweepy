# 네이티브 상태 요약 팝오버의 닫힘과 재질

status: SUPERSEDED
superseded-by: [[022-native-status-panel]]
date: 2026-10-21
source: codex
tags: macos-menu-bar, popover-dismissal, native-glass, appkit, observer-lifecycle
evidence: conversations/2026-10-21-menu-bar-popover.md#user-request (원래 사용자 턴 시각 미제공; 2026-10-21 명시 요청)
alternatives: transient-or-focus-only — 기존 Transient가 있었고 중간 설치본도 외부 조작 검증을 통과했다고 할 수 없어 명시적 앱 비활성화 닫힘을 추가; global-input-monitor — AppKit 알림을 먼저 사용하고 전역 입력 수집·소비를 늘리지 않음; second-blur-or-custom-panel — 기존 팝오버가 이미 native effect/위치 배치를 소유하므로 불필요; appearance-only — 실제 중간 캡처의 회색 판 인상이 남아 단일 canvas tint 추가.
depends-on: [[007-native-window-material]]
sources: docs/design-refs/2026-09-07-experience-macos-menu-bar.md; docs/qa/menu-bar-2026-10-21/review.md; https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/EventOverview/MonitoringEvents/MonitoringEvents.html
files: apps/desktop/src-tauri/src/menu_bar/macos.rs, apps/desktop/src-tauri/Cargo.toml, DESIGN.md
reopen-when: 외부 클릭·앱 전환 후 잔류, 아이콘 재클릭 역전, 새 앱 테마 선택, 실행 중 접근성 설정 동기화 요구가 생기면 실제 native 창으로 다시 검증한다.
last_verified: 2026-10-21 (최종 tint/observer 소스 release-library Clippy/-D warnings·형식 PASS; 중간 ARM64 설치·서명·DB 보존 확인. 최종 native 수락 검사는 QA에서 별도 기록.)

대체 이유(2026-10-21): 사용자가 중간 NSPopover에서 실제 바깥 클릭 닫힘은 확인했지만 여전히 진한 회색 판이라고 설명하고 데스크톱 합성 화면을 제공했다. 아래44% tint는 빌드만 했고 설치·수락하지 않았으며 더 짙게 만드는 방향이라 기각했다. 닫힘의 활성화/자기 앱 알림 원칙은022가 계승하고 자동 NSPopover 재질 대신 한 NSPanel의 단일 효과를 사용한다. 아래는 당시 기각된 설계 기록이지 현재 구현 설명이 아니다.

기존 NSStatusItem + NSPopover와 10초 집계 sampler를 유지한다. 열기에서는 앱 활성화(14+의 activate, 구버전 호환 분기) 후 팝오버의 창만 key로 만들며 숨긴 메인 창을 직접 복원하지 않는다. 자기 NSApplication의 did-resign-active를 관찰해 명시적으로 close한다. selector 콜백은 ivars의 Retained<NSPopover>를 사용해 UI RefCell을 다시 빌리지 않으며, NativeMenuBar Drop에서 대상 관찰자를 먼저 제거한다. 버튼 target은 weak이므로 새 retain cycle을 만들지 않는다. 전역 키/마우스 모니터나 이력은 추가하지 않는다.

현재 main의 Dark 테마에 DarkAqua를 맞춘다. AppKit이 이미 제공하는 유리 배경 위에 #090b14 44% 틴트 하나만 두며 별도 blur/WebView/NSPanel로 교체하지 않는다. 16pt medium 제목·15pt regular 수치·14pt 의미색 라벨을 사용한다. 투명도 감소는 native API로 생성 시 읽어 tint를 불투명하게 하고 OS 설정 자체는 바꾸지 않는다. 실행 중 사용자 설정 변경까지 tint alpha가 즉시 재계산된다고 주장하지 않는다.

앱별 자동 캡처/AX 조작은 실제 외부 foreground 클릭이나 배경 합성 질감의 완전한 증거가 아니다. 실제 중간 캡처가 부족하면 정적 API 설명만으로 성공 처리하지 않는다. 기존 상주·저빈도 집계 계약은 memory/architecture.md의 같은 제목 항목에 남아 있다. 공개 v1.8.0 설치 파일은 이 로컬 수정으로 교체하지 않는다.
