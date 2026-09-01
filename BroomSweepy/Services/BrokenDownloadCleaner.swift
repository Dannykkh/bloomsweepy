import Foundation

struct BrokenDownload: Identifiable, Hashable, Sendable {
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
        case incompleteDownload = "미완료 다운로드"
        case resourceFork = "리소스 포크"
        case zeroBytes = "빈 파일 (0 bytes)"
    }
}

final class BrokenDownloadCleaner {
    static let shared = BrokenDownloadCleaner()
    private let fm = FileManager.default

    private let incompleteExtensions: Set<String> = [
        "crdownload", "part", "download", "tmp", "partial"
    ]

    /// Scan ~/Downloads for incomplete/broken downloads.
    func scan(homeURL: URL? = nil) -> [BrokenDownload] {
        guard let home = approvedHomePath(homeURL) else { return [] }
        let downloadsURL = URL(fileURLWithPath: home)
            .appendingPathComponent("Downloads", isDirectory: true)
            .standardizedFileURL
        guard let rootSnapshot = FileIdentitySnapshot.capture(path: downloadsURL.path),
              rootSnapshot.kind == .directory else { return [] }
        let resolvedDownloadsURL = downloadsURL.resolvingSymlinksInPath().standardizedFileURL
        guard downloadsURL.path == resolvedDownloadsURL.path else { return [] }
        let downloadsPath = downloadsURL.path

        var results: [BrokenDownload] = []

        guard let contents = try? fm.contentsOfDirectory(atPath: downloadsPath) else { return [] }

        for name in contents {
            let fullPath = (downloadsPath as NSString).appendingPathComponent(name)

            guard let snapshot = FileIdentitySnapshot.capture(path: fullPath),
                  snapshot.kind == .regularFile,
                  let attrs = try? fm.attributesOfItem(atPath: fullPath) else { continue }
            let size = (attrs[.size] as? Int64) ?? 0

            // Check incomplete download extensions
            let ext = (name as NSString).pathExtension.lowercased()
            if incompleteExtensions.contains(ext) {
                results.append(BrokenDownload(
                    name: name, path: fullPath, size: size,
                    reason: .incompleteDownload,
                    snapshot: snapshot,
                    approvedRootPath: downloadsPath,
                    approvedRootSnapshot: rootSnapshot
                ))
                continue
            }

            // Check resource fork files (start with "._")
            if name.hasPrefix("._") {
                results.append(BrokenDownload(
                    name: name, path: fullPath, size: size,
                    reason: .resourceFork,
                    snapshot: snapshot,
                    approvedRootPath: downloadsPath,
                    approvedRootSnapshot: rootSnapshot
                ))
                continue
            }

            // Check zero-byte files
            if size == 0 {
                results.append(BrokenDownload(
                    name: name, path: fullPath, size: size,
                    reason: .zeroBytes,
                    snapshot: snapshot,
                    approvedRootPath: downloadsPath,
                    approvedRootSnapshot: rootSnapshot
                ))
                continue
            }
        }

        guard rootSnapshot.exactlyMatches(path: downloadsPath) else { return [] }
        return results.sorted { $0.size > $1.size }
    }

    /// Move selected broken downloads to trash.
    func clean(items: [BrokenDownload]) -> (freed: Int64, errors: [String], movedIDs: Set<UUID>) {
        var totalFreed: Int64 = 0
        var errors: [String] = []
        var movedIDs: Set<UUID> = []

        var activeRoots: [String: (reviewed: FileIdentitySnapshot, current: FileIdentitySnapshot)] = [:]

        for item in items {
            guard let root = verifiedRoot(for: item, activeRoots: &activeRoots),
                  isContained(item.path, in: item.approvedRootPath),
                  item.snapshot.kind == .regularFile,
                  item.snapshot.size == item.size else {
                errors.append("\(item.name): 스캔 뒤 파일 또는 다운로드 폴더가 변경되어 이동하지 않았습니다")
                continue
            }

            let result = VerifiedFileMover.shared.moveToTrash(
                path: item.path,
                expectedSnapshot: item.snapshot
            )
            refreshRoot(item.approvedRootPath, reviewed: root.reviewed, activeRoots: &activeRoots)
            if result.succeeded {
                totalFreed += item.size
                movedIDs.insert(item.id)
            } else {
                errors.append("\(item.name): \(result.error ?? "휴지통으로 이동하지 못했습니다")")
            }
        }

        return (totalFreed, errors, movedIDs)
    }

    private func verifiedRoot(
        for item: BrokenDownload,
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
}
