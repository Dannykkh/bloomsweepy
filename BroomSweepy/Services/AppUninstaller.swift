import Foundation
import AppKit

struct InstalledApp: Identifiable {
    let id = UUID()
    let name: String
    let bundleIdentifier: String
    let path: String
    let icon: NSImage?
    let size: Int64
    let lastUsed: Date?
    var relatedFiles: [String]
    var relatedFilesSize: Int64

    var totalSize: Int64 { size + relatedFilesSize }
    var sizeFormatted: String { ByteCountFormatter.string(fromByteCount: totalSize, countStyle: .file) }

    /// True when the app bundle was not modified in 180+ days (used as a proxy for "unused").
    var isUnused: Bool {
        guard let last = lastUsed else { return true }
        return Calendar.current.dateComponents([.day], from: last, to: Date()).day ?? 0 > 180
    }
}

final class AppUninstaller {
    static let shared = AppUninstaller()
    private let fm = FileManager.default

    // MARK: - Scan

    func scanApps(homeURL: URL? = nil) -> [InstalledApp] {
        let home = homeURL?.path ?? ("/Users/" + NSUserName())
        var apps: [InstalledApp] = []

        let appDirs = ["/Applications"]

        for dir in appDirs {
            guard let contents = try? fm.contentsOfDirectory(atPath: dir) else { continue }
            for item in contents where item.hasSuffix(".app") {
                let appPath = "\(dir)/\(item)"
                guard let bundle = Bundle(path: appPath) else { continue }

                let bundleId = bundle.bundleIdentifier ?? item
                let name = bundle.infoDictionary?["CFBundleDisplayName"] as? String
                    ?? bundle.infoDictionary?["CFBundleName"] as? String
                    ?? item.replacingOccurrences(of: ".app", with: "")

                let icon = NSWorkspace.shared.icon(forFile: appPath)
                let appSize = dirSize(appPath)
                let lastUsed = (try? fm.attributesOfItem(atPath: appPath))?[.modificationDate] as? Date

                let relatedPaths = findRelatedFiles(bundleId: bundleId, appName: name, home: home)
                let relatedSize = relatedPaths.reduce(Int64(0)) { $0 + fileOrDirSize($1) }

                apps.append(InstalledApp(
                    name: name,
                    bundleIdentifier: bundleId,
                    path: appPath,
                    icon: icon,
                    size: appSize,
                    lastUsed: lastUsed,
                    relatedFiles: relatedPaths,
                    relatedFilesSize: relatedSize
                ))
            }
        }

        return apps.sorted { $0.totalSize > $1.totalSize }
    }

    // MARK: - Uninstall

    /// Moves the app bundle and all related files to Trash.
    /// Returns the total freed size and any per-path errors.
    func uninstall(app: InstalledApp) -> (freedSize: Int64, errors: [String]) {
        var freed: Int64 = 0
        var errors: [String] = []

        // 앱 본체 삭제 시도 (샌드박스에서 /Applications는 실패할 수 있음)
        do {
            try fm.trashItem(at: URL(fileURLWithPath: app.path), resultingItemURL: nil)
            freed += app.size
        } catch {
            errors.append("앱 삭제에 권한이 필요합니다. Finder에서 직접 삭제해주세요.")
        }

        // 관련 파일 (~/Library 하위) — 샌드박스에서 삭제 가능
        for path in app.relatedFiles {
            do {
                let size = fileOrDirSize(path)
                try fm.trashItem(at: URL(fileURLWithPath: path), resultingItemURL: nil)
                freed += size
            } catch {
                // 삭제 실패한 관련 파일은 무시 (권한 문제)
            }
        }

        return (freed, errors)
    }

    // MARK: - Helpers

    private func findRelatedFiles(bundleId: String, appName: String, home: String) -> [String] {
        var paths: [String] = []

        let candidates = [
            "\(home)/Library/Caches/\(bundleId)",
            "\(home)/Library/Preferences/\(bundleId).plist",
            "\(home)/Library/Application Support/\(bundleId)",
            "\(home)/Library/Application Support/\(appName)",
            "\(home)/Library/Logs/\(bundleId)",
            "\(home)/Library/Logs/\(appName)",
            "\(home)/Library/Containers/\(bundleId)",
            "\(home)/Library/Saved Application State/\(bundleId).savedState",
            "\(home)/Library/WebKit/\(bundleId)",
            "\(home)/Library/HTTPStorages/\(bundleId)",
        ]

        for path in candidates where fm.fileExists(atPath: path) {
            paths.append(path)
        }

        // Group Containers: scan once and match by bundleId
        let groupPath = "\(home)/Library/Group Containers"
        if let groups = try? fm.contentsOfDirectory(atPath: groupPath) {
            for group in groups where group.contains(bundleId) {
                paths.append("\(groupPath)/\(group)")
            }
        }

        return paths
    }

    private func dirSize(_ path: String) -> Int64 {
        var total: Int64 = 0
        guard let enumerator = fm.enumerator(atPath: path) else { return fileOrDirSize(path) }
        while let file = enumerator.nextObject() as? String {
            let fullPath = (path as NSString).appendingPathComponent(file)
            if let attrs = try? fm.attributesOfItem(atPath: fullPath),
               attrs[.type] as? FileAttributeType != .typeDirectory {
                total += (attrs[.size] as? Int64) ?? 0
            }
        }
        return total
    }

    private func fileOrDirSize(_ path: String) -> Int64 {
        var isDir: ObjCBool = false
        guard fm.fileExists(atPath: path, isDirectory: &isDir) else { return 0 }
        if isDir.boolValue { return dirSize(path) }
        return (try? fm.attributesOfItem(atPath: path))?[.size] as? Int64 ?? 0
    }
}
