import Darwin
import Foundation
import CoreImage

// MARK: - Organize Plan

struct OrganizePlan: Identifiable, Sendable {
    let id = UUID()
    let originalURL: URL
    let destinationURL: URL
    let approvedRootURL: URL
    let approvedRootSnapshot: FileIdentitySnapshot
    let sourceSnapshot: FileIdentitySnapshot
    let action: String // "move", "rename"
    var executed = false
    var executedDestinationURL: URL? = nil
}

// MARK: - Organize Options

struct OrganizeOptions {
    var addDatePrefix = true
    var sortByType = true
    var sortPhotosByDate = true
    var sortScreenshots = true
}

// MARK: - File Organizer Engine

final class FileOrganizerEngine {
    static let shared = FileOrganizerEngine()
    private let fileManager = FileManager.default
    private let dateFormatter: DateFormatter = {
        let df = DateFormatter()
        df.dateFormat = "yyyy-MM-dd"
        return df
    }()
    private let monthFormatter: DateFormatter = {
        let df = DateFormatter()
        df.dateFormat = "MM-MMMM"
        df.locale = Locale(identifier: "en_US")
        return df
    }()

    // MARK: - Preview (Dry Run)

    func preview(folderURL: URL, options: OrganizeOptions) -> [OrganizePlan] {
        var plans: [OrganizePlan] = []

        let approvedRootURL = folderURL.standardizedFileURL
        guard let approvedRootSnapshot = FileIdentitySnapshot.capture(path: approvedRootURL.path),
              approvedRootSnapshot.kind == .directory else { return [] }

        guard let contents = try? fileManager.contentsOfDirectory(
            at: approvedRootURL,
            includingPropertiesForKeys: [.contentModificationDateKey, .creationDateKey, .isDirectoryKey],
            options: [.skipsHiddenFiles]
        ) else { return [] }

        for fileURL in contents {
            let sourceURL = fileURL.standardizedFileURL
            guard approvedRootSnapshot.exactlyMatches(path: approvedRootURL.path),
                  sourceURL.deletingLastPathComponent() == approvedRootURL,
                  let sourceSnapshot = FileIdentitySnapshot.capture(path: sourceURL.path),
                  sourceSnapshot.kind == .regularFile else { continue }

            let ext = sourceURL.pathExtension.lowercased()
            let category = categorize(ext: ext)
            let fileName = sourceURL.lastPathComponent
            let fileDate = getFileDate(url: sourceURL)

            var destFolder = folderURL
            var newName = fileName

            // Sort by type
            if options.sortByType {
                destFolder = folderURL.appendingPathComponent(category.folderName)
            }

            // Photos: sort by EXIF date
            if options.sortPhotosByDate && category == .photo {
                let exifDate = getEXIFDate(url: sourceURL) ?? fileDate
                let year = Calendar.current.component(.year, from: exifDate)
                let month = monthFormatter.string(from: exifDate)
                destFolder = folderURL
                    .appendingPathComponent("사진")
                    .appendingPathComponent("\(year)")
                    .appendingPathComponent(month)
            }

            // Screenshots
            if options.sortScreenshots && isScreenshot(fileName: fileName) {
                let yearMonth = dateFormatter.string(from: fileDate).prefix(7) // YYYY-MM
                destFolder = folderURL
                    .appendingPathComponent("스크린샷")
                    .appendingPathComponent(String(yearMonth))
            }

            // Date prefix
            if options.addDatePrefix && !hasDatePrefix(fileName: fileName) {
                let dateStr = dateFormatter.string(from: fileDate)
                newName = "\(dateStr)_\(fileName)"
            }

            let destURL = destFolder.appendingPathComponent(newName).standardizedFileURL

            // Only add if something changes
            if destURL != sourceURL,
               isSameOrDescendant(destURL.path, of: approvedRootURL.path) {
                plans.append(OrganizePlan(
                    originalURL: sourceURL,
                    destinationURL: destURL,
                    approvedRootURL: approvedRootURL,
                    approvedRootSnapshot: approvedRootSnapshot,
                    sourceSnapshot: sourceSnapshot,
                    action: "move"
                ))
            }
        }

        return plans.sorted { $0.originalURL.path < $1.originalURL.path }
    }

    // MARK: - Execute

