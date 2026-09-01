import Foundation
import SwiftUI
import Combine

final class OperationCancellationToken: @unchecked Sendable {
    private let lock = NSLock()
    private var cancelled = false

    var isCancelled: Bool {
        lock.lock()
        defer { lock.unlock() }
        return cancelled
    }

    func cancel() {
        lock.lock()
        cancelled = true
        lock.unlock()
    }
}

struct CleanerOperationLease: @unchecked Sendable {
    let id: UUID
    let token: OperationCancellationToken

    var isCancelled: Bool { token.isCancelled }
}

@MainActor
@Observable
final class CleanerViewModel {
    // MARK: - Scan State
    var isScanning = false
    var scanProgress: Double = 0
    var scanMessage = ""
    var cleanErrors: [String] = []
    var isCancellationPending = false

    // MARK: - Data
    var cacheItems: [CacheItem] = []
    var largeFiles: [LargeFile] = []
    var duplicateGroups: [DuplicateGroup] = []
    var summary = ScanSummary()

    // MARK: - Selection
    var selectedCacheIDs: Set<UUID> = []
    var selectedLargeFileIDs: Set<UUID> = []
    var selectedDuplicateFileIDs: Set<UUID> = []

    // MARK: - App Uninstaller
    var installedApps: [InstalledApp] = []
    var selectedAppIDs: Set<UUID> = []
    var appFilter: AppFilter = .all

    enum AppFilter: String, CaseIterable, Identifiable {
        case all = "전체"
        case unmodified = "180일+ 수정 없음"
        case bySize = "크기순"
        var id: String { rawValue }
    }

    // MARK: - Startup Manager
    var loginItems: [LoginItem] = []

    // MARK: - File Organizer
    var organizerTargetURL: URL?
    var organizerPreview: [OrganizePlan] = []
    var isOrganizing = false

    // MARK: - Rules
    var rules: [OrganizeRule] = [] {
        didSet { saveRules() }
    }

    // MARK: - Toast
    var toastMessage: String?

    private let engine = CleanerEngine.shared
    private let fileAccess = FileAccessManager.shared
    private var activeOperationID: UUID?
    private var activeOperationToken: OperationCancellationToken?

    init() {
        loadRules()
    }

    // MARK: - Full Scan

    @MainActor
    func scanAll() async {
        guard let (scanID, token) = beginOperation() else { return }

        // 먼저 폴더 접근 권한 확인/요청 (메인 스레드에서 NSOpenPanel)
        let scanURL: URL
        if let bookmark = fileAccess.loadBookmark() {
            scanURL = bookmark
        } else if let granted = fileAccess.requestHomeAccess() {
            scanURL = granted
        } else {
            finishOperation(scanID, token: token)
            toastMessage = "폴더 접근 권한이 필요합니다"
            return
        }

        scanProgress = 0
        scanMessage = "캐시 + 대용량 파일 스캔 중..."

        // Phase 1: 캐시 + 대용량 파일 동시 스캔 (서로 다른 경로 → I/O 병렬 가능)
        let scanResults: ([CacheItem], [LargeFile]) = await withTaskCancellationHandler {
            async let cacheResult: [CacheItem] = Task.detached { [engine, scanURL, token] in
                engine.scanCache(
                    homeURL: scanURL,
                    progressCallback: { [weak self] msg, progress in
                        Task { @MainActor in
                            guard self?.isCurrentOperation(scanID, token: token) == true else { return }
                            self?.scanMessage = "캐시: \(msg)"
                            if progress >= 0 { self?.scanProgress = progress * 0.25 }
                        }
                    },
                    shouldCancel: { token.isCancelled }
                )
            }.value

            async let largeResult: [LargeFile] = Task.detached { [engine, scanURL, token] in
                engine.scanLargeFiles(
                    scanURL: scanURL,
                    minSizeMB: 50,
                    progressCallback: { [weak self] msg, _ in
                        Task { @MainActor in
                            guard self?.isCurrentOperation(scanID, token: token) == true else { return }
                            self?.scanMessage = msg
                            self?.scanProgress = 0.25 + 0.10
                        }
                    },
                    shouldCancel: { token.isCancelled }
                )
            }.value

            return await (cacheResult, largeResult)
        } onCancel: {
            token.cancel()
        }
        let (cache, large) = scanResults
        guard shouldContinueOperation(scanID, token: token) else { return }
        cacheItems = cache
        largeFiles = large
        selectedCacheIDs.removeAll()
        selectedLargeFileIDs.removeAll()
        scanProgress = 0.50

        // Phase 2: 중복 파일 분석 (해싱이 무거우므로 단독)
        scanMessage = "중복 파일 분석 중..."
        let duplicates = await withTaskCancellationHandler {
            await Task.detached { [engine, scanURL, token] in
                engine.scanDuplicates(
                    scanURL: scanURL,
                    minSizeKB: 100,
                    progressCallback: { [weak self] msg, progress in
                        Task { @MainActor in
                            guard self?.isCurrentOperation(scanID, token: token) == true else { return }
                            self?.scanMessage = msg
                            if progress >= 0 { self?.scanProgress = 0.50 + progress * 0.50 }
                        }
                    },
                    shouldCancel: { token.isCancelled }
                )
            }.value
        } onCancel: {
            token.cancel()
        }

        guard shouldContinueOperation(scanID, token: token) else { return }
        duplicateGroups = duplicates
        selectedDuplicateFileIDs.removeAll()

        updateSummary()
        scanProgress = 1.0
        scanMessage = "스캔 완료!"
        toastMessage = "스캔 완료! 결과를 확인하세요"
        finishOperation(scanID, token: token)
        HealthMonitor.shared.sendScanCompletedNotification()
    }

