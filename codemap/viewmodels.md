# viewmodels

> ViewModel 클래스/속성/명령

## BroomSweepy/ViewModels/CleanerViewModel.swift
- **class** `OperationCancellationToken` : @unchecked Sendable (L4)
- **struct** `CleanerOperationLease` : @unchecked Sendable (L21)
- **class** `CleanerViewModel` (L31)
- **enum** `AppFilter` : String, CaseIterable, Identifiable (L54)
- **function** `cancel()` (L14)
- **function** `scanAll()` (L90)
- **function** `scanCache()` (L187)
- **function** `scanLargeFiles()` (L215)
- **function** `scanDuplicates()` (L244)
- **function** `cleanSelectedCache()` (L275)
- **function** `cancelCurrentTask()` (L333)
- **function** `beginCoordinatedOperation()` (L344)
- **function** `cancelCoordinatedOperation()` (L349)
- **function** `isCoordinatedOperationActive()` (L356)
- **function** `shouldContinueCoordinatedOperation()` (L360)
- **function** `finishCoordinatedOperation()` (L364)
- **function** `deleteSelectedLargeFiles()` (L422)
- **function** `deleteSelectedDuplicates()` (L475)
- **function** `scanApps()` (L566)
- **function** `uninstallSelectedApps()` (L594)
- **function** `scanLoginItems()` (L690)
- **function** `toggleLoginItem()` (L714)
- **function** `disableAllLoginItems()` (L721)
