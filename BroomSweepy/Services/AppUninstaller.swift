import Foundation

struct InstalledApp: Identifiable, Sendable {
    let id = UUID()
    let name: String
    let bundleIdentifier: String
    let path: String
    let size: Int64
    let bundleModified: Date?
    let snapshot: FileIdentitySnapshot
    var relatedFiles: [String]
    var relatedFilesSize: Int64

    var totalSize: Int64 { size }
    var sizeFormatted: String { ByteCountFormatter.string(fromByteCount: totalSize, countStyle: .file) }

    /// Bundle modification age only. This is reference information, not proof
    /// that the user no longer needs the app.
    var isUnmodifiedFor180Days: Bool {
        guard let modified = bundleModified else { return false }
        return Calendar.current.dateComponents([.day], from: modified, to: Date()).day ?? 0 > 180
    }
}

final class AppUninstaller {
    static let shared = AppUninstaller()
    private let fm = FileManager.default

    // MARK: - Scan

    func scanApps(
        homeURL: URL? = nil,
        shouldCancel: () -> Bool = { false }
    ) -> [InstalledApp] {
        let home = homeURL?.path ?? actualUserHomeURL().path
        var apps: [InstalledApp] = []

        let appDirs = ["/Applications"]

        for dir in appDirs {
            guard !shouldCancel() else { return [] }
            guard let contents = try? fm.contentsOfDirectory(atPath: dir) else { continue }
            for item in contents where item.hasSuffix(".app") {
                guard !shouldCancel() else { return [] }
                let appPath = "\(dir)/\(item)"
                guard let snapshot = FileIdentitySnapshot.capture(path: appPath),
                      snapshot.kind == .directory,
                      let bundle = Bundle(path: appPath),
                      let bundleId = bundle.bundleIdentifier,
                      !bundleId.isEmpty else { continue }

                let name = bundle.infoDictionary?["CFBundleDisplayName"] as? String
                    ?? bundle.infoDictionary?["CFBundleName"] as? String
                    ?? item.replacingOccurrences(of: ".app", with: "")

                let appSize = dirSize(appPath, shouldCancel: shouldCancel)
                guard !shouldCancel() else { return [] }
                let bundleModified = (try? fm.attributesOfItem(atPath: appPath))?[.modificationDate] as? Date

                let relatedPaths = findRelatedFiles(bundleId: bundleId, home: home)
                var relatedSize: Int64 = 0
                for path in relatedPaths {
                    guard !shouldCancel() else { return [] }
                    relatedSize += fileOrDirSize(path, shouldCancel: shouldCancel)
                }

                apps.append(InstalledApp(
                    name: name,
                    bundleIdentifier: bundleId,
                    path: appPath,
                    size: appSize,
                    bundleModified: bundleModified,
                    snapshot: snapshot,
                    relatedFiles: relatedPaths,
                    relatedFilesSize: relatedSize
                ))
            }
        }

        return apps.sorted { $0.totalSize > $1.totalSize }
    }

    // MARK: - Uninstall

    /// App bundles are directories whose children can change while an app is
    /// running or updating. Until scan-time recursive manifests are available,
    /// both the bundle and exact bundle-ID related paths remain review-only.
    func uninstall(app: InstalledApp) -> (freedSize: Int64, errors: [String], appMoved: Bool) {
        guard app.snapshot.kind == .directory,
              Bundle(path: app.path)?.bundleIdentifier == app.bundleIdentifier,
              app.snapshot.exactlyMatches(path: app.path) else {
            return (0, ["\(app.name): 스캔 뒤 앱 또는 bundle ID가 변경되어 이동하지 않았습니다"], false)
        }
        return (
            0,
            ["\(app.name): 앱 폴더 전체가 검토 당시와 같은지 확인할 수 없어 Finder 검토 전용입니다"],
            false
        )
    }

    // MARK: - Helpers

    private func findRelatedFiles(bundleId: String, home: String) -> [String] {
        var paths: [String] = []

        let candidates = [
            "\(home)/Library/Caches/\(bundleId)",
            "\(home)/Library/Preferences/\(bundleId).plist",
            "\(home)/Library/Application Support/\(bundleId)",
            "\(home)/Library/Logs/\(bundleId)",
            "\(home)/Library/Containers/\(bundleId)",
            "\(home)/Library/Saved Application State/\(bundleId).savedState",
            "\(home)/Library/WebKit/\(bundleId)",
            "\(home)/Library/HTTPStorages/\(bundleId)",
        ]

        for path in candidates where FileIdentitySnapshot.capture(path: path) != nil {
            paths.append(path)
        }

        return paths
    }

    private func dirSize(
        _ path: String,
        shouldCancel: () -> Bool = { false }
    ) -> Int64 {
        var total: Int64 = 0
        guard let enumerator = fm.enumerator(atPath: path) else { return 0 }
        while let file = enumerator.nextObject() as? String {
            guard !shouldCancel() else { return 0 }
            let fullPath = (path as NSString).appendingPathComponent(file)
            guard let snapshot = FileIdentitySnapshot.capture(path: fullPath),
                  snapshot.kind == .regularFile else { continue }
            total += snapshot.size
        }
        return total
    }

    private func fileOrDirSize(
        _ path: String,
        shouldCancel: () -> Bool = { false }
    ) -> Int64 {
        guard let snapshot = FileIdentitySnapshot.capture(path: path) else { return 0 }
        if snapshot.kind == .directory {
            return dirSize(path, shouldCancel: shouldCancel)
        }
        return snapshot.size
    }
}
