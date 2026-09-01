import Foundation

struct MailAttachment: Identifiable, Hashable, Sendable {
    let id = UUID()
    let name: String
    let path: String
    let size: Int64
    let modified: Date
    let snapshot: FileIdentitySnapshot
    let approvedRootPath: String
    let approvedRootSnapshot: FileIdentitySnapshot

    var sizeFormatted: String { formatSize(size) }

    var ageDays: Int {
        Calendar.current.dateComponents([.day], from: modified, to: Date()).day ?? 0
    }
}

final class MailAttachmentCleaner {
    static let shared = MailAttachmentCleaner()
    private let fm = FileManager.default

    /// Scan mail download directories for attachments.
    func scan(homeURL: URL? = nil) -> [MailAttachment] {
        guard let home = approvedHomePath(homeURL) else { return [] }

        let searchPaths = [
            "\(home)/Library/Mail Downloads",
            "\(home)/Library/Containers/com.apple.mail/Data/Library/Mail Downloads",
        ]

        var results: [MailAttachment] = []

        for dir in searchPaths {
            let rootURL = URL(fileURLWithPath: dir).standardizedFileURL
            guard let rootSnapshot = FileIdentitySnapshot.capture(path: rootURL.path),
                  rootSnapshot.kind == .directory else { continue }
            let resolvedRootURL = rootURL.resolvingSymlinksInPath().standardizedFileURL
            guard rootURL.path == resolvedRootURL.path else { continue }
            let rootPath = rootURL.path
            var rootResults: [MailAttachment] = []
            collectFiles(
                in: rootPath,
                rootSnapshot: rootSnapshot,
                into: &rootResults
            )
            if rootSnapshot.exactlyMatches(path: rootPath) {
                results.append(contentsOf: rootResults)
            }
        }

        return results.sorted { $0.size > $1.size }
    }

    /// Move selected files to trash.
    func clean(items: [MailAttachment]) -> (freed: Int64, errors: [String], movedIDs: Set<UUID>) {
        var totalFreed: Int64 = 0
        var errors: [String] = []
        var movedIDs: Set<UUID> = []

        var activeRoots: [String: (reviewed: FileIdentitySnapshot, current: FileIdentitySnapshot)] = [:]

        for item in items {
            guard let root = verifiedRoot(for: item, activeRoots: &activeRoots),
                  isContained(item.path, in: item.approvedRootPath),
                  item.snapshot.kind == .regularFile,
                  item.snapshot.size == item.size else {
                errors.append("\(item.name): 스캔 뒤 파일 또는 메일 첨부 폴더가 변경되어 이동하지 않았습니다")
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

    // MARK: - Helpers

    private func collectFiles(
        in directory: String,
        rootSnapshot: FileIdentitySnapshot,
        into results: inout [MailAttachment]
    ) {
        guard let enumerator = fm.enumerator(atPath: directory) else { return }

        while let file = enumerator.nextObject() as? String {
            let fullPath = (directory as NSString).appendingPathComponent(file)
            let rootPath = URL(fileURLWithPath: directory).standardizedFileURL.path
            let normalized = URL(fileURLWithPath: fullPath).standardizedFileURL.path
            guard rootSnapshot.exactlyMatches(path: rootPath),
                  isContained(normalized, in: rootPath),
                  let snapshot = FileIdentitySnapshot.capture(path: normalized),
                  snapshot.kind == .regularFile,
                  let attrs = try? fm.attributesOfItem(atPath: fullPath) else { continue }

            let size = (attrs[.size] as? Int64) ?? 0
            let modified = (attrs[.modificationDate] as? Date) ?? Date()

            results.append(MailAttachment(
                name: (file as NSString).lastPathComponent,
                path: fullPath,
                size: size,
                modified: modified,
                snapshot: snapshot,
                approvedRootPath: rootPath,
                approvedRootSnapshot: rootSnapshot
            ))
        }
    }

    private func verifiedRoot(
        for item: MailAttachment,
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
