# 작업 중심 메뉴와 상태를 보존하는 설정 분류

status: CURRENT
date: 2026-10-20
source: codex
tags: workspace-navigation, settings-categories, preserve-state, system-cleanup, compact-dashboard
evidence: conversations/2026-10-20-workspace-navigation.md#사용자-요청--기록-2256-kst (22:56 KST 기록; 원래 사용자 턴 시각 미제공)
alternatives: one-long-settings-page — 권한/감시/개발 설정이 혼재되어 찾기 어려움; conditional-mount-panels — 전환마다 비동기 상태와 모델 조회 재시작; generic-cleanup-navigation-forces-system — 폴더 후보 트리 상태를 덮어씀; light/ink-redesign — 현재 glass/큰 드라이브 카드 선호와 불일치.
depends-on: [[007-native-window-material]], [[009-opt-in-permission-lifetime]], [[010-selected-folder-inspection]], [[016-macos-management]]
sources: docs/design-refs/2026-10-20-experience-workspace-navigation.md; docs/design-refs/2026-10-20-direction-workspace-navigation.md; docs/qa/workspace-navigation/verify-report.md
files: apps/desktop/src/components/AppShell.tsx, apps/desktop/src/components/WorkspaceNavigation.css, apps/desktop/src/views/SettingsView.tsx, apps/desktop/src/views/SettingsView.css, apps/desktop/src/views/DashboardView.tsx, apps/desktop/src/views/DashboardView.css, apps/desktop/src/App.tsx
reopen-when: 신규 OS 설정 범위/메뉴가 추가되거나 설정 백그라운드 패널 비용·동시 작업 수가 늘어날 때 mount/polling과 탐색 분류를 다시 검토한다.
last_verified: 2026-10-20 (frontend154/154, type/build, IAB 실제 App 합성 DOM/desktop1280×820/narrow390×844 PASS; 설치형·Windows runtime·강제 reduced-motion 실행 NOT RUN).

컴퓨터(대시보드→성능→앱 관리→Mac 설정), 파일과 공간(공간 정리→파일 찾기), 도구(AI→선택적 Docker), 하단 설정으로 나눈다. Finder 메뉴는 Mac capability로만 노출한다. 하위 화면도 부모의 aria-current를 표시한다.

앱 설정은 일반/검사·검색/AI·권한/파일 감시/개발 도구/정보다. 패널을 한번 mount하고 hidden으로 전환한다. 분류 전환 자체는 저장·권한·CLI 조회를 호출하지 않는다. 기존 설정 callbacks와 native scope/consent 계약을 유지한다.

시스템 캐시 정리 버튼만 명시적으로 mode=system을 선택한다. generic navigate는 mode를 바꾸지 않아 폴더 트리와 generation을 보존한다. 대시보드는 실제 링·FLIP과 엔진을 유지하며 compact CSS로 첫 화면 작업을 드러낸다. 좁은 flex-column에서 wrap과 percentage basis가 새 열을 만들지 않도록 실제 render를 검증한다.
