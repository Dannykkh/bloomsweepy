import Foundation

struct MaintenanceTask: Identifiable {
    let id = UUID()
    let name: String
    let description: String
    let icon: String
    var isRunning = false
    var isCompleted = false
    var result: String?
    let requiresAdmin: Bool

    enum TaskType {
        case clearUserCache
        case clearSpotlightCache
        case clearIconCache
        case clearFontCache
        case clearDNSNote
        case clearLaunchServicesNote
    }
    let type: TaskType
}

final class MaintenanceManager {
    static let shared = MaintenanceManager()
    private let fm = FileManager.default

    private init() {}

    func getAvailableTasks() -> [MaintenanceTask] {
        [
            MaintenanceTask(
                name: "사용자 캐시 정리",
                description: "앱별 캐시 데이터를 정리합니다",
                icon: "internaldrive",
                requiresAdmin: false,
                type: .clearUserCache
            ),
            MaintenanceTask(
                name: "Spotlight 캐시 정리",
                description: "Spotlight 검색 캐시를 정리합니다",
                icon: "magnifyingglass",
                requiresAdmin: false,
                type: .clearSpotlightCache
            ),
            MaintenanceTask(
                name: "아이콘 캐시 초기화",
                description: "앱 아이콘 캐시를 초기화합니다",
                icon: "photo",
                requiresAdmin: false,
                type: .clearIconCache
            ),
            MaintenanceTask(
                name: "폰트 캐시 정리",
                description: "시스템 폰트 캐시를 정리합니다",
                icon: "textformat",
                requiresAdmin: false,
                type: .clearFontCache
            ),
            MaintenanceTask(
                name: "DNS 캐시 초기화",
                description: "터미널에서 sudo dscacheutil -flushcache 실행이 필요합니다",
                icon: "network",
                requiresAdmin: true,
                type: .clearDNSNote
            ),
            MaintenanceTask(
                name: "Launch Services 재구축",
                description: "터미널에서 lsregister 명령 실행이 필요합니다",
                icon: "arrow.triangle.2.circlepath",
                requiresAdmin: true,
                type: .clearLaunchServicesNote
            ),
        ]
    }

    func runTask(_ task: MaintenanceTask, homeURL: URL? = nil) -> String {
        let home = homeURL?.path ?? FileAccessManager.shared.loadBookmark()?.path
            ?? ("/Users/" + NSUserName())

        switch task.type {
        case .clearUserCache:
            let cachePath = "\(home)/Library/Caches"
            var freedSize: Int64 = 0
            var cleanedCount = 0
            if let dirs = try? fm.contentsOfDirectory(atPath: cachePath) {
                for dir in dirs {
                    let fullPath = (cachePath as NSString).appendingPathComponent(dir)
                    var isDir: ObjCBool = false
                    guard fm.fileExists(atPath: fullPath, isDirectory: &isDir), isDir.boolValue else { continue }
                    let size = directorySize(fullPath)
                    if size > 1_000_000 { // 1MB 이상만
                        if let _ = try? fm.removeItem(atPath: fullPath) {
                            freedSize += size
                            cleanedCount += 1
                        }
                    }
                }
            }
            return freedSize > 0
                ? "\(cleanedCount)개 캐시 정리 완료 (\(formatSize(freedSize)) 확보)"
                : "정리할 캐시가 없습니다"

        case .clearSpotlightCache:
            let paths = [
                "\(home)/Library/Caches/com.apple.SpotlightIndex",
                "\(home)/Library/Metadata/CoreSpotlight"
            ]
            var freed: Int64 = 0
            for path in paths {
                if fm.fileExists(atPath: path) {
                    let size = directorySize(path)
                    try? fm.removeItem(atPath: path)
                    freed += size
                }
            }
            return freed > 0
                ? "Spotlight 캐시 정리 완료 (\(formatSize(freed)) 확보)"
                : "Spotlight 캐시가 없습니다"

        case .clearIconCache:
            let iconCache = "\(home)/Library/Caches/com.apple.iconservices.store"
            if fm.fileExists(atPath: iconCache) {
                let size = directorySize(iconCache)
                try? fm.removeItem(atPath: iconCache)
                return "아이콘 캐시 초기화 완료 (\(formatSize(size))). 재시작 후 적용됩니다"
            }
            return "아이콘 캐시가 없습니다"

        case .clearFontCache:
            let fontCaches = [
                "\(home)/Library/Caches/com.apple.FontRegistry",
                "\(home)/Library/Caches/ATS"
            ]
            var freed: Int64 = 0
            for path in fontCaches {
                if fm.fileExists(atPath: path) {
                    let size = directorySize(path)
                    try? fm.removeItem(atPath: path)
                    freed += size
                }
            }
            return freed > 0
                ? "폰트 캐시 정리 완료 (\(formatSize(freed)) 확보)"
                : "폰트 캐시가 없습니다"

        case .clearDNSNote:
            return "터미널에서 다음 명령을 실행하세요:\nsudo dscacheutil -flushcache && sudo killall -HUP mDNSResponder"

        case .clearLaunchServicesNote:
            return "터미널에서 다음 명령을 실행하세요:\n/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -kill -r -domain local -domain user"
        }
    }

    private func directorySize(_ path: String) -> Int64 {
        var total: Int64 = 0
        guard let enumerator = fm.enumerator(atPath: path) else { return 0 }
        while let file = enumerator.nextObject() as? String {
            let fullPath = (path as NSString).appendingPathComponent(file)
            if let attrs = try? fm.attributesOfItem(atPath: fullPath),
               attrs[.type] as? FileAttributeType != .typeDirectory {
                total += (attrs[.size] as? Int64) ?? 0
            }
        }
        return total
    }
}
