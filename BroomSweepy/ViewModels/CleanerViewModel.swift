import SwiftUI
import Combine

@Observable
final class CleanerViewModel {
    // MARK: - Scan State
    var isScanning = false
    var scanProgress: Double = 0
    var scanMessage = ""
    var cleanErrors: [String] = []
    var currentTask: Task<Void, Never>?

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
        case unused = "미사용 (6개월+)"
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

    init() {
        loadRules()
    }

    // MARK: - Full Scan

    @MainActor
    func scanAll() async {
        // 먼저 폴더 접근 권한 확인/요청 (메인 스레드에서 NSOpenPanel)
        let scanURL: URL
        if let bookmark = fileAccess.loadBookmark() {
            scanURL = bookmark
        } else if let granted = fileAccess.requestHomeAccess() {
            scanURL = granted
        } else {
            toastMessage = "폴더 접근 권한이 필요합니다"
            return
        }

        isScanning = true
        scanProgress = 0
        scanMessage = "캐시 + 대용량 파일 스캔 중..."

        // Phase 1: 캐시 + 대용량 파일 동시 스캔 (서로 다른 경로 → I/O 병렬 가능)
        async let cacheResult = Task.detached { [engine, scanURL] in
            engine.scanCache(homeURL: scanURL) { [weak self] msg, progress in
                Task { @MainActor in
                    self?.scanMessage = "캐시: \(msg)"
                    if progress >= 0 { self?.scanProgress = progress * 0.25 }
                }
            }
        }.value

        async let largeResult = Task.detached { [engine, scanURL] in
            engine.scanLargeFiles(scanURL: scanURL, minSizeMB: 50) { [weak self] msg, _ in
                Task { @MainActor in
                    self?.scanMessage = msg
                    self?.scanProgress = 0.25 + 0.10
                }
            }
        }.value

        // 두 결과를 동시에 기다림
        let (cache, large) = await (cacheResult, largeResult)
        cacheItems = cache
        largeFiles = large
        scanProgress = 0.50

        // Phase 2: 중복 파일 분석 (해싱이 무거우므로 단독)
        scanMessage = "중복 파일 분석 중..."
        duplicateGroups = await Task.detached { [engine, scanURL] in
            engine.scanDuplicates(scanURL: scanURL, minSizeKB: 100) { [weak self] msg, progress in
                Task { @MainActor in
                    self?.scanMessage = msg
                    if progress >= 0 { self?.scanProgress = 0.50 + progress * 0.50 }
                }
            }
        }.value

        updateSummary()
        scanProgress = 1.0
        scanMessage = "스캔 완료!"
        isScanning = false
        toastMessage = "스캔 완료! 결과를 확인하세요"
    }

    // MARK: - Individual Scans

    @MainActor
    func scanCache() async {
        isScanning = true
        let homeURL = fileAccess.loadBookmark()
        cacheItems = await Task.detached { [engine] in
            engine.scanCache(homeURL: homeURL, progressCallback: nil)
        }.value
        isScanning = false
        updateSummary()
        toastMessage = "캐시 스캔 완료: \(cacheItems.count)개 항목"
    }

    @MainActor
    func scanLargeFiles() async {
        isScanning = true
        guard let url = fileAccess.loadBookmark() ?? fileAccess.requestHomeAccess() else {
            isScanning = false
            toastMessage = "폴더 접근 권한이 필요합니다"
            return
        }
        largeFiles = await Task.detached { [engine] in
            engine.scanLargeFiles(scanURL: url, minSizeMB: 50, progressCallback: nil)
        }.value
        isScanning = false
        updateSummary()
        toastMessage = "대용량 파일 스캔 완료: \(largeFiles.count)개"
    }

    @MainActor
    func scanDuplicates() async {
        isScanning = true
        guard let url = fileAccess.loadBookmark() ?? fileAccess.requestHomeAccess() else {
            isScanning = false
            toastMessage = "폴더 접근 권한이 필요합니다"
            return
        }
        duplicateGroups = await Task.detached { [engine] in
            engine.scanDuplicates(scanURL: url, minSizeKB: 100, progressCallback: nil)
        }.value
        isScanning = false
        updateSummary()
        toastMessage = "중복 탐색 완료: \(duplicateGroups.count)개 그룹"
    }

    // MARK: - Clean (백그라운드에서 실행 → UI 프리징 방지)