    // MARK: - Individual Scans

    @MainActor
    func scanCache() async {
        guard let (scanID, token) = beginOperation() else { return }
        guard let homeURL = fileAccess.loadBookmark() ?? fileAccess.requestHomeAccess() else {
            finishOperation(scanID, token: token)
            toastMessage = "홈 폴더 접근 권한이 필요합니다"
            return
        }
        let result = await withTaskCancellationHandler {
            await Task.detached { [engine, token] in
                engine.scanCache(
                    homeURL: homeURL,
                    progressCallback: nil,
                    shouldCancel: { token.isCancelled }
                )
            }.value
        } onCancel: {
            token.cancel()
        }

        guard shouldContinueOperation(scanID, token: token) else { return }
        cacheItems = result
        selectedCacheIDs.removeAll()
        finishOperation(scanID, token: token)
        updateSummary()
        toastMessage = "캐시 스캔 완료: \(cacheItems.count)개 항목"
    }

    @MainActor
    func scanLargeFiles() async {
        guard let (scanID, token) = beginOperation() else { return }
        guard let url = fileAccess.loadBookmark() ?? fileAccess.requestHomeAccess() else {
            finishOperation(scanID, token: token)
            toastMessage = "폴더 접근 권한이 필요합니다"
            return
        }
        let result = await withTaskCancellationHandler {
            await Task.detached { [engine, token] in
                engine.scanLargeFiles(
                    scanURL: url,
                    minSizeMB: 50,
                    progressCallback: nil,
                    shouldCancel: { token.isCancelled }
                )
            }.value
        } onCancel: {
            token.cancel()
        }

        guard shouldContinueOperation(scanID, token: token) else { return }
        largeFiles = result
        selectedLargeFileIDs.removeAll()
        finishOperation(scanID, token: token)
        updateSummary()
        toastMessage = "대용량 파일 스캔 완료: \(largeFiles.count)개"
    }

    @MainActor
    func scanDuplicates() async {
        guard let (scanID, token) = beginOperation() else { return }
        guard let url = fileAccess.loadBookmark() ?? fileAccess.requestHomeAccess() else {
            finishOperation(scanID, token: token)
            toastMessage = "폴더 접근 권한이 필요합니다"
            return
        }
        let result = await withTaskCancellationHandler {
            await Task.detached { [engine, token] in
                engine.scanDuplicates(
                    scanURL: url,
                    minSizeKB: 100,
                    progressCallback: nil,
                    shouldCancel: { token.isCancelled }
                )
            }.value
        } onCancel: {
            token.cancel()
        }

        guard shouldContinueOperation(scanID, token: token) else { return }
        duplicateGroups = result
        selectedDuplicateFileIDs.removeAll()
        finishOperation(scanID, token: token)
        updateSummary()
        toastMessage = "중복 탐색 완료: \(duplicateGroups.count)개 그룹"
    }

    // MARK: - Clean (백그라운드에서 실행 → UI 프리징 방지)

