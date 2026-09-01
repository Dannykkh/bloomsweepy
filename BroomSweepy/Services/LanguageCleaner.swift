import Foundation

final class LanguageCleaner {
    static let shared = LanguageCleaner()
    private let fileManager = FileManager.default

    /// Languages to keep (English + Base + user's Korean)
    private let keepLanguages: Set<String> = ["en.lproj", "Base.lproj", "ko.lproj"]

    struct LanguageResource: Identifiable, Sendable {
        let id = UUID()
        let appName: String
        let appPath: String
        let language: String  // e.g. "fr.lproj"
        let path: String
        let size: Int64

        var sizeFormatted: String { formatSize(size) }
    }

    /// Scan /Applications for unused .lproj folders
    func scan(progressCallback: ((String, Double) -> Void)? = nil) -> [LanguageResource] {
        var results: [LanguageResource] = []
        let appsDir = "/Applications"

        guard let appNames = try? fileManager.contentsOfDirectory(atPath: appsDir) else {
            return []
        }

        let apps = appNames.filter { $0.hasSuffix(".app") }

        for (index, appName) in apps.enumerated() {
            progressCallback?(appName, Double(index) / Double(apps.count))

            let appPath = (appsDir as NSString).appendingPathComponent(appName)
            let resourcesPath = (appPath as NSString).appendingPathComponent("Contents/Resources")

            guard fileManager.fileExists(atPath: resourcesPath),
                  let contents = try? fileManager.contentsOfDirectory(atPath: resourcesPath) else {
                continue
            }

            for item in contents {
                guard item.hasSuffix(".lproj"), !keepLanguages.contains(item) else { continue }

                let lprojPath = (resourcesPath as NSString).appendingPathComponent(item)
                let size = directorySize(lprojPath)
                guard size > 0 else { continue }

                results.append(LanguageResource(
                    appName: appName.replacingOccurrences(of: ".app", with: ""),
                    appPath: appPath,
                    language: item.replacingOccurrences(of: ".lproj", with: ""),
                    path: lprojPath,
                    size: size
                ))
            }
        }

        return results.sorted { $0.size > $1.size }
    }

    /// App language resources are review-only because changing a signed app
    /// bundle can invalidate its code signature.
    @available(*, unavailable, message: "앱 언어 리소스는 Finder 검토 전용입니다")
    func clean(resources: [LanguageResource]) -> (freed: Int64, errors: [String]) {
        guard !resources.isEmpty else { return (0, []) }
        return (0, ["앱 서명이 손상될 수 있어 자동 이동을 지원하지 않습니다. Finder에서 직접 검토해 주세요."])
    }

    /// Total size of removable language files
    func totalSize(of resources: [LanguageResource]) -> Int64 {
        resources.reduce(0) { $0 + $1.size }
    }

    private func directorySize(_ path: String) -> Int64 {
        guard let rootSnapshot = FileIdentitySnapshot.capture(path: path),
              rootSnapshot.kind == .directory else { return 0 }
        var total: Int64 = 0
        guard let enumerator = fileManager.enumerator(atPath: path) else { return 0 }
        while let file = enumerator.nextObject() as? String {
            let fullPath = (path as NSString).appendingPathComponent(file)
            guard let snapshot = FileIdentitySnapshot.capture(path: fullPath),
                  snapshot.kind == .regularFile else { continue }
            total += snapshot.size
        }
        return total
    }
}
