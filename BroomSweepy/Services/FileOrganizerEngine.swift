import Foundation
import CoreImage

// MARK: - Organize Plan

struct OrganizePlan: Identifiable {
    let id = UUID()
    let originalURL: URL
    let destinationURL: URL
    let action: String // "move", "rename"
    var executed = false
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

        guard let contents = try? fileManager.contentsOfDirectory(
            at: folderURL,
            includingPropertiesForKeys: [.contentModificationDateKey, .creationDateKey, .isDirectoryKey],
            options: [.skipsHiddenFiles]
        ) else { return [] }

        for fileURL in contents {
            guard let values = try? fileURL.resourceValues(forKeys: [.isDirectoryKey]),
                  values.isDirectory == false else { continue }

            let ext = fileURL.pathExtension.lowercased()
            let category = categorize(ext: ext)
            let fileName = fileURL.lastPathComponent
            let fileDate = getFileDate(url: fileURL)

            var destFolder = folderURL
            var newName = fileName

            // Sort by type
            if options.sortByType {
                destFolder = folderURL.appendingPathComponent(category.folderName)
            }

            // Photos: sort by EXIF date
            if options.sortPhotosByDate && category == .photo {
                let exifDate = getEXIFDate(url: fileURL) ?? fileDate
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

            let destURL = destFolder.appendingPathComponent(newName)

            // Only add if something changes
            if destURL != fileURL {
                plans.append(OrganizePlan(originalURL: fileURL, destinationURL: destURL, action: "move"))
            }
        }

        return plans
    }

    // MARK: - Execute

    func execute(plans: inout [OrganizePlan]) -> (moved: Int, errors: [String]) {
        var moved = 0
        var errors: [String] = []

        for i in plans.indices {
            let plan = plans[i]
            let destDir = plan.destinationURL.deletingLastPathComponent()

            do {
                if !fileManager.fileExists(atPath: destDir.path) {
                    try fileManager.createDirectory(at: destDir, withIntermediateDirectories: true)
                }

                // Handle name collision
                var finalURL = plan.destinationURL
                var counter = 1
                while fileManager.fileExists(atPath: finalURL.path) {
                    let stem = plan.destinationURL.deletingPathExtension().lastPathComponent
                    let ext = plan.destinationURL.pathExtension
                    let newName = "\(stem)_\(counter).\(ext)"
                    finalURL = destDir.appendingPathComponent(newName)
                    counter += 1
                }

                try fileManager.moveItem(at: plan.originalURL, to: finalURL)
                plans[i].executed = true
                moved += 1
            } catch {
                errors.append("\(plan.originalURL.lastPathComponent): \(error.localizedDescription)")
            }
        }

        return (moved, errors)
    }

    // MARK: - Undo

    func undo(plans: [OrganizePlan]) -> Int {
        var undone = 0
        for plan in plans.reversed() where plan.executed {
            do {
                let originalDir = plan.originalURL.deletingLastPathComponent()
                if !fileManager.fileExists(atPath: originalDir.path) {
                    try fileManager.createDirectory(at: originalDir, withIntermediateDirectories: true)
                }
                try fileManager.moveItem(at: plan.destinationURL, to: plan.originalURL)
                undone += 1
            } catch {
                continue
            }
        }
        return undone
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
}
