# viewmodels

> ViewModel 클래스/속성/명령

## BroomSweepy/ViewModels/CleanerViewModel.swift
- **class** `OperationCancellationToken` : @unchecked Sendable (L4)
- **struct** `CleanerOperationLease` : @unchecked Sendable (L21)
- **class** `CleanerViewModel` (L31)
- **enum** `AppFilter` : String, CaseIterable, Identifiable (L54)
- **function** `cancel()` (L14)
- **function** `scanAll()` (L90)
- **function** `scanCache()` (L186)
- **function** `scanLargeFiles()` (L214)
- **function** `scanDuplicates()` (L243)
- **function** `cleanSelectedCache()` (L274)
- **function** `cancelCurrentTask()` (L332)
- **function** `beginCoordinatedOperation()` (L343)
- **function** `cancelCoordinatedOperation()` (L348)
- **function** `isCoordinatedOperationActive()` (L355)
- **function** `shouldContinueCoordinatedOperation()` (L359)
- **function** `finishCoordinatedOperation()` (L363)
- **function** `deleteSelectedLargeFiles()` (L421)
- **function** `deleteSelectedDuplicates()` (L474)
- **function** `scanApps()` (L565)
- **function** `uninstallSelectedApps()` (L593)
- **function** `scanLoginItems()` (L689)
- **function** `toggleLoginItem()` (L713)
- **function** `disableAllLoginItems()` (L720)
