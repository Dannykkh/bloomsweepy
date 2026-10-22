# Section 4: File Organizer

## Files
- `BroomSweepy/Views/FileOrganizerView.swift`
- `BroomSweepy/Services/FileOrganizerEngine.swift`

## Requirements
### FileOrganizerView
- "정리할 폴더 선택" 버튼 (NSOpenPanel)
- 정리 규칙 체크리스트 (토글):
  - 날짜 접두어 추가
  - 확장자별 폴더 분류
  - 사진 날짜별 분류
  - 스크린샷 분류
- 미리보기: 변경 전→후 테이블
- "정리 실행" + "되돌리기(Undo)" 버튼

### FileOrganizerEngine
- `organizeByType(url:)` — 확장자별 폴더 분류
- `addDatePrefix(url:)` — YYYY-MM-DD 접두어 추가
- `organizePhotos(url:)` — EXIF CreateDate → YYYY/MM-Month/ 폴더
- `organizeScreenshots(url:)` — "Screenshot" 패턴 → 스크린샷/YYYY-MM/
- `preview(url:rules:)` — 실제 이동 없이 변경 계획 반환
- `execute(plan:)` — 미리보기 계획 실행
- `undo(plan:)` — 실행 결과 되돌리기

### 폴더 생성 규칙
```
대상폴더/
├── 사진/YYYY/MM-Month/
├── 동영상/YYYY/
├── 문서/PDF/ | 오피스/ | 텍스트/
├── 음악/
├── 압축파일/
├── 설치파일/
├── 스크린샷/YYYY-MM/
├── 개발/
└── 기타/
```

### 날짜 소스 우선순위
1. EXIF CreateDate (사진)
2. File Creation Date (kMDItemContentCreationDate)
3. File Modification Date (fallback)
