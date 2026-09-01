# services

> 서비스 클래스/메서드

## BroomSweepy/Services/AIClassifier.swift
- **struct** `AIClassification` (L4)
- **class** `AIClassifier` (L13)
- **function** `classify()` (L27)
- **function** `fallbackClassify()` (L108)

## BroomSweepy/Services/AppUninstaller.swift
- **struct** `InstalledApp` : Identifiable, Sendable (L2)
- **class** `AppUninstaller` (L24)
- **function** `scanApps()` (L30)
- **function** `uninstall()` (L88)

## BroomSweepy/Services/AppVersionChecker.swift
- **struct** `AppVersionInfo` : Identifiable, Sendable (L2)
- **class** `AppVersionChecker` (L14)
- **function** `scanInstalledApps()` (L20)

## BroomSweepy/Services/BrokenDownloadCleaner.swift
- **struct** `BrokenDownload` : Identifiable, Hashable, Sendable (L2)
- **enum** `Reason` : String, Sendable (L14)
- **class** `BrokenDownloadCleaner` (L21)
- **function** `scan()` (L31)
- **function** `clean()` (L97)

## BroomSweepy/Services/BrokenPlistCleaner.swift
- **class** `BrokenPlistCleaner` (L2)
- **struct** `BrokenPlist` : Identifiable, Sendable (L6)
- **enum** `Reason` : String, Sendable (L18)
- **function** `scan()` (L26)
- **function** `clean()` (L99)

## BroomSweepy/Services/CleanerEngine.swift
- **class** `CleanerEngine` (L3)
- **function** `scanCache()` (L28)
- **function** `scanLargeFiles()` (L96)
- **function** `scanDuplicates()` (L174)
- **function** `cleanCache()` (L352)
- **function** `deleteFiles()` (L426)
- **function** `trashVerifiedLargeFiles()` (L432)
- **function** `trashVerifiedDuplicates()` (L474)

## BroomSweepy/Services/CleanHistory.swift
- **class** `CleanHistory` (L5)
- **struct** `CleanRecord` : Codable, Identifiable (L7)
- **function** `record()` (L32)

## BroomSweepy/Services/CloudStorageCleaner.swift
- **struct** `CloudProvider` : Identifiable, Sendable (L4)
- **struct** `CloudFile` : Identifiable, Hashable, Sendable (L17)
- **class** `CloudStorageCleaner` (L35)
- **function** `scan()` (L43)
- **function** `trashReviewedFiles()` (L101)

## BroomSweepy/Services/FileAccessManager.swift
- **class** `FileAccessManager` (L12)
- **function** `actualUserHomeURL()` (L3)
- **function** `requestFolderAccess()` (L23)
- **function** `requestHomeAccess()` (L36)
- **function** `loadBookmark()` (L70)
- **function** `releaseFolderAccess()` (L105)
- **function** `stopAccessingResources()` (L115)

## BroomSweepy/Services/FileOrganizerEngine.swift
- **struct** `OrganizePlan` : Identifiable, Sendable (L6)
- **struct** `OrganizeOptions` (L20)
- **class** `FileOrganizerEngine` (L29)
- **function** `preview()` (L46)
- **function** `execute()` (L125)
- **function** `undo()` (L190)

## BroomSweepy/Services/HealthMonitor.swift
- **class** `HealthMonitor` (L7)
- **struct** `DailyBriefing` (L28)
- **struct** `Recommendation` (L38)
- **enum** `Priority` (L44)
- **function** `generateBriefing()` (L72)
- **function** `recordClean()` (L144)
- **function** `startScheduleIfEnabled()` (L158)
- **function** `sendNotificationIfNeeded()` (L247)
- **function** `sendScanCompletedNotification()` (L263)
- **function** `requestNotificationPermission()` (L280)

## BroomSweepy/Services/LanguageCleaner.swift
- **class** `LanguageCleaner` (L2)
- **struct** `LanguageResource` : Identifiable, Sendable (L9)
- **function** `scan()` (L22)
- **function** `clean()` (L66)
- **function** `totalSize()` (L72)

