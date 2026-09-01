# Handoff: macOS 기능 정합화와 Windows 트레이

## Session Metadata
- Created: 2026-09-01T12:21:48+09:00
- Project: `D:\git\bloomsweepy`
- Branch: `main`

## Current State Summary
Windows Tauri 앱의 트레이 수명주기와 macOS 기존 SwiftUI 앱의 메뉴 막대·알림·정리 안전성을 구현했다. Swift 사용자 파일 이동은 `VerifiedFileMover` 한 곳으로 모아 같은 디스크의 비공개 위치로 배타적 원자 이동하고 identity를 재검증한 뒤에만 휴지통으로 보낸다. 중단 복구 기록과 휴지통 결과 이력을 동기화하며, 재귀 내용을 증명하지 못한 폴더와 앱 번들은 Finder 검토 전용이다. Rust·TypeScript·Swift 정적 검증과 독립 재감사는 통과했으며, 실제 macOS Xcode 빌드만 새 CI의 첫 실행을 기다린다.

## Work Completed
- [x] Windows 트레이 열기·진행 상태·명시적 종료와 창 닫기 시 숨김 구현
- [x] 트레이 생성 실패 시 일반 창으로 계속 실행하는 폴백 구현
- [x] macOS 메뉴 막대 메모리 표시, 알림 설정 재예약, 검사 완료 알림과 창 복원 연결
- [x] macOS 설치 앱 인벤토리를 직접 `.app`의 제한된 `Info.plist` 읽기로 추가
- [x] Swift 정리 작업의 공용 취소 lease와 stale 결과 차단 구현
- [x] Swift 사용자 파일의 원자 staging, identity 재검증, 휴지통 결과 확인, 중단 복구 journal 구현
- [x] 재귀 manifest가 없는 폴더·앱 번들·언어 리소스·이름 기반 의심 항목을 Finder 검토 전용으로 축소
- [x] 실행 중 브라우저의 캐시 포함 모든 개인정보 항목 이동 차단
- [x] 문서·미디어 허용 목록 기반 파일 열기 정책과 회귀 테스트 추가
- [x] macOS Tauri·Swift 앱을 빌드하고 ad-hoc 서명 검증하는 `macos-15` workflow 추가
- [x] Rust 110개 테스트, Clippy, fmt, TypeScript, 파일 정책 4개, Vite 빌드, Swift 정적 정책과 독립 안전 재감사 통과

### Files Modified
| File | Changes |
|------|---------|
| `apps/desktop/src-tauri/src/windows_tray.rs` | Windows 트레이·창 숨김·종료 수명주기 |
| `apps/desktop/src-tauri/src/system_inventory.rs` | 취소·상한이 있는 macOS `.app` 및 Windows 레지스트리 인벤토리 |
| `apps/desktop/src/lib/fileInspectionPolicy.ts` | 허용한 문서·미디어만 직접 여는 정책 |
| `BroomSweepy/Services/VerifiedFileMover.swift` | 원자 staging·journal·휴지통 결과·시작 복구의 단일 파일 이동 경계 |
| `BroomSweepy/Models/ScanModels.swift` | no-follow 파일 identity 스냅샷 |
| `BroomSweepy/ViewModels/CleanerViewModel.swift` | 공용 operation ID·취소 lease·성공 용량 기록 |
| `BroomSweepy/Views/SmartCleanView.swift` | 공용 lease 사용과 위험 후보 기본 제외 |
| `BroomSweepy/Services/FileAccessManager.swift` | 홈·임의 폴더 보안 bookmark 분리와 화면별 release |
| `BroomSweepy/BroomSweepyApp.swift` | 메뉴 막대·알림 진입과 비동기 파일 이동 복구 |
| `.github/workflows/macos-check.yml` | Tauri와 Swift의 macOS 15 빌드·ad-hoc 서명 검증 |
| `project.yml` | 공유 scheme과 Swift 5 언어 모드 |
| `docs/flow-diagrams/macos-runtime-parity.mmd` | macOS 정리·복구 흐름 |
| `docs/flow-diagrams/macos-parity-verification.md` | 구현 근거와 검증 상태 |

