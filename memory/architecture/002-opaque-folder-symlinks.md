# POSIX 일반 폴더 내부 링크는 원본을 따라가지 않는다

status: CURRENT
date: 2026-10-04
source: codex
tags: folder-trash, symlink, no-follow, metadata-fingerprint, macos
supersedes: #nested-link-rejection (memory/architecture.md의 일반 폴더 검토 내부 링크 전면 거부 정책만)
evidence: conversations/2026-10-04-codex-file-management.md#1531-kst--첫-설치-검증; #1539-kst--최종-설치-검증
alternatives: 내부 링크 전면 거부 — 정상 node_modules/.bin이 있는 promo-video의 이동을 막아 탈락; 원본까지 canonicalize/탐색 — 외부/클라우드/순환 원본을 포함하거나 건드릴 수 있어 탈락; Windows까지 동일 허용 — 정션/재분석 지점 native 검증이 없어 보류.
depends-on: [[001-conversational-file-workspace]]
sources: docs/qa/2026-10-04-conversational-files.md
files: crates/bloomsweepy-core/src/actions.rs; apps/desktop/src-tauri/src/folder_actions.rs; apps/desktop/src-tauri/src/assistant_files.rs; apps/desktop/src/components/AssistantFileCard.tsx; apps/desktop/src/components/StorageTreemapPanel.tsx
reopen-when: Windows 정션/링크 지원을 추가할 때; OS Trash가 폴더 내부 원본을 따라가는 사례가 관측될 때; 실행 직전 원자적 no-follow 핸들 기반 이동을 도입할 때.
last_verified: 2026-10-04

선택된 실제 POSIX 폴더 안의 심볼릭 링크는 `symlink_metadata`와 `read_link`로 링크 객체 신원/수정 시각/대상 문자열을 fingerprint한다. 읽기/탐색/용량 합산에는 원본을 포함하지 않고 링크 수를 별도로 표시한다. 깨진 링크·외부 링크·순환 링크·클라우드 대상을 가진 링크도 원본을 열지 않는다.

20,000항목 예산에 링크를 포함하며 검토 후 링크 교체/대상 변경은 실행 직전 검증에서 중단한다. 새 링크 삽입도 변경으로 처리한다. 선택 대상/부모 링크, 클라우드 실제 항목, 마운트 경계, 특수 파일, Windows reparse/junction은 계속 거부한다. 최종 OS 경로 호출의 경쟁을 완전히 제거했다고 주장하지 않는다.

검증: 외부 원본 변화는 계획에 영향을 주지 않고 링크 retargeting은 거부하는 회귀 PASS. 실제 macOS Trash로 합성 폴더 안 링크 자체를 함께 이동하면서 비선택 외부 파일 내용 보존 확인. 실제 promo-video 29링크 포함 계획 성공, 사용자 폴더는 이동하지 않음.
