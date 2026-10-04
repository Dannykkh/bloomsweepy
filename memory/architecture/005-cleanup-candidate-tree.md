# 정리 후보 트리의 상속 선택과 삭제 frontier

status: CURRENT
date: 2026-10-05
source: codex
tags: cleanup-tree, inherited-selection, child-exclusion, protected-descendants, bounded-memory
evidence: conversations/2026-10-05-cleanup-tree.md#현재-요청 (사용자 턴 시각·세션 UUID 미제공)
alternatives: assistant browse 상태 재사용 — 탐색 때 선택/revision을 교체하여 트리 선택을 잃음; 프런트 경로로 삭제 대상 추론 — stale/보호/불완전 분기와 승인 권한을 검증하지 못함. 작은 단일 페이지 작업만 요구될 경우 기존 카드 유지가 대안이다.
depends-on: [[001-conversational-file-workspace]], [[003-conversational-storage-map]], [[004-app-tool-investigation]]
sources: docs/plan/cleanup-tree/plan.md; docs/flow-diagrams/cleanup-tree.mmd; docs/design-refs/2026-10-05-experience-cleanup-tree.md; docs/qa/2026-10-05-cleanup-tree.md; docs/qa/2026-10-05-cleanup-tree-native.md
files: apps/desktop/src-tauri/src/cleanup_tree.rs; crates/bloomsweepy-core/src/actions.rs; apps/desktop/src-tauri/src/assistant_files.rs; apps/desktop/src/components/CleanupTreePanel.tsx; apps/desktop/src/App.tsx
reopen-when: 실제 사용에서16가지/2048노드/100실행대상/5분한도가 과업을 막거나 OS별 protected-path 정책 때문에 일반 자료가 과도하게 차단될 때. native 자원 계측에서 범위 안에서도 UI 지연이 확인되면 분기 보관/상태 집계를 재검토한다.
last_verified: 2026-10-05 (Rust320/프런트51 및 이 맥 새 설치본의3대상209B OS Trash·KEEP/보호 marker 해시·취소·재검사·AI 로컬 카드 진입·재실행 기록 유지 통과; Windows·외부 LLM 조사 종단·장시간 자원 검증은 NOT RUN)

하위 제외가 있는 상위 폴더는 삭제 대상으로 만들지 않는다. 서버 include/exclude 상속 규칙으로 선택 frontier를 계산하고 겹치는 부모/자식은 중복 제거한다. 부분 분기의 완전한 하위를 확보하지 못하면 추정 이동 대신 검토를 거부한다.

독립 bounded 트리 상태는 펼침 때문에 지도 generation이나 assistant workspace를 교체하지 않는다. 다른 검사가 source를 교체하면 기존 트리는 stale이며 다시 열어야 한다. 앱의 사용자 확인에서만 단발 계획을 실행한다. 일반 폴더 전체 이동에는 보호 하위 strict audit을 추가하되 기존 시스템 캐시/일반 폴더 기능의 정책은 보존한다.

전체 선택은 확인된 최상위 후보에 적용하며 폴더 체크는 아직 펼치지 않은 하위도 포함한다. 미측정/불완전 크기는0 또는 정확한 값으로 위장하지 않고, 논리 이동 용량은 실제 확보 공간과 구분한다. 최종 결과 뒤에는 무효화된 assistant revision을 반복하지 않고 같은 로컬 폴더를 재검사한다.