    @MainActor
    func cleanSelectedCache() async {
        let items = cacheItems.filter { selectedCacheIDs.contains($0.id) }
        guard !items.isEmpty else { return }
        guard let (operationID, token) = beginOperation() else { return }
        scanMessage = "캐시를 휴지통으로 이동 중..."
        cleanErrors = []

        let result = await withTaskCancellationHandler {
            await Task.detached { [engine, token] in
                engine.cleanCache(items: items, shouldCancel: { token.isCancelled })
            }.value
        } onCancel: {
            token.cancel()
        }

        guard isActiveOperation(operationID, token: token) else { return }
        selectedCacheIDs.removeAll()
        cleanErrors = result.errors

        if !token.isCancelled {
            scanMessage = "캐시 목록을 다시 확인 중..."
            let homeURL = fileAccess.loadBookmark()
            let refreshed = await withTaskCancellationHandler {
                await Task.detached { [engine, token] in
                    engine.scanCache(homeURL: homeURL, shouldCancel: { token.isCancelled })
                }.value
            } onCancel: {
                token.cancel()
            }
            guard isActiveOperation(operationID, token: token) else { return }
            if !token.isCancelled {
                cacheItems = refreshed
                updateSummary()
            }
        }

        let wasCancelled = token.isCancelled

        let resultMessage: String
        if wasCancelled {
            resultMessage = "중단했습니다. 이미 휴지통으로 이동한 논리 용량: \(formatSize(result.freed))"
        } else if result.errors.isEmpty {
            resultMessage = "휴지통으로 이동한 논리 용량: \(formatSize(result.freed)). 휴지통을 비워야 디스크 여유가 늘어납니다."
        } else if result.freed > 0 {
            resultMessage = "휴지통으로 이동한 논리 용량: \(formatSize(result.freed)) (\(result.errors.count)개 실패)"
        } else {
            resultMessage = "휴지통으로 이동하지 못했습니다: \(result.errors.first ?? "경로와 권한을 확인하세요")"
        }
        finishOperation(operationID, token: token)
        if result.freed > 0 {
            HealthMonitor.shared.recordClean()
            CleanHistory.shared.record(freed: result.freed, type: "cache")
        }
        toastMessage = resultMessage
    }

    /// 진행 중인 작업 취소
    @MainActor
    func cancelCurrentTask() {
        guard let token = activeOperationToken else { return }
        token.cancel()
        isCancellationPending = true
        scanMessage = "현재 항목을 마친 뒤 중단합니다..."
        toastMessage = "중단 요청을 보냈습니다"
    }

    /// Shared operation lease used by feature views that run CleanerEngine
    /// workers directly. It keeps those workers mutually exclusive with the
    /// dashboard scans and preserves cancellation state until the worker exits.
    func beginCoordinatedOperation(message: String) -> CleanerOperationLease? {
        guard let (id, token) = beginOperation() else { return nil }
        scanMessage = message
        return CleanerOperationLease(id: id, token: token)
    }

    func cancelCoordinatedOperation(_ lease: CleanerOperationLease) {
        guard isActiveOperation(lease.id, token: lease.token) else { return }
        lease.token.cancel()
        isCancellationPending = true
        scanMessage = "현재 항목을 마친 뒤 중단합니다..."
    }

    func isCoordinatedOperationActive(_ lease: CleanerOperationLease) -> Bool {
        isActiveOperation(lease.id, token: lease.token)
    }

    func shouldContinueCoordinatedOperation(_ lease: CleanerOperationLease) -> Bool {
        isCurrentOperation(lease.id, token: lease.token) && !Task.isCancelled
    }

    func finishCoordinatedOperation(_ lease: CleanerOperationLease) {
        finishOperation(lease.id, token: lease.token)
    }

    @MainActor
    private func beginOperation() -> (UUID, OperationCancellationToken)? {
        guard activeOperationID == nil, activeOperationToken == nil else {
            toastMessage = "진행 중인 작업이 끝난 뒤 다시 시도해 주세요"
            return nil
        }

        let operationID = UUID()
        let token = OperationCancellationToken()
        activeOperationID = operationID
        activeOperationToken = token
        isScanning = true
        isCancellationPending = false
        return (operationID, token)
    }

    @MainActor
    private func isActiveOperation(_ operationID: UUID, token: OperationCancellationToken) -> Bool {
        activeOperationID == operationID && activeOperationToken === token
    }

