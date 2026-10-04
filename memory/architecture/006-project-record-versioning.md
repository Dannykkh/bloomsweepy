# 프로젝트 메모리·대화·문서의 Git 공유 경계

status: CURRENT
date: 2026-10-05
source: codex
tags: git-records, project-memory, conversation-history, public-repository, documentation
evidence: conversations/2026-10-05-cleanup-tree.md#메모리대화docs-추가-공유 (사용자 턴 시각·세션 UUID 미제공)
alternatives: 기존 ignore를 유지하고 한 번만 force-add — 신규 기록이 다시 누락돼 다른 컴퓨터의 인계가 불완전해짐; 원시 관찰/중복 인덱스까지 공유 — 기계별 실행 상태·도구 로그를 불필요하게 복제함. 사용자가 일회성 공유나 비공개 기록을 명시하면 추적 범위를 다시 검토한다.
depends-on: none — 제품 실행 구조와 별개인 저장소 기록 정책
sources: .gitignore; docs/handoffs/2026-10-05-000000-cleanup-tree.md
files: .gitignore; MEMORY.md; memory/architecture/index.md; conversations/2026-10-05-cleanup-tree.md
reopen-when: 대화에 실제 인증값/개인자료가 유입되거나 저장소 공개 범위·공유 요구가 바뀌면, 새 커밋/푸시 전에 공개 대상을 재검토한다.
last_verified: 2026-10-05 (GitHub API의 private=false 확인, 현재 텍스트 기록에서 인증키/private 표시/주민번호/전화번호 패턴 일치0; staged MEMORY1/정제 memory11/conversations12/docs51, 원시 JSONL/중복 상태0, Git check-ignore로 로컬 로그 제외 확인)

사용자는 첫 기능 커밋 뒤 “메모리, 대화내역, docs까지 푸시해줘.”라고 명시했다. 기존 MEMORY.md/memory/conversations 전체 제외를 해제하고 정제된 Markdown 기억·대화 사본과 남은 docs를 Git으로 공유한다. 용어집·기존 디자인/QA·문서 스크린샷도 요청된 docs 범위에 포함한다. 기존 제품 아키텍처001–005를 대체하지 않는다.

원시 observations JSONL, .mnemo 파생 색인/오프셋/진료·중복 상태, 운영용 .mnemo-root, OS 메타데이터와 빌드/인증 파일은 로컬에 남긴다. 실제 키/민감 표시가 발견되면 먼저 제외 또는 비식별화를 검토한다. 패턴 검사 통과는 모든 개인정보가 없다는 보장이 아니다. 이 결정은 앞으로 Git에서 추적 가능한 범위를 정하며 자동 커밋/푸시 권한을 부여하지 않는다.
