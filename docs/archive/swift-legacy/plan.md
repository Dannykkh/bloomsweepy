# BroomSweepy — Implementation Plan

## Project Overview
macOS 네이티브 클리너 + 파일 정리 앱. SwiftUI 기반, App Store 배포 목표.
CCleaner/CleanMyMac 대안이면서 AI 파일 정리 기능으로 차별화.

## Tech Stack
- **Language**: Swift 5.9+
- **UI**: SwiftUI (NavigationSplitView, Charts)
- **Platform**: macOS 13.0+ (Ventura)
- **Build**: Xcode 16 + xcodegen
- **AI**: Anthropic Swift SDK (Claude API)
- **Sandbox**: Security-Scoped Bookmarks

## Architecture
```
BroomSweepy/
├── BroomSweepyApp.swift        # App entry point
├── ContentView.swift            # NavigationSplitView + sidebar
├── Views/
│   ├── DashboardView.swift      # 메인 대시보드 + 전체 스캔
│   ├── CacheCleanerView.swift   # 캐시/임시파일 정리
│   ├── LargeFilesView.swift     # 대용량 파일 탐색
│   ├── DuplicateFilesView.swift # 중복 파일 탐색
│   ├── FileOrganizerView.swift  # AI 파일 정리
│   └── RuleBuilderView.swift    # 커스텀 규칙 빌더
├── ViewModels/
│   └── CleanerViewModel.swift   # 메인 ViewModel (@Observable)
├── Models/
│   └── ScanModels.swift         # 데이터 모델
├── Services/
│   ├── CleanerEngine.swift      # 스캔/정리 엔진
│   ├── FileOrganizerEngine.swift # 파일 정리 엔진
│   ├── FileAccessManager.swift  # 샌드박스 파일 접근
│   └── AIClassifier.swift       # Claude API 연동
├── Resources/
│   └── Assets.xcassets/
├── Info.plist
└── BroomSweepy.entitlements
```

## Sections (구현 단위)

### Section 1: Core Infrastructure
- BroomSweepyApp.swift (앱 진입점)
- ContentView.swift (사이드바 네비게이션)
- ScanModels.swift (데이터 모델) ✅ 기존
- CleanerEngine.swift (스캔 엔진) ✅ 기존
- FileAccessManager.swift (파일 접근) ✅ 기존
- CleanerViewModel.swift (메인 ViewModel)

### Section 2: Dashboard & Cache Cleaner
- DashboardView.swift (전체 스캔, 요약 카드, 디스크 시각화)
- CacheCleanerView.swift (캐시 리스트, 선택/정리)

### Section 3: Large Files & Duplicates
- LargeFilesView.swift (카테고리 필터, 안전등급)
- DuplicateFilesView.swift (중복 그룹, 원본 표시)

### Section 4: File Organizer
- FileOrganizerView.swift (폴더 선택, 규칙 미리보기, 실행)
- FileOrganizerEngine.swift (날짜 접두어, EXIF 분류, 폴더 생성)

### Section 5: Rule Builder & AI
- RuleBuilderView.swift (조건/액션 UI)
- AIClassifier.swift (Claude API 연동)

### Section 6: Build & Polish
- Assets.xcassets (앱 아이콘, 색상)
- project.yml 완성 ✅ 기존
- xcodegen + xcodebuild 빌드 검증

## Safety Rating System
| 등급 | 색상 | 의미 |
|------|------|------|
| Safe | 초록 | 삭제해도 안전 (캐시, 임시파일) |
| Review | 노랑 | 확인 후 삭제 권장 (대용량, 오래된 파일) |
| Caution | 빨강 | 주의 필요 (시스템 관련, 앱 데이터) |

## File Organization Rules
- 날짜 접두어: `YYYY-MM-DD_원본파일명.ext`
- 사진: EXIF 날짜 → `사진/YYYY/MM-Month/`
- 스크린샷: `스크린샷/YYYY-MM/`
- 문서: 확장자별 `문서/PDF/`, `문서/오피스/`
- 설치파일: `설치파일/` + 30일 경과 알림
- AI 분류: 파일명 분석 불가 시 Claude API로 카테고리 판단