    @MainActor
    private func isCurrentOperation(_ operationID: UUID, token: OperationCancellationToken) -> Bool {
        isActiveOperation(operationID, token: token) && !token.isCancelled
    }

    @MainActor
    private func shouldContinueOperation(_ operationID: UUID, token: OperationCancellationToken) -> Bool {
        if Task.isCancelled {
            token.cancel()
        }

        guard isCurrentOperation(operationID, token: token) else {
            if isActiveOperation(operationID, token: token) {
                finishOperation(operationID, token: token)
                toastMessage = "작업을 중단했습니다"
            }
            return false
        }
        return true
    }

    @MainActor
    private func finishOperation(_ operationID: UUID, token: OperationCancellationToken) {
        guard isActiveOperation(operationID, token: token) else { return }
        activeOperationID = nil
        activeOperationToken = nil
        isScanning = false
        isCancellationPending = false
        scanMessage = ""
    }

    @MainActor
    func deleteSelectedLargeFiles() async {
        let files = largeFiles.filter { selectedLargeFileIDs.contains($0.id) }
        guard !files.isEmpty else { return }
        guard let (operationID, token) = beginOperation() else { return }
        scanMessage = "파일을 휴지통으로 이동 중..."

        let result = await withTaskCancellationHandler {
            await Task.detached { [engine, token] in
                engine.trashVerifiedLargeFiles(files: files, shouldCancel: { token.isCancelled })
            }.value
        } onCancel: {
            token.cancel()
        }

        guard isActiveOperation(operationID, token: token) else { return }
        selectedLargeFileIDs.removeAll()
        cleanErrors = result.errors

        if !token.isCancelled, let scanURL = fileAccess.loadBookmark() {
            scanMessage = "대용량 파일 목록을 다시 확인 중..."
            let refreshed = await withTaskCancellationHandler {
                await Task.detached { [engine, token] in
                    engine.scanLargeFiles(
                        scanURL: scanURL,
                        minSizeMB: 50,
                        shouldCancel: { token.isCancelled }
                    )
                }.value
            } onCancel: {
                token.cancel()
            }
            guard isActiveOperation(operationID, token: token) else { return }
            if !token.isCancelled {
                largeFiles = refreshed
                updateSummary()
            }
        }

        let wasCancelled = token.isCancelled
        let resultMessage = wasCancelled
            ? "중단했습니다. 이미 휴지통으로 이동한 논리 용량: \(formatSize(result.freed))"
            : result.errors.isEmpty
            ? "휴지통으로 이동한 논리 용량: \(formatSize(result.freed))"
            : "휴지통으로 이동한 논리 용량: \(formatSize(result.freed)) (\(result.errors.count)개 실패: \(result.errors[0]))"
        finishOperation(operationID, token: token)
        if result.freed > 0 {
            HealthMonitor.shared.recordClean()
            CleanHistory.shared.record(freed: result.freed, type: "large")
        }
        toastMessage = resultMessage
    }

