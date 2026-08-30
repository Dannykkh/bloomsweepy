# broomsweepy-models

> 경로 기반 자동 분류 결과

## BroomSweepy/Models/RuleModels.swift
- **struct** `OrganizeRule` : Codable, Identifiable (L4)
- **enum** `RuleCondition` : Codable, Hashable (L23)
- **enum** `RuleAction` : Codable, Hashable (L41)

## BroomSweepy/Models/ScanModels.swift
- **struct** `CacheItem` : Identifiable, Hashable (L4)
- **enum** `CacheType` : String (L16)
- **struct** `LargeFile` : Identifiable, Hashable (L23)
- **enum** `FileCategory` : String, CaseIterable (L38)
- **struct** `DuplicateGroup` : Identifiable (L85)
- **struct** `DuplicateFile` : Identifiable, Hashable (L97)
- **struct** `ScanSummary` (L107)
- **function** `from()` (L63)
- **function** `formatSize()` (L125)
