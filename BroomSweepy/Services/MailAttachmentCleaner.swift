import Foundation

struct MailAttachment: Identifiable, Hashable {
    let id = UUID()
    let name: String
    let path: String
    let size: Int64
    let modified: Date

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
        let home = homeURL?.path ?? ("/Users/" + NSUserName())

        let searchPaths = [
            "\(home)/Library/Mail Downloads",
            "\(home)/Library/Containers/com.apple.mail/Data/Library/Mail Downloads",
        ]

        var results: [MailAttachment] = []

        for dir in searchPaths {
            guard fm.fileExists(atPath: dir) else { continue }
            collectFiles(in: dir, into: &results)
        }

        return results.sorted { $0.size > $1.size }
    }

    /// Move selected files to trash.
    func clean(paths: [String]) -> (freed: Int64, errors: [String]) {
        var totalFreed: Int64 = 0
        var errors: [String] = []

        for path in paths {
            do {
                let attrs = try fm.attributesOfItem(atPath: path)
                let size = (attrs[.size] as? Int64) ?? 0
                try fm.trashItem(at: URL(fileURLWithPath: path), resultingItemURL: nil)
                totalFreed += size
            } catch {
                errors.append("\(path): \(error.localizedDescription)")
            }
        }

        return (totalFreed, errors)
    }

    // MARK: - Helpers

    private func collectFiles(in directory: String, into results: inout [MailAttachment]) {
        guard let enumerator = fm.enumerator(atPath: directory) else { return }

        while let file = enumerator.nextObject() as? String {
            let fullPath = (directory as NSString).appendingPathComponent(file)
            guard let attrs = try? fm.attributesOfItem(atPath: fullPath),
                  attrs[.type] as? FileAttributeType != .typeDirectory else { continue }

            let size = (attrs[.size] as? Int64) ?? 0
            let modified = (attrs[.modificationDate] as? Date) ?? Date()

            results.append(MailAttachment(
                name: (file as NSString).lastPathComponent,
                path: fullPath,
                size: size,
                modified: modified
            ))
        }
    }
}
