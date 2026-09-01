# Handoff: 채팅 CLI 검사 제어 v2와 14px UI

## Session Metadata
- Created: 2026-09-01 09:10:47 +09:00
- Project: D:\git\bloomsweepy
- Branch: main

## Current State Summary
채팅 CLI와 모델 공급자는 판단·요약·명시적 명령만 담당하고, 실행 중인 Tauri 앱이 실제 검색과 저장공간 검사를 수행하는 제어 규격 v2를 구현했다. 앱 대시보드에서 검색과 검사를 별도로 이번 실행에만 허용하며, 검사 시작·상태·정확한 작업 번호 취소까지 외부에 제공한다. 삭제·휴지통 이동은 계속 앱 내부 최종 확인 경계에만 남겼다. 전체 UI 표시 글자는 최소 14 CSS px로 올렸고 실제 Windows 760×600 E2E를 통과했다.

## Work Completed
- [x] 공용 loopback 제어 규격 v2와 JSON CLI/MCP 7개 도구 구현
- [x] Tauri 앱 소유 검사 시작·상태·취소, 결과 snapshot과 React hydration 구현
- [x] 검색/검사 별도 세션 권한과 읽기 전용 대시보드 상태 패널 구현
- [x] 정확한 작업 번호 취소, canonical 경로, 종료 gate, terminal lease, absolute frame deadline 검증
- [x] 전체 사용자 표시 글자 최소 14px와 760×600 세로 스크롤·접근성 검증
- [x] Windows Common-Controls v6 테스트 manifest를 추가해 표준 workspace 테스트 실행 복구

### Files Modified
| File | Changes |
|------|---------|
| `crates/bloomsweepy-control/` | v2 명령·응답·프레임·발견 파일 계약 |
| `apps/bloomsweepy-mcp/` | JSON CLI와 stdio MCP thin client |
| `apps/desktop/src-tauri/src/control_server.rs` | 앱 소유 loopback 서버, 권한, 검색·검사 조합 |
| `apps/desktop/src-tauri/src/lib.rs` | 공용 ScanRuntime, 작업별 취소, report snapshot |
| `apps/desktop/src/App.tsx` | 제어 이벤트 동기화, 결과 반영, 전역 완료 안내 |
| `apps/desktop/src/components/ControlStatusPanel.tsx` | 채팅 검색·검사 별도 승인 대시보드 |
| `apps/desktop/src/App.css`, `DESIGN.md` | 사용자 표시 글자 최소 14px 계약 |
| `apps/desktop/src-tauri/windows-test-manifest.*` | Windows 테스트 Common-Controls v6 활성화 |
| `docs/cli-control.md`, `docs/architecture/cli-control-v2.md` | 사용자·구현 계약 문서 |

### Decisions Made
| Decision | Rationale |
|----------|-----------|
| CLI/provider는 판단·명령, 앱은 실제 I/O | 파일 잠금·취소·보고서·삭제 안전 경계를 한 프로세스에 유지 |
| 검사 시작 명령에 경로·설정 인수 없음 | 앱에서 사용자가 본 폴더와 설정 snapshot만 실행 |
| 외부 삭제 명령 제외 | 정확한 대상 표시와 사용자 최종 확인, OS 휴지통 복구 흐름을 보존 |
| `14pt` 요청을 최소 14 CSS px로 적용 | 14pt는 약 18.7px라 760×600 데이터 도구 정보 구조를 과도하게 축소 |
| 완료 lease를 상태·이벤트 확정 뒤 해제 | 다음 검사가 직전 결과를 terminal 확정 전에 지우는 race 방지 |

## Pending Work
### Immediate Next Steps
1. 사용자가 원하면 현재 변경을 검토해 커밋하고 GitHub에 push한다.
2. macOS APFS 실제 대용량·권한 환경에서 같은 검사와 14px 화면을 별도 검증한다.
3. 외부 휴지통 요청을 추가하려면 앱 안 대상 목록과 최종 확인 UI를 먼저 설계한다.

### Blockers/Open Questions
- [ ] Windows 125%·150% 배율과 macOS WKWebView는 아직 실제 화면 검증이 없다.
- [ ] 설치 프로그램에 `bloomsweepy-mcp`를 함께 배포하는 방식은 아직 결정하지 않았다.

## Context for Resuming
### Important Context
표준 `cargo test --workspace`는 106 passed, 5 ignored, 0 failed이고 `npm run check`, `npm run build`, clippy가 통과했다. 실제 E2E는 네이티브 창 760×600, WebView 744×561에서 기본 거부, 승인, 240ms 시작 응답, 결과 hydration, 키보드 승인 해제·복구, 잘못된 작업 번호 거부, 비대시보드 dock, 정확한 취소와 종료 안내, 실패 `alert`의 오류 상세를 검증했다. 독립 UI/accessibility 재검토도 P0–P3 잔여 없음으로 통과했다. 최종 화면은 `.termsnap/runtime-e2e/screenshots/control-scan-type14-final-760.png`다.

### Potential Gotchas
- `codemap/**` 변경은 병렬 도구가 만든 별도 변경으로 취급해 이번 작업에서 되돌리거나 덮어쓰지 않았다.
- Tauri 시작 직후에는 설정된 기본 창 크기가 늦게 다시 적용될 수 있다. 실제 반응형 E2E는 WebView가 열린 뒤 네이티브 창을 리사이즈하고 client 폭을 확인해야 한다.
- Windows 테스트 manifest는 앱·테스트 대상에 중복 리소스로 들어가지 않게 현재 `build.rs` 조합을 유지한다.
