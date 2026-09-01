import Darwin
import Foundation

// MARK: - File Identity

/// A no-follow snapshot used to ensure that a reviewed filesystem entry is the
/// same entry immediately before it is moved to Trash. Symbolic links and
/// unsupported node kinds are deliberately not representable.
struct FileIdentitySnapshot: Hashable, Codable, Sendable {
    enum Kind: String, Hashable, Codable, Sendable {
        case regularFile
        case directory
    }

    let device: UInt64
    let inode: UInt64
    let kind: Kind
    let size: Int64
    let modificationSeconds: Int64
    let modificationNanoseconds: Int64

    static func capture(path: String) -> FileIdentitySnapshot? {
        var value = stat()
        guard lstat(path, &value) == 0 else { return nil }

        let type = value.st_mode & mode_t(S_IFMT)
        let kind: Kind
        switch type {
        case mode_t(S_IFREG):
            kind = .regularFile
        case mode_t(S_IFDIR):
            kind = .directory
        default:
            return nil
        }

        return FileIdentitySnapshot(
            device: UInt64(truncatingIfNeeded: value.st_dev),
            inode: UInt64(truncatingIfNeeded: value.st_ino),
            kind: kind,
            size: Int64(value.st_size),
            modificationSeconds: Int64(value.st_mtimespec.tv_sec),
            modificationNanoseconds: Int64(value.st_mtimespec.tv_nsec)
        )
    }

    func exactlyMatches(path: String) -> Bool {
        Self.capture(path: path) == self
    }
}

// MARK: - Cache Item

struct CacheItem: Identifiable, Hashable, Sendable {
    let id = UUID()
    let name: String
    let path: String
    let icon: String
    let description: String
    let size: Int64
    let fileCount: Int
    let type: CacheType
    let snapshot: FileIdentitySnapshot

    var sizeFormatted: String { formatSize(size) }

    enum CacheType: String, Sendable {
        case cache, dsStore, log, temp
    }
}

// MARK: - Large File

struct LargeFile: Identifiable, Hashable, Sendable {
    let id = UUID()
    let name: String
    let path: String
    let size: Int64
    let modified: Date
    let ext: String
    let category: FileCategory
    let snapshot: FileIdentitySnapshot

    var sizeFormatted: String { formatSize(size) }

    var ageDays: Int {
        Calendar.current.dateComponents([.day], from: modified, to: Date()).day ?? 0
    }

    enum FileCategory: String, CaseIterable, Sendable {
        case video = "동영상"
        case image = "이미지"
        case music = "음악"
        case document = "문서"
        case archive = "압축파일"
        case installer = "설치파일"
        case dev = "개발"
        case backup = "백업"
        case other = "기타"

        var icon: String {
            switch self {
            case .video: return "film"
            case .image: return "photo"
            case .music: return "music.note"
            case .document: return "doc.text"
            case .archive: return "archivebox"
            case .installer: return "externaldrive"
            case .dev: return "chevron.left.forwardslash.chevron.right"
            case .backup: return "clock.arrow.circlepath"
            case .other: return "doc"
            }
        }

        static func from(ext: String) -> FileCategory {
            let e = ext.lowercased()
            let map: [FileCategory: Set<String>] = [
                .video: [".mp4", ".avi", ".mov", ".mkv", ".wmv", ".flv", ".m4v"],
                .image: [".jpg", ".jpeg", ".png", ".gif", ".bmp", ".tiff", ".raw", ".psd", ".heic"],
                .music: [".mp3", ".wav", ".flac", ".aac", ".ogg", ".m4a"],
                .document: [".pdf", ".doc", ".docx", ".pptx", ".xlsx", ".hwp", ".txt", ".pages", ".numbers", ".keynote"],
                .archive: [".zip", ".rar", ".7z", ".tar", ".gz"],
                .installer: [".dmg", ".pkg", ".app", ".iso"],
                .dev: [".sql", ".db", ".sqlite", ".jar", ".war"],
                .backup: [".bak", ".backup", ".old", ".tmp"],
            ]
            for (category, exts) in map {
                if exts.contains(e) { return category }
            }
            return .other
        }
    }
}

// MARK: - Duplicate Group

struct DuplicateGroup: Identifiable, Sendable {
    let id = UUID()
    let hash: String
    let files: [DuplicateFile]
    let eachSize: Int64

    var count: Int { files.count }
    var wastedSize: Int64 { eachSize * Int64(files.count - 1) }
    var eachSizeFormatted: String { formatSize(eachSize) }
    var wastedSizeFormatted: String { formatSize(wastedSize) }
}

struct DuplicateFile: Identifiable, Hashable, Sendable {
    let id = UUID()
    let name: String
    let path: String
    let size: Int64
    let modified: Date
    let snapshot: FileIdentitySnapshot
}

// MARK: - Scan Summary

struct ScanSummary {
    var cacheSize: Int64 = 0
    var cacheCount: Int = 0
    var largeFilesSize: Int64 = 0
    var largeFilesCount: Int = 0
    var duplicateWaste: Int64 = 0
    var duplicateGroups: Int = 0

    var totalCleanable: Int64 { cacheSize + duplicateWaste }

    var cacheSizeFormatted: String { formatSize(cacheSize) }
    var largeFilesSizeFormatted: String { formatSize(largeFilesSize) }
    var duplicateWasteFormatted: String { formatSize(duplicateWaste) }
    var totalCleanableFormatted: String { formatSize(totalCleanable) }
}

// MARK: - Helpers

func formatSize(_ bytes: Int64) -> String {
    let formatter = ByteCountFormatter()
    formatter.countStyle = .file
    return formatter.string(fromByteCount: bytes)
}
