import Foundation

struct BrowserData: Identifiable {
    let id = UUID()
    let browserName: String
    let icon: String // SF Symbol
    let dataType: BrowserDataType
    let path: String
    let size: Int64
    var sizeFormatted: String { ByteCountFormatter.string(fromByteCount: size, countStyle: .file) }

    enum BrowserDataType: String, CaseIterable {
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
        let home = homeURL?.path ?? ("/Users/" + NSUserName())
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
                results.append(BrowserData(browserName: "Chrome", icon: "globe", dataType: type, path: path, size: size))
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
                results.append(BrowserData(browserName: "Safari", icon: "safari", dataType: type, path: path, size: size))
            }
        }

        // Firefox
        let firefoxProfiles = "\(home)/Library/Application Support/Firefox/Profiles"
        if let profiles = try? fm.contentsOfDirectory(atPath: firefoxProfiles) {
            for profile in profiles {
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
                        results.append(BrowserData(browserName: "Firefox", icon: "flame", dataType: type, path: path, size: size))
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
                results.append(BrowserData(browserName: "Edge", icon: "globe", dataType: type, path: path, size: size))
            }
        }

        // Arc
        let arcBase = "\(home)/Library/Application Support/Arc"
        let arcCache = "\(home)/Library/Caches/company.thebrowser.Browser"
        for (type, path) in [(.cache, arcCache), (.localStorage, "\(arcBase)/StorageData")] as [(BrowserData.BrowserDataType, String)] {
            let size = sizeOf(path)
            if size > 0 {
                results.append(BrowserData(browserName: "Arc", icon: "globe", dataType: type, path: path, size: size))
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
                results.append(BrowserData(browserName: "Brave", icon: "shield", dataType: type, path: path, size: size))
            }
        }

        return results.sorted { $0.size > $1.size }
    }

    func clean(items: [BrowserData]) -> (freed: Int64, errors: [String]) {
        var freed: Int64 = 0
        var errors: [String] = []

        for item in items {
            do {
                let size = item.size
                if fm.fileExists(atPath: item.path) {
                    var isDir: ObjCBool = false
                    fm.fileExists(atPath: item.path, isDirectory: &isDir)
                    if isDir.boolValue {
                        // Delete contents but keep folder
                        if let contents = try? fm.contentsOfDirectory(atPath: item.path) {
                            for file in contents {
                                try? fm.removeItem(atPath: "\(item.path)/\(file)")
                            }
                        }
                    } else {
                        try fm.removeItem(atPath: item.path)
                    }
                    freed += size
                }
            } catch {
                errors.append("\(item.browserName) \(item.dataType.rawValue): \(error.localizedDescription)")
            }
        }
        return (freed, errors)
    }

    private func sizeOf(_ path: String) -> Int64 {
        var isDir: ObjCBool = false
        guard fm.fileExists(atPath: path, isDirectory: &isDir) else { return 0 }
        if !isDir.boolValue {
            return (try? fm.attributesOfItem(atPath: path))?[.size] as? Int64 ?? 0
        }
        var total: Int64 = 0
        guard let enumerator = fm.enumerator(atPath: path) else { return 0 }
        while let file = enumerator.nextObject() as? String {
            let full = (path as NSString).appendingPathComponent(file)
            total += (try? fm.attributesOfItem(atPath: full))?[.size] as? Int64 ?? 0
        }
        return total
    }
}