    @MainActor
    func deleteSelectedDuplicates() async {
        let selectedIDs = selectedDuplicateFileIDs
        let selectedPathsByID = Dictionary(uniqueKeysWithValues: duplicateGroups
            .flatMap(\.files)
            .filter { selectedIDs.contains($0.id) }
            .map { ($0.id, $0.path) })
        let groups = duplicateGroups.filter { group in
            group.files.contains { selectedIDs.contains($0.id) }
        }
        guard !groups.isEmpty, !selectedPathsByID.isEmpty else { return }
        guard let (operationID, token) = beginOperation() else { return }
        scanMessage = "중복 파일을 최종 확인한 뒤 휴지통으로 이동 중..."

        let result = await withTaskCancellationHandler {
            await Task.detached { [engine, token] in
                engine.trashVerifiedDuplicates(
                    groups: groups,
                    selectedFileIDs: selectedIDs,
                    shouldCancel: { token.isCancelled }
                )
            }.value
        } onCancel: {
            token.cancel()
        }

        guard isActiveOperation(operationID, token: token) else { return }
        let selectedFiles = groups.flatMap(\.files).filter { selectedIDs.contains($0.id) }
        let failedPaths = Set(selectedFiles.filter {
            $0.snapshot.exactlyMatches(path: $0.path)
        }.map(\.path))
        cleanErrors = result.errors

        if !token.isCancelled, let scanURL = fileAccess.loadBookmark() {
            scanMessage = "중복 파일 목록을 다시 확인 중..."
            let refreshed = await withTaskCancellationHandler {
                await Task.detached { [engine, token] in
                    engine.scanDuplicates(
                        scanURL: scanURL,
                        minSizeKB: 100,
                        shouldCancel: { token.isCancelled }
                    )
                }.value
            } onCancel: {
                token.cancel()
            }
            guard isActiveOperation(operationID, token: token) else { return }
            if !token.isCancelled {
                duplicateGroups = refreshed
                updateSummary()
            }
        }

        let wasCancelled = token.isCancelled
        let resultMessage: String
        if wasCancelled {
            resultMessage = "중단했습니다. 이미 휴지통으로 이동한 논리 용량: \(formatSize(result.freed))"
        } else if result.errors.isEmpty {
            resultMessage = "휴지통으로 이동한 논리 용량: \(formatSize(result.freed))"
        } else {
            resultMessage = "휴지통으로 이동한 논리 용량: \(formatSize(result.freed)) · " +
                "\(result.errors.count)개 이동 안 함: \(result.errors[0])"
        }

        if !wasCancelled {
            selectedDuplicateFileIDs = Set(duplicateGroups.flatMap { group in
                group.files.sorted { $0.path < $1.path }.dropFirst()
                    .filter { failedPaths.contains($0.path) }.map(\.id)
            })
        }
        finishOperation(operationID, token: token)
        if result.freed > 0 {
            HealthMonitor.shared.recordClean()
            CleanHistory.shared.record(freed: result.freed, type: "duplicate")
        }
        toastMessage = resultMessage
    }

    // MARK: - Summary

    private func updateSummary() {
        summary.cacheSize = cacheItems.reduce(0) { $0 + $1.size }
        summary.cacheCount = cacheItems.count
        summary.largeFilesSize = largeFiles.reduce(0) { $0 + $1.size }
        summary.largeFilesCount = largeFiles.count
        summary.duplicateWaste = duplicateGroups.reduce(0) { $0 + $1.wastedSize }
        summary.duplicateGroups = duplicateGroups.count
    }

    // MARK: - App Uninstaller

    @MainActor
    func scanApps() async {
        guard let (operationID, token) = beginOperation() else { return }
        guard let homeURL = fileAccess.loadBookmark() ?? fileAccess.requestHomeAccess() else {
            finishOperation(operationID, token: token)
            toastMessage = "홈 폴더 접근 권한이 필요합니다"
            return
        }
        scanMessage = "설치된 앱 스캔 중..."
        let result = await withTaskCancellationHandler {
            await Task.detached { [token] in
                AppUninstaller.shared.scanApps(
                    homeURL: homeURL,
                    shouldCancel: { token.isCancelled }
                )
            }.value
        } onCancel: {
            token.cancel()
        }

        guard shouldContinueOperation(operationID, token: token) else { return }
        installedApps = result
        selectedAppIDs.removeAll()
        let resultMessage = "\(installedApps.count)개 앱을 찾았습니다"
        finishOperation(operationID, token: token)
        toastMessage = resultMessage
    }