## BroomSweepy/Services/MailAttachmentCleaner.swift
- **struct** `MailAttachment` : Identifiable, Hashable, Sendable (L2)
- **class** `MailAttachmentCleaner` (L19)
- **function** `scan()` (L25)
- **function** `clean()` (L57)

## BroomSweepy/Services/MaintenanceManager.swift
- **struct** `MaintenanceTask` : Identifiable, Sendable (L2)
- **enum** `TaskType` : Sendable (L13)
- **struct** `MaintenanceMoveCandidate` : Sendable (L31)
- **struct** `MaintenanceApprovedRoot` : Sendable (L39)
- **struct** `MaintenancePreview` : Sendable (L44)
- **enum** `MaintenanceOutcome` : Sendable (L52)
- **enum** `Kind` : Equatable, Sendable (L54)
- **class** `MaintenanceManager` (L96)
- **function** `getAvailableTasks()` (L102)
- **function** `instruction()` (L149)
- **function** `preview()` (L160)
- **function** `runTask()` (L187)

## BroomSweepy/Services/MalwareScanner.swift
- **struct** `MalwareThreat` : Identifiable, Sendable (L5)
- **enum** `ThreatSeverity` : String, Sendable (L15)
- **class** `MalwareScanner` (L32)
- **function** `scan()` (L55)

## BroomSweepy/Services/MemoryManager.swift
- **class** `MemoryManager` (L2)
- **struct** `MemoryInfo` (L5)
- **function** `getMemoryInfo()` (L21)
- **function** `purgeMemory()` (L55)

## BroomSweepy/Services/PermissionManager.swift
- **struct** `AppPermission` : Identifiable, Sendable (L5)
- **enum** `Evidence` : String, Sendable (L19)
- **enum** `PermissionType` : String, CaseIterable, Sendable (L24)
- **class** `PermissionManager` (L64)
- **function** `scan()` (L72)
- **function** `openSystemSettings()` (L104)

## BroomSweepy/Services/PrivacyCleaner.swift
- **struct** `BrowserData` : Identifiable, Sendable (L2)
- **enum** `BrowserDataType` : String, CaseIterable, Sendable (L28)
- **class** `PrivacyCleaner` (L60)
- **function** `scan()` (L66)
- **function** `clean()` (L177)

## BroomSweepy/Services/SimilarImageFinder.swift
- **struct** `SimilarImageGroup` : Identifiable, Sendable (L5)
- **struct** `SimilarImage` : Identifiable, Hashable, Sendable (L12)
- **class** `SimilarImageFinder` (L32)
- **function** `hash()` (L21)
- **function** `scan()` (L41)

## BroomSweepy/Services/StartupManager.swift
- **struct** `LoginItem` : Identifiable, Sendable (L2)
- **enum** `LoginItemType` : String, Sendable (L10)
- **class** `StartupManager` (L16)
- **function** `scanLoginItems()` (L22)
- **function** `setEnabled()` (L66)
- **function** `disableItem()` (L73)
- **function** `enableItem()` (L76)

## BroomSweepy/Services/SystemMonitor.swift
- **class** `SystemMonitor` (L3)
- **struct** `CPUInfo` (L8)
- **struct** `BatteryInfo` (L66)
- **struct** `DiskInfo` (L139)
- **function** `getCPUInfo()` (L16)
- **function** `getBatteryInfo()` (L75)
- **function** `getDiskInfo()` (L154)

## BroomSweepy/Services/VerifiedFileMover.swift
- **class** `VerifiedFileMover` : @unchecked Sendable (L11)
- **struct** `MoveResult` : Sendable (L13)
- **struct** `RecoveryReport` : Sendable (L19)
- **function** `moveToTrash()` (L101)
- **function** `moveAtomically()` (L118)
- **function** `recoverPendingOperations()` (L136)
