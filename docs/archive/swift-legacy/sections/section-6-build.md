# Section 6: Build & Polish

## Tasks
1. Assets.xcassets — AccentColor, AppIcon placeholder
2. project.yml 최종 검증
3. xcodegen으로 .xcodeproj 생성
4. xcodebuild 빌드 확인
5. 앱 실행 테스트

## Build Commands
```bash
cd /Users/dannysmacair/Downloads/BroomSweepy
xcodegen generate
xcodebuild -project BroomSweepy.xcodeproj -scheme BroomSweepy -configuration Debug build
```