    func execute(plans: inout [OrganizePlan]) -> (moved: Int, errors: [String]) {
        var moved = 0
        var errors: [String] = []

        for i in plans.indices {
            let plan = plans[i]
            let destDir = plan.destinationURL.deletingLastPathComponent()

            do {
                guard plan.sourceSnapshot.exactlyMatches(path: plan.originalURL.path) else {
                    throw OrganizerSafetyError.sourceChanged
                }
                try ensureDestinationDirectory(
                    destDir,
                    under: plan.approvedRootURL,
                    approvedRootSnapshot: plan.approvedRootSnapshot
                )

                // Handle name collision
                var finalURL = plan.destinationURL
                var counter = 1
                while entryExistsNoFollow(finalURL.path) {
                    let stem = plan.destinationURL.deletingPathExtension().lastPathComponent
                    let ext = plan.destinationURL.pathExtension
                    let newName = ext.isEmpty
                        ? "\(stem)_\(counter)"
                        : "\(stem)_\(counter).\(ext)"
                    finalURL = destDir.appendingPathComponent(newName)
                    counter += 1
                }

                guard plan.sourceSnapshot.exactlyMatches(path: plan.originalURL.path),
                      !entryExistsNoFollow(finalURL.path) else {
                    throw OrganizerSafetyError.sourceChanged
                }
                let result = VerifiedFileMover.shared.moveAtomically(
                    sourcePath: plan.originalURL.path,
                    destinationPath: finalURL.path,
                    expectedSnapshot: plan.sourceSnapshot
                )
                guard result.succeeded else {
                    throw OrganizerSafetyError.moveFailed(
                        result.error ?? "파일을 이동하지 못했습니다"
                    )
                }
                plans[i].executed = true
                let executedURL = result.resultingPath.map { URL(fileURLWithPath: $0) } ?? finalURL
                plans[i].executedDestinationURL = executedURL
                moved += 1
                if let warning = result.error {
                    errors.append("\(plan.originalURL.lastPathComponent): \(warning)")
                }
                if !plan.sourceSnapshot.exactlyMatches(path: executedURL.path) {
                    errors.append("\(plan.originalURL.lastPathComponent): 이동 뒤 파일 동일성을 확인하지 못했습니다. Finder에서 확인하세요")
                }
            } catch {
                errors.append("\(plan.originalURL.lastPathComponent): \(error.localizedDescription)")
            }
        }

        return (moved, errors)
    }

    // MARK: - Undo

    func undo(plans: [OrganizePlan]) -> (
        undone: Int,
        errors: [String],
        remaining: [OrganizePlan]
    ) {
        var undone = 0
        var errors: [String] = []
        var remaining: [OrganizePlan] = []
        for plan in plans.reversed() where plan.executed {
            do {
                guard let executedURL = plan.executedDestinationURL,
                      plan.sourceSnapshot.exactlyMatches(path: executedURL.path),
                      sameEntryIdentity(plan.approvedRootSnapshot, at: plan.approvedRootURL.path),
                      plan.originalURL.deletingLastPathComponent() == plan.approvedRootURL,
                      !entryExistsNoFollow(plan.originalURL.path) else {
                    throw OrganizerSafetyError.undoTargetChanged
                }
                let result = VerifiedFileMover.shared.moveAtomically(
                    sourcePath: executedURL.path,
                    destinationPath: plan.originalURL.path,
                    expectedSnapshot: plan.sourceSnapshot
                )
                guard result.succeeded else {
                    throw OrganizerSafetyError.moveFailed(
                        result.error ?? "파일을 원래 위치로 돌리지 못했습니다"
                    )
                }
                undone += 1
            } catch {
                errors.append("\(plan.originalURL.lastPathComponent): \(error.localizedDescription)")
                remaining.append(plan)
            }
        }
        return (undone, errors, Array(remaining.reversed()))
    }

    // MARK: - Helpers

    private enum FileOrgCategory: String {
        case photo = "사진"
        case video = "동영상"
        case document = "문서"
        case music = "음악"
        case archive = "압축파일"
        case installer = "설치파일"
        case screenshot = "스크린샷"
        case dev = "개발"
        case other = "기타"

        var folderName: String { rawValue }
    }

