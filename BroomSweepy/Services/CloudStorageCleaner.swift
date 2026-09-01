import Foundation

// MARK: - Cloud Models

struct CloudProvider: Identifiable, Sendable {
    let id = UUID()
    let name: String
    let icon: String
    let path: String
    let totalSize: Int64
    let fileCount: Int
    let snapshot: FileIdentitySnapshot
    var files: [CloudFile]

    var totalSizeFormatted: String { ByteCountFormatter.string(fromByteCount: totalSize, countStyle: .file) }
}

struct CloudFile: Identifiable, Hashable, Sendable {
    let id = UUID()
    let name: String
    let path: String
    let size: Int64
    let modified: Date
    let isOld: Bool // older than 6 months
    let snapshot: FileIdentitySnapshot

    var sizeFormatted: String { ByteCountFormatter.string(fromByteCount: size, countStyle: .file) }

    var ageDays: Int {
        Calendar.current.dateComponents([.day], from: modified, to: Date()).day ?? 0
    }
}

// MARK: - CloudStorageCleaner

final class CloudStorageCleaner {
    static let shared = CloudStorageCleaner()
    private let fm = FileManager.default

    private init() {}

    // MARK: - Scan

    func scan(homeURL: URL? = nil, progressCallback: ((String, Double) -> Void)? = nil) -> [CloudProvider] {
        let home = homeURL?.path ?? actualUserHomeURL().path
        var providers: [CloudProvider] = []

        let cloudTargets: [(name: String, icon: String, paths: [String])] = [
            ("iCloud Drive", "icloud", [
                "\(home)/Library/Mobile Documents/com~apple~CloudDocs",
            ]),
            ("Google Drive", "externaldrive.badge.icloud", [
                "\(home)/Google Drive",
                "\(home)/Library/CloudStorage/GoogleDrive-*",
            ]),
            ("OneDrive", "cloud", [
                "\(home)/Library/CloudStorage/OneDrive-*",
                "\(home)/OneDrive",
            ]),
            ("Dropbox", "shippingbox", [
                "\(home)/Dropbox",
                "\(home)/Library/CloudStorage/Dropbox",
            ]),
        ]

        for (i, target) in cloudTargets.enumerated() {
            let progress = Double(i) / Double(cloudTargets.count)
            progressCallback?("\(target.name) 스캔 중...", progress)

            let resolvedPaths = target.paths.flatMap { resolvePath($0) }

            for resolvedPath in resolvedPaths {
                guard let rootSnapshot = FileIdentitySnapshot.capture(path: resolvedPath),
                      rootSnapshot.kind == .directory else { continue }

                let (files, totalSize, fileCount) = scanCloudDirectory(resolvedPath)

                if fileCount > 0 {
                    providers.append(CloudProvider(
                        name: target.name,
                        icon: target.icon,
                        path: resolvedPath,
                        totalSize: totalSize,
                        fileCount: fileCount,
                        snapshot: rootSnapshot,
                        files: files
                    ))
                }
                break // Only take the first found path for each provider
            }
        }

        progressCallback?("스캔 완료", 1.0)
        return providers.sorted { $0.totalSize > $1.totalSize }
    }

    // MARK: - Reviewed Trash Move

    /// Moves reviewed filesystem entries to Trash. The sync provider decides
    /// whether that move is propagated to the remote copy.
    func trashReviewedFiles(files: [CloudFile]) -> (freed: Int64, errors: [String], movedIDs: Set<UUID>) {
        var totalFreed: Int64 = 0
        var errors: [String] = []
        var movedIDs: Set<UUID> = []

        for file in files {
            guard file.snapshot.kind == .regularFile,
                  file.snapshot.size == file.size,
                  file.snapshot.exactlyMatches(path: file.path) else {
                errors.append("\(file.name): 스캔 뒤 파일이 변경되어 이동하지 않았습니다")
                continue
            }
            let result = VerifiedFileMover.shared.moveToTrash(
                path: file.path,
                expectedSnapshot: file.snapshot
            )
            if result.succeeded {
                totalFreed += file.size
                movedIDs.insert(file.id)
            } else {
                errors.append("\(file.name): \(result.error ?? "휴지통으로 이동하지 못했습니다")")
            }
        }

        return (totalFreed, errors, movedIDs)
    }

    // MARK: - Private

    /// Resolve glob patterns like GoogleDrive-* to actual paths
    private func resolvePath(_ pattern: String) -> [String] {
        guard pattern.contains("*") else { return [pattern] }

        let dir = (pattern as NSString).deletingLastPathComponent
        let prefix = (pattern as NSString).lastPathComponent.replacingOccurrences(of: "*", with: "")

        guard let contents = try? fm.contentsOfDirectory(atPath: dir) else { return [] }

        return contents
            .filter { $0.hasPrefix(prefix) }
            .map { "\(dir)/\($0)" }
    }

    private func scanCloudDirectory(_ path: String) -> (files: [CloudFile], totalSize: Int64, fileCount: Int) {
        let sixMonthsAgo = Calendar.current.date(byAdding: .month, value: -6, to: Date()) ?? Date()
        var files: [CloudFile] = []
        var totalSize: Int64 = 0
        var fileCount = 0
        let minSizeForListing: Int64 = 1024 * 1024 // 1 MB

        let rootPath = URL(fileURLWithPath: path).standardizedFileURL.path
        guard let rootSnapshot = FileIdentitySnapshot.capture(path: rootPath),
              rootSnapshot.kind == .directory else { return ([], 0, 0) }
        let enumerator = fm.enumerator(
            at: URL(fileURLWithPath: path),
            includingPropertiesForKeys: [.fileSizeKey, .isDirectoryKey, .contentModificationDateKey],
            options: [.skipsHiddenFiles]
        ) { _, _ in true }

        while let url = enumerator?.nextObject() as? URL {
            let normalized = url.standardizedFileURL.path
            guard normalized.hasPrefix(rootPath.hasSuffix("/") ? rootPath : rootPath + "/"),
                  rootSnapshot.exactlyMatches(path: rootPath),
                  let snapshot = FileIdentitySnapshot.capture(path: normalized),
                  snapshot.kind == .regularFile,
                  let values = try? url.resourceValues(forKeys: [.fileSizeKey, .isDirectoryKey, .contentModificationDateKey]),
                  values.isDirectory == false,
                  let fileSize = values.fileSize else { continue }

            let size = Int64(fileSize)
            guard snapshot.size == size else { continue }
            totalSize += size
            fileCount += 1

            let modified = values.contentModificationDate ?? Date()
            let isOld = modified < sixMonthsAgo

            // Only add files >= 1MB to the listing to keep UI manageable
            if size >= minSizeForListing {
                files.append(CloudFile(
                    name: url.lastPathComponent,
                    path: url.path,
                    size: size,
                    modified: modified,
                    isOld: isOld,
                    snapshot: snapshot
                ))
            }
        }

        // Sort by size descending, limit to top 200
        files.sort { $0.size > $1.size }
        if files.count > 200 { files = Array(files.prefix(200)) }

        return (files, totalSize, fileCount)
    }
}
