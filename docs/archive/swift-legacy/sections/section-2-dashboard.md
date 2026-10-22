# Section 2: Dashboard & Cache Cleaner

## Files
- `BroomSweepy/Views/DashboardView.swift`
- `BroomSweepy/Views/CacheCleanerView.swift`

## Requirements
### DashboardView
- 히어로 영역: 앱 아이콘 + "전체 스캔 시작" 버튼
- 프로그레스 바 (스캔 중)
- 4개 요약 카드: 캐시, 대용량, 중복, 정리 가능 총량
- Swift Charts로 디스크 사용량 도넛차트

### CacheCleanerView
- 캐시 항목 리스트 (SF Symbol 아이콘)
- 체크박스 선택/전체선택
- 크기 바 시각화
- 안전등급 뱃지 (Safe 초록)
- "선택 항목 정리" 버튼 + confirm alert