### Decisions Made
| Decision | Rationale |
|----------|-----------|
| Windows는 트레이, 기존 macOS SwiftUI 앱은 메뉴 막대 사용 | 각 운영체제의 기존 상주 UX와 수명주기를 유지하기 위해 |
| Swift 사용자 파일 이동을 `VerifiedFileMover` 한 곳에 집중 | 검사·사용 경합과 오류 무시 우회를 한 경계에서 통제하기 위해 |
| 일반 파일만 자동 이동하고 폴더·앱 번들은 Finder 검토 | 디렉터리 stat만으로 스캔 뒤 추가된 하위 파일까지 증명할 수 없기 때문에 |
| CLI/MCP는 검색·검사 명령만 제공하고 휴지통 이동은 앱 내부 확인으로 유지 | 외부 공급자가 파일 삭제 권한을 직접 갖지 않도록 하기 위해 |
| PR CI는 ad-hoc 서명, 배포는 별도 Developer ID·공증 단계 | 비밀 없는 재현 가능한 검증과 실제 배포 신뢰 경계를 분리하기 위해 |

## Pending Work
### Immediate Next Steps
1. 변경을 원격 브랜치에 올린 뒤 `macos-15` workflow에서 XcodeGen 생성, Swift type-check, Tauri·Swift `.app` 빌드와 ad-hoc 서명을 확인한다.
2. 실제 macOS에서 메뉴 막대 표시, 알림 권한·클릭 복원, APFS 휴지통 이동·Finder Put Back·중단 복구를 점검한다.
3. 필요하면 동일 폴더를 두 화면이 공유하는 security-scope lease에 참조 횟수를 추가하고 FileOrganizer 대상 디렉터리를 dirfd 기반으로 강화한다.

### Blockers/Open Questions
- [ ] Windows 호스트에는 `swift`, `xcodebuild`, `xcodegen`, Apple SDK가 없어 실제 Swift type-check를 실행하지 못했다.
- [ ] 새 workflow는 아직 원격에 없으므로 GitHub Actions 실행 결과가 없다.
- [ ] Developer ID 서명·공증·stapling은 배포 자격 증명과 보호 환경을 준비할 때 별도로 구성해야 한다.

## Context for Resuming
### Important Context
작업 트리는 넓게 수정됐고 아직 커밋하지 않았다. `codemap/**` 변경은 병렬 작업 과정에서 생긴 사용자/다른 작업자 변경으로 취급해 이번 마감에서 되돌리거나 정리하지 않았다. 사용자 파일을 직접 휴지통으로 보내는 Swift API는 `VerifiedFileMover.swift`의 검증된 staged path 호출 한 곳만 허용한다. 최종 독립 재감사는 파일 정리 완료 journal 저장 실패의 undo 누락까지 수정한 뒤 P0/P1 없이 GO를 판정했다.

### Potential Gotchas
- `lstat` 재검사 뒤 곧바로 경로 기반 휴지통 API를 호출하는 것만으로는 TOCTOU가 닫히지 않는다. 원자 staging 뒤 identity 재검증이 필요하다.
- `FileManager.trashItem`에는 원본 경로가 아니라 검증된 `stagedRecord.stagedPath`만 전달해야 한다.
- 파일 정리 destination 이동 후 journal 저장 실패 시 원위치 rollback을 시도하고, rollback 불가지만 destination identity가 유지되면 성공과 실제 undo 경로를 반환해야 한다.
- 재귀 manifest 없이 폴더 자동 이동을 다시 켜면 스캔 뒤 추가된 중요 파일까지 함께 이동할 수 있다.
- `SWIFT_VERSION`은 도구체인 버전이 아니라 언어 모드이므로 `5.9`가 아니라 `5.0`을 유지한다.
