# broomsweepy-models

> 경로 기반 자동 분류 결과

## BroomSweepy/Models/RuleModels.swift
- **struct** `OrganizeRule` : Codable, Identifiable (L4)
- **enum** `RuleCondition` : Codable, Hashable (L23)
- **enum** `RuleAction` : Codable, Hashable (L41)

## BroomSweepy/Models/ScanModels.swift
- **struct** `FileIdentitySnapshot` : Hashable, Codable, Sendable (L9)
- **enum** `Kind` : String, Hashable, Codable, Sendable (L10)
- **struct** `CacheItem` : Identifiable, Hashable, Sendable (L53)
- **enum** `CacheType` : String, Sendable (L66)
- **struct** `LargeFile` : Identifiable, Hashable, Sendable (L73)
- **enum** `FileCategory` : String, CaseIterable, Sendable (L89)
- **struct** `DuplicateGroup` : Identifiable, Sendable (L136)
- **struct** `DuplicateFile` : Identifiable, Hashable, Sendable (L148)
- **struct** `ScanSummary` (L159)
- **function** `capture()` (L21)
- **function** `exactlyMatches()` (L46)
- **function** `from()` (L114)
- **function** `formatSize()` (L177)
