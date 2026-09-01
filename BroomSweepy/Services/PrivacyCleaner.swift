import Foundation

struct BrowserData: Identifiable, Sendable {
    let id = UUID()
    let browserName: String
    let icon: String // SF Symbol
    let dataType: BrowserDataType
    let path: String
    let size: Int64
    let snapshot: FileIdentitySnapshot
    var sizeFormatted: String { ByteCountFormatter.string(fromByteCount: size, countStyle: .file) }

    init?(
        browserName: String,
        icon: String,
        dataType: BrowserDataType,
        path: String,
        size: Int64
    ) {
        guard let snapshot = FileIdentitySnapshot.capture(path: path) else { return nil }
        self.browserName = browserName
        self.icon = icon
        self.dataType = dataType
        self.path = path
        self.size = size
        self.snapshot = snapshot
    }

    enum BrowserDataType: String, CaseIterable, Sendable {
        case history = "방문 기록"
        case cookies = "쿠키"
        case cache = "캐시"
        case downloads = "다운로드 기록"
        case localStorage = "로컬 스토리지"
        case sessions = "세션 데이터"

        var color: String {
            switch self {
            case .history: return "red"
            case .cookies: return "orange"
            case .cache: return "blue"
            case .downloads: return "purple"
            case .localStorage: return "teal"
            case .sessions: return "indigo"
            }
        }

        var icon: String {
            switch self {
            case .history: return "clock.arrow.circlepath"
            case .cookies: return "doc.text"
            case .cache: return "internaldrive"
            case .downloads: return "arrow.down.circle"
            case .localStorage: return "externaldrive"
            case .sessions: return "person.crop.rectangle"
            }
        }
    }
}

final class PrivacyCleaner {
    static let shared = PrivacyCleaner()
    private let fm = FileManager.default

    private init() {}

    func scan(homeURL: URL? = nil) -> [BrowserData] {
        let home = homeURL?.path ?? actualUserHomeURL().path
        var results: [BrowserData] = []

        // Chrome
        let chromeBase = "\(home)/Library/Application Support/Google/Chrome/Default"
        let chromeTargets: [(BrowserData.BrowserDataType, String)] = [
            (.history, "\(chromeBase)/History"),
            (.cookies, "\(chromeBase)/Cookies"),
            (.cache, "\(home)/Library/Caches/Google/Chrome"),
            (.localStorage, "\(chromeBase)/Local Storage"),
            (.sessions, "\(chromeBase)/Sessions"),
        ]
        for (type, path) in chromeTargets {
            let size = sizeOf(path)
            if size > 0 {
                if let item = BrowserData(browserName: "Chrome", icon: "globe", dataType: type, path: path, size: size) {
                    results.append(item)
                }
            }
        }

        // Safari
        let safariTargets: [(BrowserData.BrowserDataType, String)] = [
            (.history, "\(home)/Library/Safari/History.db"),
            (.cache, "\(home)/Library/Caches/com.apple.Safari"),
            (.localStorage, "\(home)/Library/Safari/LocalStorage"),
            (.downloads, "\(home)/Library/Safari/Downloads.plist"),
        ]
        for (type, path) in safariTargets {
            let size = sizeOf(path)
            if size > 0 {
                if let item = BrowserData(browserName: "Safari", icon: "safari", dataType: type, path: path, size: size) {
                    results.append(item)
                }
            }
        }

        // Firefox
        let firefoxProfiles = "\(home)/Library/Application Support/Firefox/Profiles"
        if let profilesSnapshot = FileIdentitySnapshot.capture(path: firefoxProfiles),
           profilesSnapshot.kind == .directory,
           let profiles = try? fm.contentsOfDirectory(atPath: firefoxProfiles) {
            for profile in profiles {
                guard profilesSnapshot.exactlyMatches(path: firefoxProfiles) else { break }
                let profilePath = "\(firefoxProfiles)/\(profile)"
                let ffTargets: [(BrowserData.BrowserDataType, String)] = [
                    (.history, "\(profilePath)/places.sqlite"),
                    (.cookies, "\(profilePath)/cookies.sqlite"),
                    (.cache, "\(home)/Library/Caches/Firefox/Profiles/\(profile)"),
                    (.sessions, "\(profilePath)/sessionstore-backups"),
                ]
                for (type, path) in ffTargets {
                    let size = sizeOf(path)
                    if size > 0 {
                        if let item = BrowserData(browserName: "Firefox", icon: "flame", dataType: type, path: path, size: size) {
                            results.append(item)
                        }
                    }
                }
            }
        }

        // Edge
        let edgeBase = "\(home)/Library/Application Support/Microsoft Edge/Default"
        let edgeTargets: [(BrowserData.BrowserDataType, String)] = [
            (.history, "\(edgeBase)/History"),
            (.cookies, "\(edgeBase)/Cookies"),
            (.cache, "\(home)/Library/Caches/Microsoft Edge"),
        ]
        for (type, path) in edgeTargets {
            let size = sizeOf(path)
            if size > 0 {
                if let item = BrowserData(browserName: "Edge", icon: "globe", dataType: type, path: path, size: size) {
                    results.append(item)
                }
            }
        }

        // Arc
        let arcBase = "\(home)/Library/Application Support/Arc"
        let arcCache = "\(home)/Library/Caches/company.thebrowser.Browser"
        for (type, path) in [(.cache, arcCache), (.localStorage, "\(arcBase)/StorageData")] as [(BrowserData.BrowserDataType, String)] {
            let size = sizeOf(path)
            if size > 0 {
                if let item = BrowserData(browserName: "Arc", icon: "globe", dataType: type, path: path, size: size) {
                    results.append(item)
                }
            }
        }

        // Brave
        let braveBase = "\(home)/Library/Application Support/BraveSoftware/Brave-Browser/Default"
        let braveTargets: [(BrowserData.BrowserDataType, String)] = [
            (.history, "\(braveBase)/History"),
            (.cookies, "\(braveBase)/Cookies"),
            (.cache, "\(home)/Library/Caches/BraveSoftware/Brave-Browser"),
            (.localStorage, "\(braveBase)/Local Storage"),
        ]
        for (type, path) in braveTargets {
            let size = sizeOf(path)
            if size > 0 {
                if let item = BrowserData(browserName: "Brave", icon: "shield", dataType: type, path: path, size: size) {
                    results.append(item)
                }
            }
        }

        return results.sorted { $0.size > $1.size }
    }