    private func categorize(ext: String) -> FileOrgCategory {
        let map: [FileOrgCategory: Set<String>] = [
            .photo: ["jpg", "jpeg", "png", "gif", "bmp", "tiff", "raw", "psd", "heic", "heif", "webp"],
            .video: ["mp4", "avi", "mov", "mkv", "wmv", "flv", "m4v", "webm"],
            .document: ["pdf", "doc", "docx", "pptx", "xlsx", "hwp", "txt", "rtf", "pages", "numbers", "keynote", "csv"],
            .music: ["mp3", "wav", "flac", "aac", "ogg", "m4a", "wma"],
            .archive: ["zip", "rar", "7z", "tar", "gz", "bz2"],
            .installer: ["dmg", "pkg", "iso"],
            .dev: ["swift", "py", "js", "ts", "json", "xml", "html", "css", "sql", "db", "sqlite"],
        ]
        for (cat, exts) in map {
            if exts.contains(ext) { return cat }
        }
        return .other
    }

    private func isScreenshot(fileName: String) -> Bool {
        let lower = fileName.lowercased()
        return lower.contains("screenshot") || lower.contains("스크린샷")
            || lower.hasPrefix("screen shot")
    }

    private func hasDatePrefix(fileName: String) -> Bool {
        // Check for YYYY-MM-DD_ pattern
        let pattern = #"^\d{4}-\d{2}-\d{2}_"#
        return fileName.range(of: pattern, options: .regularExpression) != nil
    }

    private func getFileDate(url: URL) -> Date {
        let values = try? url.resourceValues(forKeys: [.creationDateKey, .contentModificationDateKey])
        return values?.creationDate ?? values?.contentModificationDate ?? Date()
    }

    private func getEXIFDate(url: URL) -> Date? {
        guard let source = CGImageSourceCreateWithURL(url as CFURL, nil),
              let properties = CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [CFString: Any],
              let exif = properties[kCGImagePropertyExifDictionary] as? [CFString: Any],
              let dateStr = exif[kCGImagePropertyExifDateTimeOriginal] as? String else {
            return nil
        }
        let df = DateFormatter()
        df.dateFormat = "yyyy:MM:dd HH:mm:ss"
        return df.date(from: dateStr)
    }

    private func ensureDestinationDirectory(
        _ destination: URL,
        under approvedRoot: URL,
        approvedRootSnapshot: FileIdentitySnapshot
    ) throws {
        let root = approvedRoot.standardizedFileURL
        let target = destination.standardizedFileURL
        guard isSameOrDescendant(target.path, of: root.path),
              let rootSnapshot = FileIdentitySnapshot.capture(path: root.path),
              rootSnapshot.kind == .directory,
              rootSnapshot.device == approvedRootSnapshot.device,
              rootSnapshot.inode == approvedRootSnapshot.inode else {
            throw OrganizerSafetyError.unsafeDestination
        }

        let relative = target.path.dropFirst(root.path.count)
        var current = root
        for component in relative.split(separator: "/") {
            current.appendPathComponent(String(component), isDirectory: true)
            if entryExistsNoFollow(current.path) {
                guard let snapshot = FileIdentitySnapshot.capture(path: current.path),
                      snapshot.kind == .directory,
                      snapshot.device == approvedRootSnapshot.device else {
                    throw OrganizerSafetyError.unsafeDestination
                }
            } else {
                try fileManager.createDirectory(at: current, withIntermediateDirectories: false)
                guard let snapshot = FileIdentitySnapshot.capture(path: current.path),
                      snapshot.kind == .directory,
                      snapshot.device == approvedRootSnapshot.device else {
                    throw OrganizerSafetyError.unsafeDestination
                }
            }
        }
    }

    private func entryExistsNoFollow(_ path: String) -> Bool {
        var value = stat()
        return lstat(path, &value) == 0
    }

    private func sameEntryIdentity(_ snapshot: FileIdentitySnapshot, at path: String) -> Bool {
        guard let current = FileIdentitySnapshot.capture(path: path) else { return false }
        return current.device == snapshot.device
            && current.inode == snapshot.inode
            && current.kind == snapshot.kind
    }

    private func isSameOrDescendant(_ path: String, of root: String) -> Bool {
        path == root || path.hasPrefix(root.hasSuffix("/") ? root : root + "/")
    }
}

private enum OrganizerSafetyError: LocalizedError {
    case sourceChanged
    case unsafeDestination
    case undoTargetChanged
    case moveFailed(String)

    var errorDescription: String? {
        switch self {
        case .sourceChanged:
            return "미리보기 뒤 원본 파일이 변경되어 이동하지 않았습니다"
        case .unsafeDestination:
            return "선택한 폴더 안의 안전한 대상 경로를 확인하지 못했습니다"
        case .undoTargetChanged:
            return "이동한 파일 또는 원래 위치가 변경되어 되돌리지 않았습니다"
        case .moveFailed(let message):
            return message
        }
    }
}
