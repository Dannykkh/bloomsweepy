# Rust 전용 소스 트리와 Swift 구현 퇴역

status: CURRENT
date: 2026-10-22
source: codex
tags: rust-only, swift-retirement, legacy-ci, history-preservation
evidence: conversations/2026-10-22-swift-retirement.md#user-request (원래 세션 ID·사용자 턴 시각 미제공)
alternatives: keep-dormant-swift — 제품 의존성은 없지만 별도 빌드·CI/문서 혼동이 남아 제외; rewrite-git-history — 공유 태그·과거 복구를 바꾸는 별도 작업이라 제외; prune-unreachable — 미참조 객체도 보존하기 위해 제외.
depends-on: [[007-native-window-material]], [[006-project-record-versioning]]
sources: docs/qa/2026-10-22-swift-retirement.md; docs/archive/swift-legacy/README.md; https://github.com/Dannykkh/bloomsweepy/tree/v1.8.1/BroomSweepy
files: .github/workflows/macos-check.yml; apps/desktop/tests/rustOnlyBuild.test.ts; README.md; DESIGN.md
reopen-when: Swift 독립 배포·유지가 필요하거나, 과거 이력 삭제·저장소 분리로 원격 크기를 줄이는 명시적 요청이 생기면 복원·이력 계약을 재검토한다.
last_verified: 2026-10-22 (frontend220/220·타입·frontend build·CI YAML PASS. 로컬 repack 전후 Git12210객체/ref/reflog/HEAD동일; 이후 정리 커밋은 main을 정상 전진시킨다. 설치Rust앱서명/해시동일; 로컬 정리 시점 새native빌드/원격CI NOT RUN)

현재 `BroomSweepy/`, `project.yml`, 생성된 `BroomSweepy.xcodeproj/`를 제거한다. macOS CI의 XcodeGen·Swift 생성/정책/빌드/서명만 제거하고 Rust/Tauri용 SDK·native/MCP/document-worker 검사와 DMG artifact는 보존한다. SwiftUI에서 계승한 UX와 과거 QA는 역사 참조이며 현재 유지보수 타깃이 아니다. 과거 소스는 변경하지 않은 v1.8.1 태그로 조회·복원한다.

루트의 옛 Swift 계획/section 문서는 `docs/archive/swift-legacy/`로 보관한다.007의 AppKit 재질·접근성 정책이나 Rust 기능/삭제 정책은 바꾸지 않는다. `git repack -ad --keep-unreachable`은 모든 객체를 보존하는 로컬 저장 정리이며 원격 이력을 줄였다는 뜻이 아니다.
