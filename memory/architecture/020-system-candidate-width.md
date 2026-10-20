# 시스템 후보의 명시적 폭 경계

status: CURRENT
date: 2026-10-21
source: codex
tags: system-candidates, inline-size, grid-width, platform-copy
evidence: conversations/2026-10-21-system-candidates.md#user-request (원래 턴 시각 미제공; 2026-10-21 요청 범위 정리)
alternatives: global-view-stack-resize — 다른 화면에 영향을 줌; new-card-redesign — 기존 화면의 intrinsic 폭 결함을 고치는 데 불필요; hidden-warning — 부분 결과를 완전한 결과로 오인할 수 있어 기각.
depends-on: [[016-macos-management]], [[017-workspace-navigation]]
sources: DESIGN.md; docs/qa/system-candidates-2026-10-21/review.md
files: apps/desktop/src/App.tsx, apps/desktop/src/App.css, apps/desktop/src/views/CleanupView.tsx, apps/desktop/src/views/CleanupView.css, apps/desktop/src/system-cleanup-fixture.tsx
reopen-when: 새 cleanup source·행 액션 추가나 실제 native 폭/가로넘침 회귀가 관찰되면 동일 조합과 container breakpoint에서 재검토한다.
last_verified: 2026-10-21 (설치 전 읽기 전용 결함 재현; 수정 후 합성12상태·frontend192/192·TypeScript·production build PASS. 새 native 화면/Windows 실기/삭제 NOT RUN)

중첩 grid 안에서 inline-size query container의 intrinsic 폭에 의존하지 않는다. cleanup workspace와 CleanupView에 width100%·min-width0·단일 minmax track을 명시하고 전역 view-stack은 바꾸지 않는다. 실제 AppShell 조합으로 전체 폭과 행 경계를 검사한다. 시작 검사는 하나만, 긴 경로와 근거는 wrap하고 부분 검사 경고는 선택 전에 표시한다.

Mac 개발 캐시와 Windows 설치 기록 조회를 같은 기능으로 보이게 하지 않는다. 필터·설명을 OS별로 분리하되 기존 선택 payload·개발 캐시 acknowledgement·최종 확인·앱 재검증과 Trash 실행은 유지한다. 합성 fixture는 허용된 가짜 경로 외 IPC를 거부하며 native 삭제 근거가 아니다.
