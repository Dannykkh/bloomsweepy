import Foundation

struct MaintenanceTask: Identifiable, Sendable {
    let id = UUID()
    let name: String
    let description: String
    let icon: String
    var isRunning = false
    var isCompleted = false
    var result: String?
    var resultKind: MaintenanceOutcome.Kind?
    let requiresAdmin: Bool

    enum TaskType: Sendable {
        case clearUserCache
        case clearSpotlightCache
        case clearIconCache
        case clearFontCache
        case clearDNSNote
        case clearLaunchServicesNote

        var isInstructionOnly: Bool {
            switch self {
            case .clearDNSNote, .clearLaunchServicesNote: return true
            default: return false
            }
        }
    }
    let type: TaskType
}

struct MaintenanceMoveCandidate: Sendable {
    let name: String
    let path: String
    let rootPath: String
    let logicalSize: Int64
    let snapshot: FileIdentitySnapshot
}

struct MaintenanceApprovedRoot: Sendable {
    let path: String
    let snapshot: FileIdentitySnapshot
}

struct MaintenancePreview: Sendable {
    let roots: [MaintenanceApprovedRoot]
    let candidates: [MaintenanceMoveCandidate]
    let errors: [String]

    var logicalSize: Int64 { candidates.reduce(0) { $0 + $1.logicalSize } }
}

enum MaintenanceOutcome: Sendable {
    enum Kind: Equatable, Sendable {
        case success
        case partial
        case failure
        case noChange
        case instruction
    }

    case success(message: String, movedSize: Int64)
    case partial(message: String, movedSize: Int64, errors: [String])
    case failure(message: String, errors: [String])
    case noChange(message: String)
    case instruction(message: String)

    var kind: Kind {
        switch self {
        case .success: return .success
        case .partial: return .partial
        case .failure: return .failure
        case .noChange: return .noChange
        case .instruction: return .instruction
        }
    }

    var message: String {
        switch self {
        case .success(let message, _),
             .partial(let message, _, _),
             .failure(let message, _),
             .noChange(let message),
             .instruction(let message):
            return message
        }
    }

    var movedSize: Int64 {
        switch self {
        case .success(_, let size), .partial(_, let size, _): return size
        default: return 0
        }
    }
}

final class MaintenanceManager {
    static let shared = MaintenanceManager()
    private let fm = FileManager.default

    private init() {}

    func getAvailableTasks() -> [MaintenanceTask] {
        [
            MaintenanceTask(
                name: "사용자 캐시 정리",
                description: "1MB 이상인 앱별 캐시를 검토 후 휴지통으로 이동합니다",
                icon: "internaldrive",
                requiresAdmin: false,
                type: .clearUserCache
            ),
            MaintenanceTask(
                name: "Spotlight 캐시 정리",
                description: "Spotlight 검색 캐시를 검토 후 휴지통으로 이동합니다",
                icon: "magnifyingglass",
                requiresAdmin: false,
                type: .clearSpotlightCache
            ),
            MaintenanceTask(
                name: "아이콘 캐시 초기화",
                description: "아이콘 캐시를 검토 후 휴지통으로 이동합니다",
                icon: "photo",
                requiresAdmin: false,
                type: .clearIconCache
            ),
            MaintenanceTask(
                name: "폰트 캐시 정리",
                description: "폰트 캐시를 검토 후 휴지통으로 이동합니다",
                icon: "textformat",
                requiresAdmin: false,
                type: .clearFontCache
            ),
            MaintenanceTask(
                name: "DNS 캐시 초기화 안내",
                description: "관리자 권한이 필요한 터미널 명령을 안내만 합니다",
                icon: "network",
                requiresAdmin: true,
                type: .clearDNSNote
            ),
            MaintenanceTask(
                name: "Launch Services 재구축 안내",
                description: "터미널 명령을 안내만 하며 자동 실행하지 않습니다",
                icon: "arrow.triangle.2.circlepath",
                requiresAdmin: true,
                type: .clearLaunchServicesNote
            ),
        ]
    }

