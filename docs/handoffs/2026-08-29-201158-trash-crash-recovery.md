# Handoff: 휴지통 중단 작업 시작 복구

## Session Metadata
- Created: 2026-08-29 20:11:58 +09:00
- Project: `<repository-root>`
- Branch: main

## Current State Summary
앱이 운영체제 휴지통 이동 직후 비정상 종료되어 JSONL 저널이 `moving` 또는 미완료 상태로 남은 경우를 시작 시 자동 대조한다. Windows와 Freedesktop 환경에서는 원본 경로와 휴지통의 원래 경로·삭제 시각·가능한 파일 크기를 비교하고, 충돌·누락·조회 실패만 사용자 확인 상태로 남긴다. UI는 자동 복원이나 영구 삭제 없이 운영체제 휴지통을 열어 주며, Rust 전체 테스트·실제 Windows 휴지통 대조/복원·WebView2 1280×820 및 760×600 렌더·MSI/NSIS 패키징을 검증했다.

## Work Completed
- [x] 현재 및 직전 회전 저널의 미완료 작업 파싱과 최근 작업 우선 상한
- [x] `planned`, `moving`, `moved`, `failed`, `completed`, `reconciled` 상태 재구성
- [x] 원본 존재·휴지통 원래 경로·±15초 삭제 시각·파일 크기 기반 보수적 판정
- [x] 원본/휴지통 동시 존재, 양쪽 누락, 접근 실패, 휴지통 목록 실패를 직접 확인 상태로 분리
- [x] Windows `\\?\` 및 8.3 짧은 경로와 일반 긴 경로 정규화
- [x] 새 작업 저널에 삭제 전 정규 경로 저장
- [x] 완전 판정 `reconciled` 감사 레코드와 직전 저널 미해결 상태 체크포인트
- [x] 손상·읽기 오류·16 MiB 파일·200개 미완료 상한 초과 시 저널 쓰기 금지
- [x] 시작 시 300ms 지연 상태, 복구 결과 목록, 운영체제 휴지통 열기, 단일 작업 잠금 UI
- [x] 실제 Windows 휴지통 이동→목록 대조→복원 스모크와 테스트 잔여물 0개 확인
- [x] 1280×820 및 최소 760×600 WebView2 렌더 확인
- [x] Rust·TypeScript 전체 검사, 릴리스 실행, MSI·NSIS 번들 생성

### Files Modified
| File | Changes |
|------|---------|
| apps/desktop/src-tauri/src/action_recovery.rs | 저널 파서, OS 휴지통 대조, 감사·체크포인트, 휴지통 열기, 합성·실OS 테스트 |
| apps/desktop/src-tauri/src/trash_actions.rs | 저널 상한 공유, 복구용 정규 경로 기록, 감사 레코드 append 경계 |
| apps/desktop/src-tauri/src/lib.rs | 시작 복구 및 휴지통 열기 Tauri 명령 등록 |
| crates/bloomsweepy-core/src/actions.rs | 검증 항목의 삭제 전 정규 경로 노출 |
| apps/desktop/src/components/RecoveryNotice.tsx | 대조 중·오류·결과·직접 확인 UI |
| apps/desktop/src/{App.tsx,types.ts,lib/bridge.ts} | 시작 검사 상태, IPC 모델, 작업 중복 차단 연결 |
| apps/desktop/src/components/AppShell.tsx | 복구 검사 중 폴더 선택 차단 |
| apps/desktop/src/views/OverviewView.tsx | 외부 작업 잠금에 따른 스캔 버튼 차단 |
| apps/desktop/src/App.css | 기존 글래스 토큰 기반 복구 알림과 760px 반응형 스타일 |
| README.md, docs/architecture/safe-trash-actions.md | 현재 복구 규칙·플랫폼 차이·한계 문서화 |
| MEMORY.md, memory/architecture.md | 시작 저널 대조 설계 결정 인덱스 |

### Decisions Made
| Decision | Rationale |
|----------|-----------|
| 자동 복원·영구 삭제 대신 상태 대조와 OS 휴지통 열기만 제공 | 경로 충돌과 사용자 의도를 앱이 대신 결정하지 않도록 함 |
| 휴지통 경로·시각·크기를 함께 사용하고 ±15초만 허용 | 같은 경로의 오래된 별도 삭제를 현재 작업으로 오인하지 않도록 함 |
| `failed`도 원본과 휴지통을 다시 확인 | OS 호출이 오류를 반환했더라도 실제 이동 여부를 추측하지 않기 위함 |
| 해결된 작업만 `reconciled`, 미해결 이전 저널은 체크포인트 | 반복 알림을 막으면서 다음 회전에도 모호한 상태를 보존 |
| 저널 무결성이 깨지면 어떤 감사 레코드도 쓰지 않음 | 손상 증거를 회전이나 append로 덮지 않도록 함 |
| macOS 원본 누락은 확인 필요로 유지 | 사용 중인 `trash` 라이브러리가 macOS 휴지통 목록 API를 제공하지 않음 |

## Pending Work
### Immediate Next Steps
1. 실제 macOS APFS 앱 번들에서 휴지통 이동·권한·복구 알림을 회귀 테스트한다.
2. 설치본에서 시작 복구 알림·닫기·휴지통 열기를 자동화하는 Tauri/WebView 통합 테스트를 추가한다.
3. 앱 내부 개별 복원 기능은 충돌 정책과 사용자 확인 흐름을 별도 설계한 뒤 결정한다.
4. 이후 문서 내용 검색 인덱스와 유사 사진 탐지를 독립 기능으로 설계한다.

### Blockers/Open Questions
- [ ] 현재 Windows 호스트에는 실제 macOS APFS·샌드박스·공증 환경이 없다.
- [ ] macOS에서 목록 대조 대신 Finder 휴지통 확인만 유지할지 별도 네이티브 어댑터를 만들지 제품 결정이 필요하다.

## Context for Resuming
### Important Context
실제 Windows 복구 스모크는 작은 파일을 휴지통으로 이동한 뒤 같은 원래 경로의 항목을 찾아 복원한다. 이전 테스트가 남겼던 35바이트 `bloomsweepy-trash-smoke-...txt`도 테스트 전용 접두사로 확인해 제거했으며 현재 `bloomsweepy-*-smoke-*` 휴지통 항목은 0개다. UI 진단용 Roaming 앱 데이터는 캡처 후 삭제했다. 렌더 증거는 `.termsnap/design-audit/recovery-notice-runtime.png`와 `.termsnap/design-audit/recovery-notice-760x600.png`다.

최종 번들은 `target/release/bundle/msi/BroomSweepy_0.1.0_x64_en-US.msi`와 `target/release/bundle/nsis/BroomSweepy_0.1.0_x64-setup.exe`다. SHA-256은 MSI `12AAF8B8C98FF1B187F260017FF3F6397819FC774B924E037EA79ACFE4CA70F5`, NSIS `5DD36D48F2567E8DA2CD916E4C3D2EE33B11CA0E93F890F2D69C286B96A1CAB8`다.

### Potential Gotchas
- Windows 휴지통은 저널의 8.3 짧은 경로와 다른 긴 원래 경로를 반환할 수 있으므로 `recovery_match_key` 정규화를 우회하면 안 된다.
- 휴지통 파일 크기는 파일에만 제공되고 폴더는 직접 항목 수이므로 폴더 판정은 경로·시각 근거만 사용한다.
- `reconciled` append가 저널 회전을 일으킬 수 있어 직전 저널의 미해결 작업을 먼저 체크포인트해야 한다.
- 시작 복구는 `ScanRuntime` 단일 잠금을 사용하며 React StrictMode의 이중 effect는 공유 Promise로 중복 IPC를 막는다.
- 저장소 기준선 대부분이 아직 untracked이고 `.gitignore`에는 사용자 변경이 있으므로 일괄 정리·리셋하지 않는다.
- 이 셸의 PATH에는 Cargo가 없어 `%USERPROFILE%\.cargo\bin\cargo.exe`를 직접 사용하거나 명령 범위 PATH를 추가해야 한다.