    @MainActor
    func uninstallSelectedApps() async {
        let targets = installedApps.filter { selectedAppIDs.contains($0.id) }
        guard !targets.isEmpty else { return }
        guard let (operationID, token) = beginOperation() else { return }
        scanMessage = "앱을 휴지통으로 이동 중..."

        let results = await withTaskCancellationHandler {
            await Task.detached { [token] in
                var results: [(path: String, freedSize: Int64, errors: [String], appMoved: Bool)] = []
                for app in targets {
                    guard !token.isCancelled else { break }
                    let result = AppUninstaller.shared.uninstall(app: app)
                    results.append((
                        path: app.path,
                        freedSize: result.freedSize,
                        errors: result.errors,
                        appMoved: result.appMoved
                    ))
                }
                return results
            }.value
        } onCancel: {
            token.cancel()
        }

        guard isActiveOperation(operationID, token: token) else { return }

        let totalFreed = results.reduce(Int64(0)) { $0 + $1.freedSize }
        let errors = results.flatMap { $0.errors }
        let movedAppPaths = Set(results.filter { $0.appMoved }.map { $0.path })
        let failedAppPaths = Set(targets.map(\.path)).subtracting(movedAppPaths)
        cleanErrors = errors

        installedApps.removeAll { movedAppPaths.contains($0.path) }
        selectedAppIDs = Set(installedApps
            .filter { failedAppPaths.contains($0.path) }
            .map(\.id))

        if !token.isCancelled,
           let homeURL = fileAccess.loadBookmark() {
            scanMessage = "앱 목록을 다시 확인 중..."
            let refreshed = await withTaskCancellationHandler {
                await Task.detached { [token] in
                    AppUninstaller.shared.scanApps(
                        homeURL: homeURL,
                        shouldCancel: { token.isCancelled }
                    )
                }.value
            } onCancel: {
                token.cancel()
            }
            guard isActiveOperation(operationID, token: token) else { return }
            if !token.isCancelled {
                installedApps = refreshed
                selectedAppIDs = Set(installedApps
                    .filter { failedAppPaths.contains($0.path) }
                    .map(\.id))
            }
        }

        let wasCancelled = token.isCancelled
        let resultMessage: String
        if wasCancelled {
            resultMessage = "중단했습니다. 이미 휴지통으로 이동한 논리 용량: \(formatSize(totalFreed))"
        } else if errors.isEmpty {
            resultMessage = "휴지통으로 이동한 논리 용량: \(formatSize(totalFreed)). 휴지통을 비워야 디스크 여유가 늘어납니다."
        } else {
            let shownErrors = errors.prefix(2).joined(separator: " / ")
            let remainingCount = errors.count - min(errors.count, 2)
            let remainingText = remainingCount > 0 ? " 외 \(remainingCount)개" : ""
            if totalFreed > 0 {
                resultMessage = "휴지통으로 이동한 논리 용량: \(formatSize(totalFreed)), \(errors.count)개 실패: \(shownErrors)\(remainingText)"
            } else {
                resultMessage = "휴지통으로 이동하지 못했습니다 (\(errors.count)개): \(shownErrors)\(remainingText)"
            }
        }

        finishOperation(operationID, token: token)
        if totalFreed > 0 {
            HealthMonitor.shared.recordClean()
            CleanHistory.shared.record(freed: totalFreed, type: "manual")
        }
        toastMessage = resultMessage
    }

    var filteredApps: [InstalledApp] {
        switch appFilter {
        case .all:    return installedApps
        case .unmodified: return installedApps.filter(\.isUnmodifiedFor180Days)
        case .bySize: return installedApps.sorted { $0.totalSize > $1.totalSize }
        }
    }

    // MARK: - Startup Manager

    @MainActor
    func scanLoginItems() async {
        guard let (operationID, token) = beginOperation() else { return }
        guard let homeURL = fileAccess.loadBookmark() ?? fileAccess.requestHomeAccess() else {
            finishOperation(operationID, token: token)
            toastMessage = "홈 폴더 접근 권한이 필요합니다"
            return
        }
        scanMessage = "시작프로그램 스캔 중..."
        let result = await withTaskCancellationHandler {
            await Task.detached { [token] in
                StartupManager.shared.scanLoginItems(
                    homeURL: homeURL,
                    shouldCancel: { token.isCancelled }
                )
            }.value
        } onCancel: {
            token.cancel()
        }
        guard shouldContinueOperation(operationID, token: token) else { return }
        loginItems = result
        let resultMessage = "\(loginItems.count)개 시작프로그램을 찾았습니다"
        finishOperation(operationID, token: token)
        toastMessage = resultMessage
    }

    func toggleLoginItem(id: UUID) {
        guard let idx = loginItems.firstIndex(where: { $0.id == id }) else { return }
        let newEnabled = !loginItems[idx].isEnabled
        StartupManager.shared.setEnabled(newEnabled, for: loginItems[idx])
        loginItems[idx].isEnabled = newEnabled
    }

    func disableAllLoginItems() {
        for idx in loginItems.indices {
            StartupManager.shared.setEnabled(false, for: loginItems[idx])
            loginItems[idx].isEnabled = false
        }
        toastMessage = "모든 시작프로그램을 비활성화했습니다"
    }

    // MARK: - Rules Persistence

    private func saveRules() {
        if let data = try? JSONEncoder().encode(rules) {
            UserDefaults.standard.set(data, forKey: "com.broomsweepy.rules")
        }
    }

    private func loadRules() {
        guard let data = UserDefaults.standard.data(forKey: "com.broomsweepy.rules"),
              let saved = try? JSONDecoder().decode([OrganizeRule].self, from: data) else { return }
        rules = saved
    }
}
