# macOS 기능 정합화 흐름 검증

- 검증일: 2026-09-01
- 대상 도면: `macos-runtime-parity.mmd`, `desktop-resident-shell.mmd`, `macos-build-verification.mmd`
- 상태: `FULL`은 코드와 현재 환경의 검증 신호가 일치, `PARTIAL`은 구현됐지만 대상 운영체제 실행이 남음, `FUTURE`는 의도적으로 후속 범위

## macOS 메뉴 막대·알림·휴지통 흐름

| 도면 노드·분기 | 구현 근거 | 판정 |
|---|---|---|
| 메뉴 막대 진입, 메모리 표시 설정, 아이콘 전환 | `BroomSweepy/BroomSweepyApp.swift:12,87-99`, `BroomSweepy/Views/SettingsView.swift:119` | PARTIAL — 정적 연결은 완료, 실제 macOS 렌더 확인은 CI·실기기 대기 |
| 팝오버의 메모리·CPU·디스크와 열기·종료 | `BroomSweepy/BroomSweepyApp.swift`의 `MenuBarContent`, `MenuBarMonitor` | PARTIAL — 기존 구현 유지, 실제 메뉴 막대 실행 대기 |
| 알림 끄기→타이머·대기 요청 중지 | `BroomSweepy/Services/HealthMonitor.swift`의 `configureReminderSchedule` | PARTIAL — 설정·예약 경로 정적 확인, macOS 알림 센터 실행 대기 |
| 알림 켜기→권한 확인·요청→현재 주기 재예약 | `HealthMonitor.swift:173,280`, `SettingsView.swift`의 알림 설정 변경 처리 | PARTIAL |
| 정리 시점 알림 24시간 제한 | `HealthMonitor.swift`의 마지막 발송 시각·최근 정리 시각 대조 | PARTIAL |
| 자동 이동 가능 후보→사용자 최종 확인 | `SmartCleanView.swift`, `StorageTreemapView.swift`, `LargeFilesView.swift`, `DuplicateFilesView.swift`, `SimilarImagesView.swift`, `MaintenanceView.swift` | FULL — 항목 수·논리 용량·위험도를 확인한 뒤에만 실행 |
| 자동 이동 근거가 부족한 후보→Finder 검토 | `LanguageCleaner.swift`, `MalwareScanner.swift`, `AppUninstaller.swift`, `BrokenPlistCleaner.swift`, `VerifiedFileMover.swift` | FULL — 앱 언어 리소스, 이름 패턴 의심 항목, 앱 관련 파일, 소유권 불명 plist와 재귀 내용 증명이 없는 폴더는 자동 이동하지 않음 |
| 실행 직전 같은 파일인지 재검사 | `ScanModels.swift`, `VerifiedFileMover.swift`, `CleanerEngine.swift`, 개별 정리 서비스 | FULL — `lstat`의 기기·파일 번호·종류·크기·수정 시각이 같은 일반 파일만 원자적 임시 보관 위치로 옮긴 뒤 동일성을 다시 확인하며, 폴더와 심볼릭 링크는 거부 |
| 승인 루트 자체와 자식 범위 재검사 | `BrokenDownloadCleaner.swift`, `MailAttachmentCleaner.swift`, `BrokenPlistCleaner.swift` | FULL — 승인 루트가 링크가 아닌 같은 디렉터리인지와 후보의 포함 관계를 실행 중 계속 확인 |
| 중복 파일 보관본 확정과 내용 재검증 | `CleanerEngine.swift`의 `trashVerifiedDuplicates`와 스트리밍 SHA-256 | FULL — 경로 정렬로 보관본을 고정하고, 보관본과 선택 복사본의 전체 내용을 이동 직전에 다시 확인 |
| 원자적 임시 보관→OS 휴지통→중단 복구 | `VerifiedFileMover.swift`, `BroomSweepyApp.swift` | FULL — 같은 디스크에서 배타적 원자 이동, 실제 휴지통 결과의 동일성 확인, 디스크 동기화 기록, 시작 시 원래 위치 복구 또는 직접 검토 안내를 한 경계에서 수행 |
| OS 휴지통 이동, 실패 시 원본 복구·보존 | 전체 Swift 소스에서 사용자 파일용 `FileManager.trashItem`은 `VerifiedFileMover.swift` 한 곳뿐이며 `removeItem` 잔여 0건 | FULL |
| 성공 항목의 논리 용량과 항목별 실패만 보고 | `CleanerEngine`, `AppUninstaller`, `PrivacyCleaner`, `MaintenanceManager` 및 대응 화면 | FULL |
| 정리 취소→현재 항목 뒤 중단 | `CleanerViewModel.swift`, `SmartCleanView.swift`의 공용 operation ID·잠금 취소 토큰과 각 이동 루프의 `shouldCancel` 확인 | FULL — 스마트 정리를 포함해 작업자가 실제 종료되기 전에는 새 작업을 시작하지 않음 |
| 실행 중 브라우저의 개인정보 DB 보호 | `PrivacyCleaner.swift`, `PrivacyCleanerView.swift` | FULL — 실행 중인 브라우저는 캐시를 포함한 모든 프로필 항목 이동을 거부 |
| 파일 정리와 되돌리기 | `FileOrganizerEngine.swift`, `FileOrganizerView.swift`, `VerifiedFileMover.swift` | FULL — 승인한 루트와 원본 스냅샷을 다시 확인하고 배타적 원자 이동으로 실제 충돌 해결 경로를 기록하며, 같은 파일일 때만 되돌림 |
| 완료 기록 실패 뒤 이동 결과 보존 | `VerifiedFileMover.swift`, `FileOrganizerEngine.swift` | FULL — 대상 이동 뒤 기록 실패 시 원위치 원자 복구를 시도하고, 복구 불가지만 대상 identity가 유지되면 실제 성공 경로와 경고를 반환해 되돌리기 이력을 보존 |
| 전체 검사 성공 알림과 클릭 후 창·대시보드 복원 | `CleanerViewModel.swift:170`, `HealthMonitor.swift:263`, `BroomSweepyApp.swift:164-180` | PARTIAL — 코드 경로 완료, 완전히 닫힌 WindowGroup 재생성은 실제 macOS 확인 대기 |

