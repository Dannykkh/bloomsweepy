import Foundation

struct BrokenDownload: Identifiable, Hashable {
    let id = UUID()
    let name: String
    let path: String
    let size: Int64
    let reason: Reason

    var sizeFormatted: String { formatSize(size) }

    enum Reason: String {
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
        let home = homeURL?.path ?? ("/Users/" + NSUserName())
        let downloadsPath = "\(home)/Downloads"

        guard fm.fileExists(atPath: downloadsPath) else { return [] }

        var results: [BrokenDownload] = []

        guard let contents = try? fm.contentsOfDirectory(atPath: downloadsPath) else { return [] }

        for name in contents {
            let fullPath = (downloadsPath as NSString).appendingPathComponent(name)

            // Skip directories
            var isDir: ObjCBool = false
            guard fm.fileExists(atPath: fullPath, isDirectory: &isDir), !isDir.boolValue else { continue }

            guard let attrs = try? fm.attributesOfItem(atPath: fullPath) else { continue }
            let size = (attrs[.size] as? Int64) ?? 0

            // Check incomplete download extensions
            let ext = (name as NSString).pathExtension.lowercased()
            if incompleteExtensions.contains(ext) {
                results.append(BrokenDownload(
                    name: name, path: fullPath, size: size,
                    reason: .incompleteDownload
                ))
                continue
            }

            // Check resource fork files (start with "._")
            if name.hasPrefix("._") {
                results.append(BrokenDownload(
                    name: name, path: fullPath, size: size,
                    reason: .resourceFork
                ))
                continue
            }

            // Check zero-byte files
            if size == 0 {
                results.append(BrokenDownload(
                    name: name, path: fullPath, size: size,
                    reason: .zeroBytes
                ))
                continue
            }
        }

        return results.sorted { $0.size > $1.size }
    }

    /// Move selected broken downloads to trash.
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
}
