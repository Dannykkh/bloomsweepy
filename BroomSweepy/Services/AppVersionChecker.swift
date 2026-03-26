import Foundation
import AppKit

struct AppVersionInfo: Identifiable {
    let id = UUID()
    let name: String
    let bundleIdentifier: String
    let version: String
    let buildNumber: String
    let path: String
    let icon: NSImage?
    let size: Int64

    var sizeFormatted: String { formatSize(size) }
}

final class AppVersionChecker {
    static let shared = AppVersionChecker()
    private let fm = FileManager.default

    /// Scan /Applications for installed apps and read their version info.
    func scanInstalledApps() -> [AppVersionInfo] {
        var apps: [AppVersionInfo] = []
        let appDir = "/Applications"

        guard let contents = try? fm.contentsOfDirectory(atPath: appDir) else { return [] }

        for item in contents where item.hasSuffix(".app") {
            let appPath = "\(appDir)/\(item)"
            guard let bundle = Bundle(path: appPath) else { continue }

            let info = bundle.infoDictionary ?? [:]
            let name = info["CFBundleDisplayName"] as? String
                ?? info["CFBundleName"] as? String
                ?? item.replacingOccurrences(of: ".app", with: "")
            let bundleId = bundle.bundleIdentifier ?? item
            let version = info["CFBundleShortVersionString"] as? String ?? "알 수 없음"
            let build = info["CFBundleVersion"] as? String ?? ""
            let icon = NSWorkspace.shared.icon(forFile: appPath)
            let size = dirSize(appPath)

            apps.append(AppVersionInfo(
                name: name,
                bundleIdentifier: bundleId,
                version: version,
                buildNumber: build,
                path: appPath,
                icon: icon,
                size: size
            ))
        }

        return apps.sorted { $0.name.localizedCaseInsensitiveCompare($1.name) == .orderedAscending }
    }

    // MARK: - Helpers

    private func dirSize(_ path: String) -> Int64 {
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
