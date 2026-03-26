import AppKit
import Foundation

@MainActor
final class FileAccessManager {
    static let shared = FileAccessManager()

    private let bookmarkKey = "com.broomsweepy.bookmarks"

    /// 현재 접근 중인 URL을 thread-safe하게 저장
    private nonisolated(unsafe) var _activeURL: URL?

    /// 사용자에게 폴더 선택 다이얼로그 표시
    func requestFolderAccess(message: String = "스캔할 폴더를 선택하세요") -> URL? {
        let panel = NSOpenPanel()
        panel.message = message
        panel.prompt = "스캔 허용"
        panel.canChooseFiles = false
        panel.canChooseDirectories = true
        panel.allowsMultipleSelection = false
        panel.canCreateDirectories = false
        panel.directoryURL = FileManager.default.homeDirectoryForCurrentUser

        guard panel.runModal() == .OK, let url = panel.url else { return nil }

        saveBookmark(for: url)
        _activeURL = url
        return url
    }

    /// 홈 폴더 접근 요청
    func requestHomeAccess() -> URL? {
        if let url = loadBookmark() {
            return url
        }
        return requestFolderAccess(message: "BroomSweepy가 파일을 스캔하려면\n홈 폴더 접근 권한이 필요합니다.")
    }

    // MARK: - Security-Scoped Bookmarks

    private func saveBookmark(for url: URL) {
        guard let data = try? url.bookmarkData(
            options: .withSecurityScope,
            includingResourceValuesForKeys: nil,
            relativeTo: nil
        ) else { return }
        UserDefaults.standard.set(data, forKey: bookmarkKey)
    }

    nonisolated func loadBookmark() -> URL? {
        // 이미 접근 중이면 반환
        if let active = _activeURL { return active }

        guard let data = UserDefaults.standard.data(forKey: bookmarkKey) else { return nil }

        var isStale = false
        guard let url = try? URL(
            resolvingBookmarkData: data,
            options: .withSecurityScope,
            relativeTo: nil,
            bookmarkDataIsStale: &isStale
        ) else { return nil }

        if isStale {
            if let freshData = try? url.bookmarkData(
                options: .withSecurityScope,
                includingResourceValuesForKeys: nil,
                relativeTo: nil
            ) {
                UserDefaults.standard.set(freshData, forKey: bookmarkKey)
            }
        }

        // 접근 시작 (앱 수명 동안 유지)
        _ = url.startAccessingSecurityScopedResource()
        _activeURL = url
        return url
    }
}
