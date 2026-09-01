import Foundation

final class BrokenPlistCleaner {
    static let shared = BrokenPlistCleaner()
    private let fileManager = FileManager.default

    struct BrokenPlist: Identifiable, Sendable {
        let id = UUID()
        let name: String
        let path: String
        let size: Int64
        let reason: Reason
        let snapshot: FileIdentitySnapshot
        let approvedRootPath: String
        let approvedRootSnapshot: FileIdentitySnapshot

        var sizeFormatted: String { formatSize(size) }

        enum Reason: String, Sendable {
            case parseError = "파싱 불가"
            case orphaned = "설치 앱에서 확인되지 않음"
        }
    }

    /// Scan ~/Library/Preferences for broken or orphaned .plist files
    func scan(homeURL: URL? = nil, progressCallback: ((String, Double) -> Void)? = nil) -> [BrokenPlist] {
        guard let home = approvedHomePath(homeURL) else { return [] }

        let preferencesURL = URL(
            fileURLWithPath: (home as NSString).appendingPathComponent("Library/Preferences")
        ).standardizedFileURL
        guard let rootSnapshot = FileIdentitySnapshot.capture(path: preferencesURL.path),
              rootSnapshot.kind == .directory else { return [] }
        let resolvedPreferencesURL = preferencesURL.resolvingSymlinksInPath().standardizedFileURL
        guard preferencesURL.path == resolvedPreferencesURL.path else { return [] }
        let prefsPath = preferencesURL.path
        guard let files = try? fileManager.contentsOfDirectory(atPath: prefsPath) else {
            return []
        }

        let plistFiles = files.filter { $0.hasSuffix(".plist") }
        let installedBundleIDs = getInstalledBundleIDs()
        var results: [BrokenPlist] = []

        for (index, fileName) in plistFiles.enumerated() {
            progressCallback?(fileName, Double(index) / Double(plistFiles.count))

            let fullPath = (prefsPath as NSString).appendingPathComponent(fileName)
            guard let snapshot = FileIdentitySnapshot.capture(path: fullPath),
                  snapshot.kind == .regularFile else { continue }
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
                        reason: .parseError,
                        snapshot: snapshot,
                        approvedRootPath: prefsPath,
                        approvedRootSnapshot: rootSnapshot
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
                    reason: .orphaned,
                    snapshot: snapshot,
                    approvedRootPath: prefsPath,
                    approvedRootSnapshot: rootSnapshot
                ))
            }
        }

        guard rootSnapshot.exactlyMatches(path: prefsPath) else { return [] }
        return results.sorted { $0.size > $1.size }
    }

    /// Remove selected broken plist files
    func clean(plists: [BrokenPlist]) -> (freed: Int64, errors: [String], movedIDs: Set<UUID>) {
        var totalFreed: Int64 = 0
        var errors: [String] = []
        var movedIDs: Set<UUID> = []

        var activeRoots: [String: (reviewed: FileIdentitySnapshot, current: FileIdentitySnapshot)] = [:]

        for plist in plists {
            guard plist.reason == .parseError else {
                errors.append("\(plist.name): 설치 앱에서 확인되지 않은 항목은 Finder 검토 전용입니다")
                continue
            }
            guard let root = verifiedRoot(for: plist, activeRoots: &activeRoots),
                  isContained(plist.path, in: plist.approvedRootPath),
                  plist.snapshot.kind == .regularFile,
                  plist.snapshot.size == plist.size else {
                errors.append("\(plist.name): 스캔 뒤 파일 또는 환경설정 폴더가 변경되어 이동하지 않았습니다")
                continue
            }

            let result = VerifiedFileMover.shared.moveToTrash(
                path: plist.path,
                expectedSnapshot: plist.snapshot
            )
            refreshRoot(plist.approvedRootPath, reviewed: root.reviewed, activeRoots: &activeRoots)
            if result.succeeded {
                totalFreed += plist.size
                movedIDs.insert(plist.id)
            } else {
                errors.append("\(plist.name): \(result.error ?? "휴지통으로 이동하지 못했습니다")")
            }
        }

        return (totalFreed, errors, movedIDs)
    }

    private func verifiedRoot(
        for item: BrokenPlist,
        activeRoots: inout [String: (reviewed: FileIdentitySnapshot, current: FileIdentitySnapshot)]
    ) -> (reviewed: FileIdentitySnapshot, current: FileIdentitySnapshot)? {
        if let active = activeRoots[item.approvedRootPath] {
            guard active.reviewed == item.approvedRootSnapshot,
                  active.current.exactlyMatches(path: item.approvedRootPath) else { return nil }
            return active
        }
        guard item.approvedRootSnapshot.kind == .directory,
              item.approvedRootSnapshot.exactlyMatches(path: item.approvedRootPath) else { return nil }
        let active = (reviewed: item.approvedRootSnapshot, current: item.approvedRootSnapshot)
        activeRoots[item.approvedRootPath] = active
        return active
    }

    private func refreshRoot(
        _ path: String,
        reviewed: FileIdentitySnapshot,
        activeRoots: inout [String: (reviewed: FileIdentitySnapshot, current: FileIdentitySnapshot)]
    ) {
        guard let refreshed = FileIdentitySnapshot.capture(path: path),
              refreshed.kind == .directory,
              refreshed.device == reviewed.device,
              refreshed.inode == reviewed.inode else {
            activeRoots.removeValue(forKey: path)
            return
        }
        activeRoots[path] = (reviewed, refreshed)
    }

    private func isContained(_ path: String, in rootPath: String) -> Bool {
        let root = URL(fileURLWithPath: rootPath).resolvingSymlinksInPath().standardizedFileURL.path
        let candidate = URL(fileURLWithPath: path).resolvingSymlinksInPath().standardizedFileURL.path
        return candidate.hasPrefix(root.hasSuffix("/") ? root : root + "/")
    }

    private func approvedHomePath(_ suppliedURL: URL?) -> String? {
        let approved = actualUserHomeURL().resolvingSymlinksInPath().standardizedFileURL.path
        let supplied = (suppliedURL ?? actualUserHomeURL()).standardizedFileURL
        guard supplied.resolvingSymlinksInPath().standardizedFileURL.path == approved else {
            return nil
        }
        return approved
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
