import Foundation

final class BrokenPlistCleaner {
    static let shared = BrokenPlistCleaner()
    private let fileManager = FileManager.default

    struct BrokenPlist: Identifiable {
        let id = UUID()
        let name: String
        let path: String
        let size: Int64
        let reason: Reason

        var sizeFormatted: String { formatSize(size) }

        enum Reason: String {
            case parseError = "파싱 불가"
            case orphaned = "고아 파일 (앱 없음)"
        }
    }

    /// Scan ~/Library/Preferences for broken or orphaned .plist files
    func scan(homeURL: URL? = nil, progressCallback: ((String, Double) -> Void)? = nil) -> [BrokenPlist] {
        let home: String
        if let url = homeURL {
            home = url.path
        } else if let bookmark = FileAccessManager.shared.loadBookmark() {
            home = bookmark.path
        } else {
            home = NSHomeDirectory()
        }

        let prefsPath = (home as NSString).appendingPathComponent("Library/Preferences")
        guard let files = try? fileManager.contentsOfDirectory(atPath: prefsPath) else {
            return []
        }

        let plistFiles = files.filter { $0.hasSuffix(".plist") }
        let installedBundleIDs = getInstalledBundleIDs()
        var results: [BrokenPlist] = []

        for (index, fileName) in plistFiles.enumerated() {
            progressCallback?(fileName, Double(index) / Double(plistFiles.count))

            let fullPath = (prefsPath as NSString).appendingPathComponent(fileName)
            let attrs = try? fileManager.attributesOfItem(atPath: fullPath)
            let size = (attrs?[.size] as? Int64) ?? 0

            // Check 1: Can the plist be parsed?
            if let data = fileManager.contents(atPath: fullPath) {
                do {
                    _ = try PropertyListSerialization.propertyList(from: data, options: [], format: nil)
                } catch {
                    results.append(BrokenPlist(
                        name: fileName,
                        path: fullPath,
                        size: size,
                        reason: .parseError
                    ))
                    continue
                }
            }

            // Check 2: Is it orphaned? (bundle ID doesn't match any installed app)
            let bundleID = fileName.replacingOccurrences(of: ".plist", with: "")
            // Skip system/Apple plists and common non-app plists
            if bundleID.hasPrefix("com.apple.") || bundleID.hasPrefix("Apple") ||
               bundleID.hasPrefix("loginwindow") || bundleID.hasPrefix("NSGlobal") ||
               !bundleID.contains(".") {
                continue
            }

            if !installedBundleIDs.contains(bundleID) {
                results.append(BrokenPlist(
                    name: fileName,
                    path: fullPath,
                    size: size,
                    reason: .orphaned
                ))
            }
        }

        return results.sorted { $0.size > $1.size }
    }

    /// Remove selected broken plist files
    func clean(plists: [BrokenPlist]) -> (freed: Int64, errors: [String]) {
        var totalFreed: Int64 = 0
        var errors: [String] = []

        for plist in plists {
            do {
                try fileManager.removeItem(atPath: plist.path)
                totalFreed += plist.size
            } catch {
                errors.append("\(plist.name): \(error.localizedDescription)")
            }
        }

        return (totalFreed, errors)
    }

    /// Get bundle IDs of all installed apps
    private func getInstalledBundleIDs() -> Set<String> {
        var bundleIDs = Set<String>()
        let appDirs = ["/Applications", "/System/Applications"]

        for dir in appDirs {
            guard let apps = try? fileManager.contentsOfDirectory(atPath: dir) else { continue }
            for app in apps where app.hasSuffix(".app") {
                let plistPath = (dir as NSString)
                    .appendingPathComponent(app)
                    .appending("/Contents/Info.plist")
                if let data = fileManager.contents(atPath: plistPath),
                   let plist = try? PropertyListSerialization.propertyList(from: data, options: [], format: nil) as? [String: Any],
                   let bundleID = plist["CFBundleIdentifier"] as? String {
                    bundleIDs.insert(bundleID)
                }
            }
        }

        // Also check ~/Applications
        let homeApps = (NSHomeDirectory() as NSString).appendingPathComponent("Applications")
        if let apps = try? fileManager.contentsOfDirectory(atPath: homeApps) {
            for app in apps where app.hasSuffix(".app") {
                let plistPath = (homeApps as NSString)
                    .appendingPathComponent(app)
                    .appending("/Contents/Info.plist")
                if let data = fileManager.contents(atPath: plistPath),
                   let plist = try? PropertyListSerialization.propertyList(from: data, options: [], format: nil) as? [String: Any],
                   let bundleID = plist["CFBundleIdentifier"] as? String {
                    bundleIDs.insert(bundleID)
                }
            }
        }

        return bundleIDs
    }
}
