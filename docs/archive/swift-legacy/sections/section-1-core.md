# Section 1: Core Infrastructure

## Files
- `BroomSweepy/BroomSweepyApp.swift` — @main App 진입점, WindowGroup
- `BroomSweepy/ContentView.swift` — NavigationSplitView, 사이드바 메뉴
- `BroomSweepy/ViewModels/CleanerViewModel.swift` — @Observable ViewModel
- `BroomSweepy/Models/ScanModels.swift` — ✅ 기존 (검토/보완)
- `BroomSweepy/Services/CleanerEngine.swift` — ✅ 기존 (검토/보완)
- `BroomSweepy/Services/FileAccessManager.swift` — ✅ 기존 (검토/보완)

## Requirements
- macOS 13.0+ NavigationSplitView 사용
- @Observable (macOS 14+) 또는 @ObservableObject (macOS 13 호환)
- 사이드바: 대시보드, 캐시정리, 대용량파일, 중복파일, 파일정리, 규칙빌더
- CleanerViewModel이 모든 스캔 상태 관리
- async/await 기반 비동기 스캔
