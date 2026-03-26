# Section 5: Rule Builder & AI Classification

## Files
- `BroomSweepy/Views/RuleBuilderView.swift`
- `BroomSweepy/Services/AIClassifier.swift`

## Requirements
### RuleBuilderView
- 규칙 리스트 (저장된 규칙)
- 새 규칙 추가 시트:
  - 조건 (Condition): 확장자, 파일명 포함, 크기, 날짜
  - 액션 (Action): 폴더로 이동, 날짜 접두어, 삭제, 태그
- 규칙 ON/OFF 토글
- 규칙 실행 순서 드래그 정렬
- UserDefaults에 규칙 JSON 저장

### 규칙 모델
```swift
struct OrganizeRule: Codable, Identifiable {
    let id: UUID
    var name: String
    var isEnabled: Bool
    var conditions: [RuleCondition]
    var actions: [RuleAction]
}

enum RuleCondition: Codable {
    case extensionIs(String)
    case nameContains(String)
    case sizeGreaterThan(Int64)
    case olderThanDays(Int)
}

enum RuleAction: Codable {
    case moveToFolder(String)
    case addDatePrefix
    case addTag(String)
    case delete
}
```

### AIClassifier
- Claude API 호출로 파일 분류
- 입력: 파일명, 확장자, 크기, 수정일
- 출력: 추천 카테고리, 추천 폴더, 설명
- 배치 처리 (여러 파일 한번에)
- API 키: UserDefaults 또는 Keychain에 저장
- 오프라인 fallback: 확장자 기반 규칙
