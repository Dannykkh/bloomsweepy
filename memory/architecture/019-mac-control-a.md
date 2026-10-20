# A형 그룹 설정과 직접 조작

status: CURRENT
date: 2026-10-21
source: codex
tags: mac-control-a, grouped-settings, direct-actions, nonsticky-header
evidence: conversations/2026-10-21-mac-control-a.md#user-request (원래 사용자 턴 시각 미제공; 2026-10-21 결정 기록)
alternatives: B-feature-tiles — 확장자 하나를 과도하게 강조하고 narrow스크롤 증가; C-inspector — 현재5개에 선택단계와narrow조작거리 증가; original-menu-shortcuts — 설정실행처럼 보이지만 타메뉴 이동이라 제거. 설정이 크게 늘어날 때 분류/inspector 재검토.
depends-on: [[007-native-window-material]], [[016-macos-management]], [[017-workspace-navigation]]
sources: docs/design-refs/2026-10-21-experience-mac-control.md; docs/design-refs/2026-10-21-direction-mac-control.md; DESIGN.md
files: apps/desktop/src/views/MacManagementView.tsx, apps/desktop/src/views/MacManagementView.css, apps/desktop/src/components/AppShell.tsx, apps/desktop/src/App.css, apps/desktop/src/App.tsx
reopen-when: 현재5개외 설정 추가·실제사용자 오조작·narrow에서 상태와조작 단절이 관찰되면 그룹/직접조작 구성을 재검토한다.
last_verified: 2026-10-21 (frontend184/184·type/build·Cua합성상태/실제App조합·설치native읽기전용 PASS. 실제Finder쓰기/복원·Windows·장시간·강제reducedmotion NOT RUN; docs/qa/mac-design-review-2026-10-21/review.md)

사용자 ‘추천대로가자’는 추천 A의 실제 제품 반영 선택이다. 기존 AppShell h1만 큰 제목으로 남기고 Mac전용 헤더를 nonsticky로 둔다. 파일표시3개/경로정보2개를 그룹화한다. 끄기/켜기는 초안이고 인접 적용만 native쓰기다. null은 양쪽미선택, 현재값과변경값을 구분하며 상태/오류/복원은 접지 않는다. 보조 설명만 details. 다른메뉴 바로가기는 전역sidebar와 중복하므로 제거한다.

016의OS권한·nullable expectedValue·durable복원·충돌/실패 보호를 바꾸지 않는다. native재질과토큰을 재사용한다. 디자인검증용 실제Finder쓰기·Windows변경은 이 결정 범위밖이다. 최종 실제검증은 docs/qa/mac-design-review-2026-10-21/review.md를 따른다.1.7.0로컬개발본 설치교체·실행·native Mac그룹/기록이동 확인, 닫힌앱DB3개 swap전후동일. 선택·모의쓰기·실제설치읽기전용을구분한다.
