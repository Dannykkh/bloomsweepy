# 앱 소유 결과와 알림의 렌더 시점 현지화

status: CURRENT
date: 2026-10-21
source: codex
tags: i18n, render-time, app-owned-diagnostics, stale-closure
evidence: conversations/2026-10-21-menu-bar-popover.md#colored-bubble-acceptance (후속 영어 잔존 전수검사와 누락 수정 승인; 정확한 세션 ID·턴 시각 미제공)
alternatives: locale-change-refetch — 검사/CLI/구독을 반복하고 기존 결과를 바꾸므로 제외; global-string-replacement — 파일명·경로·대화를 훼손할 수 있어 제외; translating-all-raw-errors — 출처/의미를 보장할 수 없어 제외; typed-code+params — 장기 목표이나 기존 진단 프로토콜 전체 변경 대신 bounded legacy adapter를 먼저 적용.
depends-on: none — 기존4언어 LanguageProvider 계약의 표시 경계 보강이다.
sources: docs/qa/i18n-2026-10-21/review.md
files: apps/desktop/src/i18n/index.tsx, apps/desktop/src/lib/localizeAppText.ts, apps/desktop/src/App.tsx, apps/desktop/tests/appLanguageSwitch.test.ts, apps/desktop/tests/localizeAppText.test.ts
reopen-when: 미등록 앱 진단이 실제 화면에 남거나 backend가 typed message code+params/provenance를 도입하면 allowlist와 표시 경계를 재검토한다.
last_verified: 2026-10-21 (v1.8.1 Mac/Windows CI 각각frontend218tests·tsc/build/Rust PASS; 실제 Mac 설치/영어 경고·앱 보호 사유·native/HTML/MCP1.8.1 확인, DB3개보존. JA/ZH실화면·모든오류분기는 별도; docs/qa/release-1.8.1-verification.md)

고정 카탈로그1597개의 번역만 정상이어도 backend warning/evidence/message를 그대로 렌더하면 한국어가 남는다. 앱 소유 진단은 useLanguage().appText로 표시한다. 기존 카탈로그의 정확한 문구는 키로 복원해 현재 t를 쓰고, legacy backend는222exact/55anchored template의 순수4언어 adapter(입력16KiB상한)로 처리한다. 미확인 문자열과 캡처한 원문 파라미터는 보존한다.

동적 완료·실패·접근성 알림은 번역문 대신 key+params를 저장해 현재 언어로 렌더한다. 언어 변경 때문에 초기 조회·native listener·스캔·CLI 호출·파일 감시를 재시작하지 않는다. 사용자 파일명/경로/문서 본문/기존 대화·모델 자유 응답·OS 앱/프로세스 이름에는 이 표시 변환을 적용하지 않는다. 파괴적 동작·승인 정책과 메시지 DB 원문은 바꾸지 않는다.