    func instruction(for task: MaintenanceTask) -> MaintenanceOutcome? {
        switch task.type {
        case .clearDNSNote:
            return .instruction(message: "터미널에서 직접 실행할 명령:\nsudo dscacheutil -flushcache && sudo killall -HUP mDNSResponder")
        case .clearLaunchServicesNote:
            return .instruction(message: "터미널에서 직접 실행할 명령:\n/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -kill -r -domain local -domain user")
        default:
            return nil
        }
    }

    func preview(task: MaintenanceTask, homeURL: URL) -> MaintenancePreview {
        let canonicalHome = homeURL.standardizedFileURL.resolvingSymlinksInPath()
        guard canonicalHome.path == actualUserHomeURL().path else {
            return MaintenancePreview(
                roots: [],
                candidates: [],
                errors: ["승인된 홈 폴더를 확인하지 못했습니다"]
            )
        }
        var preview = MaintenancePreview(roots: [], candidates: [], errors: [])
        for target in targets(for: task, homeURL: canonicalHome) {
            let resolvedTarget = URL(fileURLWithPath: target.path)
                .resolvingSymlinksInPath().standardizedFileURL.path
            guard isSameOrDescendant(target.path, of: canonicalHome.path),
                  isSameOrDescendant(resolvedTarget, of: canonicalHome.path) else {
                preview = merge(preview, MaintenancePreview(
                    roots: [],
                    candidates: [],
                    errors: ["승인된 홈 폴더 밖으로 연결되는 대상은 검토하지 않았습니다"]
                ))
                continue
            }
            preview = merge(preview, inspectRoot(path: target.path, minimumChildSize: target.minimumChildSize))
        }
        return preview
    }

    func runTask(_ task: MaintenanceTask, preview: MaintenancePreview) -> MaintenanceOutcome {
        guard !task.type.isInstructionOnly else {
            return instruction(for: task) ?? .failure(message: "안내를 불러오지 못했습니다", errors: [])
        }
        guard preview.errors.isEmpty else {
            return .failure(message: "대상 검토 중 오류가 있어 실행하지 않았습니다: \(preview.errors[0])", errors: preview.errors)
        }
        guard !preview.candidates.isEmpty else {
            return .noChange(message: "휴지통으로 이동할 대상이 없습니다")
        }

        var roots = Dictionary(uniqueKeysWithValues: preview.roots.map { ($0.path, $0.snapshot) })
        var movedCount = 0
        var movedSize: Int64 = 0
        var errors: [String] = []

        for candidate in preview.candidates {
            guard let rootSnapshot = roots[candidate.rootPath],
                  rootSnapshot.kind == .directory || candidate.rootPath == candidate.path,
                  rootSnapshot.exactlyMatches(path: candidate.rootPath),
                  isSameOrDescendant(candidate.path, of: candidate.rootPath),
                  candidate.snapshot.exactlyMatches(path: candidate.path) else {
                errors.append("\(candidate.name): 검토 뒤 대상 또는 상위 폴더가 변경되어 이동하지 않았습니다")
                continue
            }
            let result = VerifiedFileMover.shared.moveToTrash(
                path: candidate.path,
                expectedSnapshot: candidate.snapshot
            )
            if result.succeeded {
                movedCount += 1
                movedSize += candidate.logicalSize
            } else {
                errors.append("\(candidate.name): \(result.error ?? "휴지통으로 이동하지 못했습니다")")
            }
            if candidate.rootPath != candidate.path,
               let refreshedRoot = FileIdentitySnapshot.capture(path: candidate.rootPath),
               refreshedRoot.kind == .directory,
               refreshedRoot.device == rootSnapshot.device,
               refreshedRoot.inode == rootSnapshot.inode {
                roots[candidate.rootPath] = refreshedRoot
            }
        }

        let movedText = "휴지통으로 이동한 논리 용량: \(formatSize(movedSize)). 휴지통을 비워야 디스크 여유가 늘어납니다."
        if movedCount > 0, errors.isEmpty {
            return .success(message: "\(movedCount)개 항목을 휴지통으로 이동했습니다. \(movedText)", movedSize: movedSize)
        }
        if movedCount > 0 {
            return .partial(message: "일부 항목만 이동했습니다. \(movedText) \(errors.count)개 실패: \(errors[0])", movedSize: movedSize, errors: errors)
        }
        return .failure(message: "휴지통으로 이동하지 못했습니다: \(errors.first ?? "대상이 변경되었습니다")", errors: errors)
    }