## Windows 트레이·macOS 메뉴 막대 상주 흐름

| 도면 노드·분기 | 구현 근거 | 판정 |
|---|---|---|
| Windows 트레이 생성, 열기, 메뉴 상태 | `apps/desktop/src-tauri/src/windows_tray.rs:17-72` | FULL — Windows 컴파일·Rust 테스트·Clippy 통과 |
| 트레이 준비 성공 뒤 창 닫기→숨김 | `windows_tray.rs:96-106`; 설정 실패 시 close handler를 등록하지 않음 | FULL |
| 트레이 준비 실패→일반 창 폴백 | `apps/desktop/src-tauri/src/lib.rs:1020-1024`, `windows_tray.rs:75-87` | FULL |
| 명시적 종료→작업 취소·제어 서버 종료 | `windows_tray.rs:39`, `lib.rs:1063-1071` | FULL |
| macOS 메뉴 막대에서 열기·상주·종료 | `BroomSweepy/BroomSweepyApp.swift:87-136` | PARTIAL — 기존 코드와 새 설정 배선 확인, 실제 macOS 실행 대기 |

## macOS 빌드 검증 흐름

| 도면 노드·분기 | 구현 근거 | 판정 |
|---|---|---|
| Windows 사전 Rust·TypeScript 검사 | 로컬 `cargo fmt`, `cargo test --workspace`, workspace Clippy `-D warnings`, `npm run check`, `npm run build` | FULL — Rust 110개 통과·운영체제 권한 의존 5개 제외 |
| 파일 열기 허용 목록 회귀 검사 | `apps/desktop/tests/fileInspectionPolicy.test.ts`, `npm run test:file-policy` | FULL — 4개 테스트 통과 |
| Swift 정적 파일 작업 정책 | `.github/workflows/macos-check.yml`의 `Check Swift file-operation policy`; 로컬 정적 감사 | FULL — `removeItem`, 오류를 무시한 `trashItem`, 언어·이름 패턴 결과의 자동 이동 0건 |
| Swift 구문 구조 검사 | 55개 Swift 파일의 tree-sitter 기준 버전 대조 | PARTIAL — 새 syntax-error node 증가는 없지만 Apple SDK 기반 type-check를 대신하지 않음 |
| macOS CI에서 공용 검사와 Tauri `.app` 빌드 | `.github/workflows/macos-check.yml:19-95` | PARTIAL — workflow와 YAML 파싱 완료, 첫 GitHub macOS 실행 대기 |
| XcodeGen Swift 앱 빌드·ad-hoc 서명 검증 | `.github/workflows/macos-check.yml:97-140`, `project.yml` shared scheme와 Swift 5 언어 모드 | PARTIAL — Windows에는 Apple SDK가 없어 실행 불가 |
| Developer ID 서명·공증·stapling | 도면의 점선 후속 릴리스 단계 | FUTURE — 현재 PR CI에는 인증서·공증 비밀을 넣지 않음 |

## 남은 운영체제 검증

1. GitHub `macos-15`에서 새 workflow를 최초 실행한다.
2. Swift 앱의 메뉴 막대 설정 반영, 알림 권한·클릭 복원, 휴지통 이동과 부분 실패를 실제 APFS 경로에서 확인한다.
3. Tauri macOS 앱의 `.app` 인벤토리, Finder 위치 표시, 휴지통 동작과 로컬 CLI 연결을 확인한다.
4. 배포 단계가 필요할 때만 Developer ID·공증용 보호 환경을 별도 workflow로 추가한다.
