# 채팅 권한·전체 로컬 접근 개발

## 사용자 요구 — 턴 시각 미확인

사용자: “채팅에서 전체 사용 엑세스같은 기능도 있어야겠는데?” → “진행”.
Product Design 설치를 승인했고 설치 이후 “ㄱㅖ속”으로 개발을 재개했다.
원본 사용자 턴 시각/세션 UUID는 이번 전달 컨텍스트에 없어 추정하지 않는다.

## 10:49 KST — 개발 결정과 경계

입력창과 설정에 조회만/확인 후 실행/전체 접근을 연결한다. 기본 confirm, 전체 접근은
사용자가 실제 앱에서 직접 선택해야 한다. 개발 중 실제 앱 권한을 켜지 않았다.
native 전체 접근만 로컬 볼륨 opaque ID·bounded 이름 검색을 허용하며 외부 MCP는 기존
선택 폴더 범위를 유지한다. OS 전체 디스크 접근, 문서 본문 공개, 영구 삭제는 별도다.
기존 확인 생략 flag를 full_access로 자동 승격하지 않고 explicit confirm은 항상 확인한다.
확인된 UI 변경은 실제 디스크 작업·AI 전송이 없는 합성 fixture로 검증했다.

## 검증 근거와 열린 범위

프런트엔드126테스트, TypeScript, production frontend build PASS. 저장 실패/지연,
390px/1280px·4언어·키보드·전체 검색→대상 열기·조회 전용 차단을 실제 브라우저에서 확인했다.
Rust 검증은 [QA](../docs/qa/2026-10-11-chat-access.md)에 최종 결과를 기록한다.
독립 소스 리뷰의 cleanup 실행 취소 누락 및 최종 provider 응답의 epoch 재검사 누락은 수정 확인했다.
설치본 교체·실제 CLI 종단·사용자 파일 삭제·Windows 런타임은 이번 작업에서 실행하지 않았다.

#tags: 채팅권한, native-access, bounded-search, epoch, arch:015

## 11:22 KST — 사용자 설치 요청과 실제 설치 확인

사용자: “설치하자”. 현재 worktree를 ARM64 release로 빌드하여 `/Applications/BroomSweepy.app`을
교체하고 실행했다. 원래 앱과 첫 설치본은 `/private/tmp/broomsweepy-install-backup-jVoGhr/`에 보관했다.
대화/권한 SQLite는 설치 전후 및 첫 실행/정상 종료 후 동일 hash였고, 현재 confirm/remember·대화38개가 복원됐다.
모드와 권한은 바꾸지 않았고 실제 질문 전송/사용자 파일 작업은 하지 않았다.

첫 native 설치에서 모델·권한 popup이 AX에는 열리지만 화면에 안 보이는 회귀를 찾았다.
`assistant-dock` z-index1 한정 변경 후 재빌드/재설치하여 두 popup 실제 표시·Escape 복귀를 확인했다.
합성 브라우저/AX 성공만으로 native 픽셀 표시를 보장하지 않는 사례다.
현재 Codex CLI 준비됨·저장 모델/강도 표시는 확인했으나 서버 인증/질문 종단은 NOT RUN이다.
근거: [실제 설치 QA](../docs/qa/2026-10-11-local-install.md).

#tags: 설치, webkit-popup, actual-render, data-preservation, arch:015
