# MEMORY.md - 프로젝트 장기기억

## 프로젝트 목표

| 목표 | 상태 |
|------|------|
| 대화형 파일관리 BroomSweepy | 트리 맥 설치 확인; LLM·Windows·장시간 검증 남음 |
| 현재 8 GiB 맥북에서 RAM·디스크 부족 시에도 안전하게 동작 | 자원 보호 소스·합성 회귀 완료, 설치형 전체 계측 남음 |

---

## 키워드 인덱스

| 키워드 | 상세 파일 |
|--------|-----------|
| 기록공유 | [006](memory/architecture/006-project-record-versioning.md) |
| rust, tauri, macos, sidecar, cargo-test | [memory/gotchas.md](memory/gotchas.md) |
| cli-discovery, authentication-status, claude-safe-mode, chat-qa | [memory/gotchas.md](memory/gotchas.md) |
| oauth-expiry, stdout-errors, standalone-cli | [memory/gotchas.md](memory/gotchas.md) |
| codex-install, arm64, cli-0.153.4, chat-success, normal-flow, macos-install, adhoc-signing | [memory/gotchas.md](memory/gotchas.md) |
| response-quality, conversation-restore, chat-header-overlap | [memory/gotchas.md](memory/gotchas.md) |
| app-capabilities, app-owned-results, llm-investigation | [004 앱 기능 정본](memory/architecture/004-app-tool-investigation.md) |
| cleanup-tree, child-exclusion, inherited-selection | [005 정리 후보 트리](memory/architecture/005-cleanup-candidate-tree.md) |
| claude-arm64, rosetta, startup-latency, macos-soak | [memory/gotchas.md](memory/gotchas.md) |
| cloud-exclusion, jwalk, codex-common-flow, scan-safety | [memory/gotchas.md](memory/gotchas.md) |
| release-1.6.0, screenshots, https-push, adhoc-dmg | [memory/gotchas.md](memory/gotchas.md) |
| release-1.6.1, tauri-config, windows-ci, private-api | [memory/gotchas.md](memory/gotchas.md) |
| release-1.7.0, stable-release, hdiutil-fallback, ci-artifacts | [memory/gotchas.md](memory/gotchas.md) |
| native-glass, window-chrome | [007](memory/architecture/007-native-window-material.md) |
| multi-drive, drive-deck, flip-animation, scan-root | [memory/architecture.md](memory/architecture.md) |
| macos, disk-image, hdiutil, volume-filter | [memory/architecture.md](memory/architecture.md) |
| performance-monitor, sysinfo, appkit, graceful-termination | [memory/architecture.md](memory/architecture.md) |
| memory-cleaner, malloc-zone, current-process, one-click | [memory/architecture.md](memory/architecture.md) |
| dual-rings, cpu, ram, scoped-cleanup | [memory/architecture.md](memory/architecture.md) |
| smooth-metrics, css-transition, reduced-motion | [memory/architecture.md](memory/architecture.md) |
| treemap-actions, file-reveal, scan-identity, os-trash | [memory/architecture.md](memory/architecture.md) |
| conversational-file-management, product-direction, ai-tools, readme | [memory/architecture.md](memory/architecture.md) |
| empty-folder-tools, one-shot-plan, structured-envelope, token-privacy | [memory/architecture.md](memory/architecture.md) |
| conversational-files, app-owned-tools, bounded-search | [001 일반 파일 대화](memory/architecture/001-conversational-file-workspace.md) |
| largest-items, shared-treemap, read-only-advice | [003 지도 공유](memory/architecture/003-conversational-storage-map.md) |
| folder-trash, symlink, no-follow, metadata-fingerprint | [002 내부 링크 분리](memory/architecture/002-opaque-folder-symlinks.md) |
| serde-unit-variant, root-link-count, mock-not-live | [memory/gotchas.md](memory/gotchas.md) |
| memory-pressure, unbounded-walker, sqlite-temp-memory, pdf-peak | [memory/gotchas.md](memory/gotchas.md) |
| low-resource-baseline, 8gb-mac, memory-budget, disk-budget | [memory/architecture.md](memory/architecture.md) |
| streaming-walk, document-worker, folder-review, resource-guards | [memory/architecture.md](memory/architecture.md) |
| worker-failure-rollback, os-resource-errors, mount-root-guard | [memory/gotchas.md](memory/gotchas.md) |
| empty-system-trash, irreversible-confirmation, finder, one-shot-nonce | [memory/architecture.md](memory/architecture.md) |
| node-x64, rust-arm64, explicit-target, application-management-pending | [memory/gotchas.md](memory/gotchas.md) |
| application-management, windows-uninstall, related-data, file-open | [memory/architecture.md](memory/architecture.md) |
| release-allocator-probe, allocation-elision, heap-budget | [memory/gotchas.md](memory/gotchas.md) |
| navigation-order, dashboard, performance, applications-nav | [memory/architecture.md](memory/architecture.md) |
| macos-menu-bar, native-popover, resident-window, aggregate-sampler | [memory/architecture.md](memory/architecture.md) |

---

## architecture/
- [memory/architecture.md](memory/architecture.md)
- [memory/architecture/index.md](memory/architecture/index.md)

## patterns/
- [memory/patterns.md](memory/patterns.md)

## tools/
- [memory/tools.md](memory/tools.md)

## gotchas/
- [memory/gotchas.md](memory/gotchas.md)

---

## meta/
- **프로젝트**: BroomSweepy
- **생성일**: 2026-09-04
- **마지막 업데이트**: 2026-10-05
