# services

> 서비스 클래스/메서드

## BroomSweepy/Services/AIClassifier.swift
- **struct** `AIClassification` (L4)
- **class** `AIClassifier` (L13)
- **function** `classify()` (L27)
- **function** `fallbackClassify()` (L108)

## BroomSweepy/Services/AppUninstaller.swift
- **struct** `InstalledApp` : Identifiable (L3)
- **class** `AppUninstaller` (L24)
- **function** `scanApps()` (L30)
- **function** `uninstall()` (L75)

## BroomSweepy/Services/AppVersionChecker.swift
- **struct** `AppVersionInfo` : Identifiable (L3)
- **class** `AppVersionChecker` (L16)
- **function** `scanInstalledApps()` (L22)

## BroomSweepy/Services/BrokenDownloadCleaner.swift
- **struct** `BrokenDownload` : Identifiable, Hashable (L2)
- **enum** `Reason` : String (L11)
- **class** `BrokenDownloadCleaner` (L18)
- **function** `scan()` (L28)
- **function** `clean()` (L81)

## BroomSweepy/Services/BrokenPlistCleaner.swift
- **class** `BrokenPlistCleaner` (L2)
- **struct** `BrokenPlist` : Identifiable (L6)
- **enum** `Reason` : String (L15)
- **function** `scan()` (L23)
- **function** `clean()` (L87)

## BroomSweepy/Services/CleanerEngine.swift
- **class** `CleanerEngine` (L3)
- **function** `scanCache()` (L32)
- **function** `scanLargeFiles()` (L85)
- **function** `scanDuplicates()` (L152)
- **function** `cleanCache()` (L265)
- **function** `deleteFiles()` (L286)

## BroomSweepy/Services/CleanHistory.swift
- **class** `CleanHistory` (L5)
- **struct** `CleanRecord` : Codable, Identifiable (L7)
- **function** `record()` (L31)

## BroomSweepy/Services/CloudStorageCleaner.swift
- **struct** `CloudProvider` : Identifiable (L4)
- **struct** `CloudFile` : Identifiable, Hashable (L16)
- **class** `CloudStorageCleaner` (L33)
- **function** `scan()` (L41)
- **function** `deleteLocalOnly()` (L97)

## BroomSweepy/Services/FileAccessManager.swift
- **class** `FileAccessManager` (L5)
- **function** `requestFolderAccess()` (L14)
- **function** `requestHomeAccess()` (L32)

## BroomSweepy/Services/FileOrganizerEngine.swift
- **struct** `OrganizePlan` : Identifiable (L5)
- **struct** `OrganizeOptions` (L15)
- **class** `FileOrganizerEngine` (L24)
- **function** `preview()` (L41)
- **function** `execute()` (L105)
- **function** `undo()` (L142)

## BroomSweepy/Services/HealthMonitor.swift
- **class** `HealthMonitor` (L7)
- **struct** `DailyBriefing` (L12)
- **struct** `Recommendation` (L22)
- **enum** `Priority` (L28)
- **function** `generateBriefing()` (L56)
- **function** `recordClean()` (L128)
- **function** `startScheduleIfEnabled()` (L137)
- **function** `sendNotificationIfNeeded()` (L184)
- **function** `requestNotificationPermission()` (L199)

## BroomSweepy/Services/LanguageCleaner.swift
- **class** `LanguageCleaner` (L2)
- **struct** `LanguageResource` : Identifiable (L9)
- **function** `scan()` (L22)
- **function** `clean()` (L64)
- **function** `totalSize()` (L85)

## BroomSweepy/Services/MailAttachmentCleaner.swift
- **struct** `MailAttachment` : Identifiable, Hashable (L2)
- **class** `MailAttachmentCleaner` (L16)
- **function** `scan()` (L22)
- **function** `clean()` (L41)

## BroomSweepy/Services/MaintenanceManager.swift
- **struct** `MaintenanceTask` : Identifiable (L2)
- **enum** `TaskType` (L12)
- **class** `MaintenanceManager` (L23)
- **function** `getAvailableTasks()` (L29)
- **function** `runTask()` (L76)

## BroomSweepy/Services/MalwareScanner.swift
- **struct** `MalwareThreat` : Identifiable (L5)
- **enum** `ThreatSeverity` : String (L15)
- **class** `MalwareScanner` (L32)
- **function** `scan()` (L55)
- **function** `quarantine()` (L81)
- **function** `deleteThreat()` (L106)

## BroomSweepy/Services/MemoryManager.swift
- **class** `MemoryManager` (L2)
- **struct** `MemoryInfo` (L5)
- **function** `getMemoryInfo()` (L21)
- **function** `purgeMemory()` (L55)

## BroomSweepy/Services/PermissionManager.swift
- **struct** `AppPermission` : Identifiable (L5)
- **enum** `PermissionType` : String, CaseIterable (L18)
- **class** `PermissionManager` (L58)
- **function** `scan()` (L66)
- **function** `openSystemSettings()` (L96)

## BroomSweepy/Services/PrivacyCleaner.swift
- **struct** `BrowserData` : Identifiable (L2)
- **enum** `BrowserDataType` : String, CaseIterable (L11)
- **class** `PrivacyCleaner` (L43)
- **function** `scan()` (L49)
- **function** `clean()` (L145)

## BroomSweepy/Services/SimilarImageFinder.swift
- **struct** `SimilarImageGroup` : Identifiable (L7)
- **struct** `SimilarImage` : Identifiable, Hashable (L14)
- **class** `SimilarImageFinder` (L34)
- **function** `hash()` (L23)
- **function** `scan()` (L43)

## BroomSweepy/Services/StartupManager.swift
- **struct** `LoginItem` : Identifiable (L2)
- **enum** `LoginItemType` : String (L10)
- **class** `StartupManager` (L16)
- **function** `scanLoginItems()` (L22)
- **function** `setEnabled()` (L61)
- **function** `disableItem()` (L68)
- **function** `enableItem()` (L71)

## BroomSweepy/Services/SystemMonitor.swift
- **class** `SystemMonitor` (L3)
- **struct** `CPUInfo` (L8)
- **struct** `BatteryInfo` (L66)
- **struct** `DiskInfo` (L139)
- **function** `getCPUInfo()` (L16)
- **function** `getBatteryInfo()` (L75)
- **function** `getDiskInfo()` (L154)
