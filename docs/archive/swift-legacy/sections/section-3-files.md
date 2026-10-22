# Section 3: Large Files & Duplicates

## Files
- `BroomSweepy/Views/LargeFilesView.swift`
- `BroomSweepy/Views/DuplicateFilesView.swift`

## Requirements
### LargeFilesView
- 카테고리 필터 칩 (동영상, 이미지, 문서 등)
- 파일 리스트: 이름, 경로, 크기, 수정일, 카테고리
- 안전등급: 30일 미접근 = Review, 설치파일/백업 = Caution
- 체크박스 선택 + 삭제 기능
- NSOpenPanel으로 스캔 폴더 선택

### DuplicateFilesView
- 중복 그룹 카드 (해시 기반)
- 그룹별: 파일명, 복사본 수, 개별 크기, 낭비 용량
- 그룹 내 파일 리스트 (첫 번째 = 원본 뱃지)
- 원본 제외 전체 선택 기능
