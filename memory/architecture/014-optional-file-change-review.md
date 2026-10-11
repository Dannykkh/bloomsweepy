# 선택적 파일 변경 비교·검토와 공개 권한 분리

status: CURRENT
date: 2026-10-08
source: codex
tags: file-change-review, optional-monitor, metadata-baseline, skip-not-safe, bounded-runtime
evidence: conversations/2026-10-08-file-change-review.md#현재-요청 (원본 사용자 시각 미제공; 11:57 KST 발췌 기록)
alternatives: 실시간 OS watcher — 고빈도 이벤트·누락 복구·플랫폼별 수명 관리가 필요한 별도 확장으로 보류; 자동 AI/내용 전송 — 사용자의 검토 옵션을 외부 공개 동의로 확대하므로 제외; 자동 삭제/격리 — 감지·검토와 별도 권한/복구 계약이 필요하므로 제외; 지속 baseline/history DB — 현재 재시작 기준과 비영속 bounded 목록이면 충분하여 저장 비용/보존 정책 확대를 피함; 기존 최근 파일 catalog 재사용만 — 수동 refresh 기반이라 수정·이름 변경·제거와 옵션 주기를 충족하지 못함.
depends-on: [[009-opt-in-permission-lifetime]], [[010-selected-folder-inspection]], [[013-common-file-mcp-results]]
sources: docs/architecture/file-change-review.md; docs/qa/2026-10-08-file-change-review.md; docs/design-refs/2026-10-08-experience-file-review.md
files: `crates/bloomsweepy-core/src/file_change_review.rs`; `crates/bloomsweepy-core/src/streaming_walk.rs`; `apps/desktop/src-tauri/src/file_change_review.rs`; `apps/desktop/src-tauri/src/app_tools_file_changes.rs`; `apps/desktop/src/components/FileChangeReviewPanel.tsx`; `crates/bloomsweepy-control/src/capabilities.rs`
reopen-when: 주기 사이 누락이 실제 과업을 막거나 여러 폴더/백신 결과/지속 기록/자동 정리 요구가 생기면 각각 별도 범위·권한·자원 계약으로 재검토한다. 설치형·Windows·장시간 검증 결과가 상한/취소/공개 범위 결함을 드러내도 재검토한다.
last_verified: 2026-10-08 (이 맥 Rust/MCP422·frontend119·합성 UI1280/390px; 신규 설치형/실제 AI 종단/Windows/soak는 NOT RUN)

설정의 기본 OFF 옵션이 선택한 로컬 폴더 하나를 5/15/30분마다 metadata로 비교한다.
첫 완전 검사·설정 재세대·앱 재시작은 baseline이다. 설정만 전용 bounded SQLite에 저장하고
baseline/history는 비영속으로 유지한다. cloud/읽기 오류/신원 실패/자원 상한은 완전 비교로
반영하지 않아 가짜 제거 기록을 만들지 않는다. 모호한 hardlink 이름 변경은 추가·제거다.

검토 필요와 규칙/직접 넘김을 구분하되 넘김은 안전 확정이 아니다. 재변경은 새 ID로 교체한다.
10k 방문/4MiB native paths/협력8초, 최근250개/1MiB, UI/MCP20행을 지킨다. 한 worker와
기존 ScanRuntime lease를 재사용하며 OFF/폴더 변경/종료 시 취소·revision으로 늦은 반영을 막는다.
단일 OS I/O를 강제8초에 끝내거나 시스템 RSS를1MiB로 제한한다는 보장이 아니다.

`files.changes`는 기존 앱 검토 결과의 읽기만 제공한다. AI/외부의 선택 루트는 옵션 폴더와
정확히 같아야 하며 외부 scope epoch와 옵션 revision을 복사 후 재검증한다. 전체/이전 경로,
로컬 오류·내부 ID는 모델에서 제외한다. 폴더 합산 용량은 미측정/null이다.
옵션은 자동 AI·파일 내용 읽기/전송·실행·삭제·격리·OS 알림을 만들지 않는다.

#tags: file-change-review, metadata, privacy-scope, bounded-runtime, arch:014