    func clean(
        items: [BrowserData],
        runningBrowsers: Set<String>
    ) -> (freed: Int64, errors: [String], movedIDs: Set<UUID>) {
        var freed: Int64 = 0
        var errors: [String] = []
        var movedIDs: Set<UUID> = []

        var acceptedPaths: [String] = []
        let candidates = items.sorted { pathDepth($0.path) < pathDepth($1.path) }
        let approvedHome = normalizedPath(actualUserHomeURL().path)

        for item in candidates {
            if runningBrowsers.contains(item.browserName) {
                errors.append("\(item.browserName) \(item.dataType.rawValue): 브라우저가 실행 중이라 캐시를 포함해 이동하지 않았습니다")
                continue
            }
            let normalized = normalizedPath(item.path)
            let resolved = URL(fileURLWithPath: normalized)
                .resolvingSymlinksInPath().standardizedFileURL.path
            guard isSameOrDescendant(normalized, of: approvedHome),
                  isSameOrDescendant(resolved, of: approvedHome) else {
                errors.append("\(item.browserName) \(item.dataType.rawValue): 승인된 홈 폴더 밖의 항목은 이동하지 않았습니다")
                continue
            }
            if acceptedPaths.contains(where: { isSameOrDescendant(normalized, of: $0) }) {
                continue
            }
            acceptedPaths.append(normalized)

            guard item.snapshot.exactlyMatches(path: normalized) else {
                errors.append("\(item.browserName) \(item.dataType.rawValue): 스캔 뒤 항목이 변경되어 이동하지 않았습니다")
                continue
            }

            if item.snapshot.kind == .directory {
                errors.append(
                    "\(item.browserName) \(item.dataType.rawValue): 폴더 내부 전체를 검토 당시와 " +
                    "동일하다고 증명할 수 없어 자동 이동하지 않았습니다. Finder에서 검토해 주세요"
                )
            } else {
                guard item.snapshot.exactlyMatches(path: normalized) else {
                    errors.append("\(item.browserName) \(item.dataType.rawValue): 최종 확인 중 변경되어 이동하지 않았습니다")
                    continue
                }
                let result = VerifiedFileMover.shared.moveToTrash(
                    path: normalized,
                    expectedSnapshot: item.snapshot
                )
                if result.succeeded {
                    freed += item.snapshot.size
                    movedIDs.insert(item.id)
                } else {
                    errors.append("\(item.browserName) \(item.dataType.rawValue): \(result.error ?? "휴지통으로 이동하지 못했습니다")")
                }
            }
        }
        return (freed, errors, movedIDs)
    }

    private func sizeOf(_ path: String) -> Int64 {
        guard let rootSnapshot = FileIdentitySnapshot.capture(path: path) else { return 0 }
        if rootSnapshot.kind == .regularFile { return rootSnapshot.size }
        var total: Int64 = 0
        guard let enumerator = fm.enumerator(atPath: path) else { return 0 }
        while let file = enumerator.nextObject() as? String {
            let full = (path as NSString).appendingPathComponent(file)
            guard let snapshot = FileIdentitySnapshot.capture(path: full),
                  snapshot.kind == .regularFile else { continue }
            total += snapshot.size
        }
        return total
    }

    private func normalizedPath(_ path: String) -> String {
        URL(fileURLWithPath: path).standardizedFileURL.path
    }

    private func pathDepth(_ path: String) -> Int {
        (normalizedPath(path) as NSString).pathComponents.count
    }

    private func isSameOrDescendant(_ path: String, of root: String) -> Bool {
        path == root || path.hasPrefix(root.hasSuffix("/") ? root : root + "/")
    }
}