    @MainActor
    func cleanSelectedCache() async {
        let items = cacheItems.filter { selectedCacheIDs.contains($0.id) }
        guard !items.isEmpty else { return }
        isScanning = true
        scanMessage = "캐시 정리 중..."
        cleanErrors = []

        let result = await Task.detached { [engine] in
            engine.cleanCache(items: items)
        }.value

        selectedCacheIDs.removeAll()
        isScanning = false
        cleanErrors = result.errors

        if result.errors.isEmpty {
            toastMessage = "정리 완료! \(formatSize(result.freed)) 확보"
        } else if result.freed > 0 {
            toastMessage = "\(formatSize(result.freed)) 확보 (\(result.errors.count)개 항목 권한 부족)"
        } else {
            toastMessage = "삭제 권한이 없습니다. 터미널에서 직접 삭제하세요."
        }
        await scanCache()
    }

    /// 진행 중인 작업 취소
    @MainActor
    func cancelCurrentTask() {
        currentTask?.cancel()
        currentTask = nil
        isScanning = false
        scanMessage = ""
        toastMessage = "작업이 취소되었습니다"
    }

    @MainActor
    func deleteSelectedLargeFiles() async {
        let paths = largeFiles.filter { selectedLargeFileIDs.contains($0.id) }.map(\.path)
        guard !paths.isEmpty else { return }
        isScanning = true
        scanMessage = "파일 삭제 중..."

        let result = await Task.detached { [engine] in
            engine.deleteFiles(paths: paths)
        }.value

        selectedLargeFileIDs.removeAll()
        isScanning = false
        toastMessage = "삭제 완료! \(formatSize(result.freed)) 확보"
        await scanLargeFiles()
    }

    @MainActor
    func deleteSelectedDuplicates() async {
        let paths = Array(selectedDuplicateFileIDs).compactMap { id in
            duplicateGroups.flatMap(\.files).first { $0.id == id }?.path
        }
        guard !paths.isEmpty else { return }
        isScanning = true
        scanMessage = "중복 파일 삭제 중..."

        let result = await Task.detached { [engine] in
            engine.deleteFiles(paths: paths)
        }.value

        selectedDuplicateFileIDs.removeAll()
        isScanning = false
        toastMessage = "삭제 완료! \(formatSize(result.freed)) 확보"
        await scanDuplicates()
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
        isScanning = true
        scanMessage = "설치된 앱 스캔 중..."
        let homeURL = fileAccess.loadBookmark()
        installedApps = await Task.detached {
            AppUninstaller.shared.scanApps(homeURL: homeURL)
        }.value
        isScanning = false
        toastMessage = "\(installedApps.count)개 앱을 찾았습니다"
    }

    @MainActor
    func uninstallSelectedApps() async {
        let targets = installedApps.filter { selectedAppIDs.contains($0.id) }
        guard !targets.isEmpty else { return }
        isScanning = true
        scanMessage = "앱 삭제 중..."

        let (totalFreed, hasErrors) = await Task.detached { () -> (Int64, Bool) in
            var freed: Int64 = 0
            var errors = false
            for app in targets {
                let result = AppUninstaller.shared.uninstall(app: app)
                freed += result.freedSize
                if !result.errors.isEmpty { errors = true }
            }
            return (freed, errors)
        }.value

        selectedAppIDs.removeAll()
        isScanning = false
        if totalFreed > 0 && !hasErrors {
            toastMessage = "삭제 완료! \(formatSize(totalFreed)) 확보"
        } else if totalFreed > 0 {
            toastMessage = "관련 파일 \(formatSize(totalFreed)) 정리 (앱 본체는 Finder에서 삭제해주세요)"
        } else {
            toastMessage = "앱 삭제에 권한이 필요합니다. Finder에서 직접 삭제해주세요"
        }
        await scanApps()
    }

    var filteredApps: [InstalledApp] {
        switch appFilter {
        case .all:    return installedApps
        case .unused: return installedApps.filter(\.isUnused)
        case .bySize: return installedApps.sorted { $0.totalSize > $1.totalSize }
        }
    }

    // MARK: - Startup Manager

    @MainActor
    func scanLoginItems() async {
        isScanning = true
        scanMessage = "시작프로그램 스캔 중..."
        let homeURL = fileAccess.loadBookmark()
        loginItems = await Task.detached {
            StartupManager.shared.scanLoginItems(homeURL: homeURL)
        }.value
        isScanning = false
        toastMessage = "\(loginItems.count)개 시작프로그램을 찾았습니다"
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