    private func targets(for task: MaintenanceTask, homeURL: URL) -> [(path: String, minimumChildSize: Int64)] {
        let home = homeURL.standardizedFileURL.path
        switch task.type {
        case .clearUserCache:
            return [("\(home)/Library/Caches", 1_000_000)]
        case .clearSpotlightCache:
            return [
                ("\(home)/Library/Caches/com.apple.SpotlightIndex", 0),
                ("\(home)/Library/Metadata/CoreSpotlight", 0),
            ]
        case .clearIconCache:
            return [("\(home)/Library/Caches/com.apple.iconservices.store", 0)]
        case .clearFontCache:
            return [
                ("\(home)/Library/Caches/com.apple.FontRegistry", 0),
                ("\(home)/Library/Caches/ATS", 0),
            ]
        case .clearDNSNote, .clearLaunchServicesNote:
            return []
        }
    }

    private func inspectRoot(path: String, minimumChildSize: Int64) -> MaintenancePreview {
        let rootPath = URL(fileURLWithPath: path).standardizedFileURL.path
        guard let rootSnapshot = FileIdentitySnapshot.capture(path: rootPath) else {
            return MaintenancePreview(roots: [], candidates: [], errors: [])
        }

        let approvedRoot = MaintenanceApprovedRoot(path: rootPath, snapshot: rootSnapshot)
        if rootSnapshot.kind == .regularFile {
            return MaintenancePreview(
                roots: [approvedRoot],
                candidates: [MaintenanceMoveCandidate(
                    name: (rootPath as NSString).lastPathComponent,
                    path: rootPath,
                    rootPath: rootPath,
                    logicalSize: rootSnapshot.size,
                    snapshot: rootSnapshot
                )],
                errors: []
            )
        }

        do {
            let children = try fm.contentsOfDirectory(
                at: URL(fileURLWithPath: rootPath),
                includingPropertiesForKeys: nil,
                options: []
            )
            var candidates: [MaintenanceMoveCandidate] = []
            for child in children {
                let childPath = child.standardizedFileURL.path
                guard rootSnapshot.exactlyMatches(path: rootPath),
                      isSameOrDescendant(childPath, of: rootPath),
                      childPath != rootPath,
                      let snapshot = FileIdentitySnapshot.capture(path: childPath) else { continue }
                let logicalSize = logicalSize(path: childPath, snapshot: snapshot)
                guard logicalSize >= minimumChildSize else { continue }
                candidates.append(MaintenanceMoveCandidate(
                    name: child.lastPathComponent,
                    path: childPath,
                    rootPath: rootPath,
                    logicalSize: logicalSize,
                    snapshot: snapshot
                ))
            }
            return MaintenancePreview(roots: [approvedRoot], candidates: candidates, errors: [])
        } catch {
            return MaintenancePreview(
                roots: [approvedRoot],
                candidates: [],
                errors: ["\((rootPath as NSString).lastPathComponent): \(error.localizedDescription)"]
            )
        }
    }

    private func logicalSize(path: String, snapshot: FileIdentitySnapshot) -> Int64 {
        if snapshot.kind == .regularFile { return snapshot.size }
        var total: Int64 = 0
        guard let enumerator = fm.enumerator(atPath: path) else { return 0 }
        while let relative = enumerator.nextObject() as? String {
            let childPath = (path as NSString).appendingPathComponent(relative)
            guard isSameOrDescendant(childPath, of: path),
                  let child = FileIdentitySnapshot.capture(path: childPath),
                  child.kind == .regularFile else { continue }
            total += child.size
        }
        return total
    }

    private func merge(_ lhs: MaintenancePreview, _ rhs: MaintenancePreview) -> MaintenancePreview {
        MaintenancePreview(
            roots: lhs.roots + rhs.roots,
            candidates: lhs.candidates + rhs.candidates,
            errors: lhs.errors + rhs.errors
        )
    }

    private func isSameOrDescendant(_ path: String, of root: String) -> Bool {
        path == root || path.hasPrefix(root.hasSuffix("/") ? root : root + "/")
    }
}
