import AppKit
import Foundation

func actualUserHomeURL() -> URL {
    let path = NSHomeDirectoryForUser(NSUserName()) ?? ("/Users/" + NSUserName())
    return URL(fileURLWithPath: path)
        .standardizedFileURL
        .resolvingSymlinksInPath()
}

@MainActor
final class FileAccessManager {
    static let shared = FileAccessManager()

    private let homeBookmarkKey = "com.broomsweepy.bookmarks.home"
    private let legacyBookmarkKey = "com.broomsweepy.bookmarks"
    private var activeHomeURL: URL?
    private var activeFolderURLs: [URL] = []
    private var leasedURLsByCanonicalPath: [String: URL] = [:]

    /// 임의 폴더 권한은 앱 실행 중에만 유지하며 홈 bookmark 슬롯에
    /// 저장하지 않는다.
    func requestFolderAccess(message: String = "스캔할 폴더를 선택하세요") -> URL? {
        guard let url = chooseFolder(message: message, prompt: "스캔 허용") else {
            return nil
        }
        guard beginLeaseIfNeeded(for: url) else { return nil }
        let canonicalURL = url.standardizedFileURL.resolvingSymlinksInPath()
        if !activeFolderURLs.contains(canonicalURL) {
            activeFolderURLs.append(canonicalURL)
        }
        return canonicalURL
    }

    /// 실제 홈 폴더만 전용 bookmark 슬롯에 저장한다.
    func requestHomeAccess() -> URL? {
        if let url = loadBookmark() {
            return url
        }

        let expectedHome = actualUserHomeURL()
        guard let selected = chooseFolder(
            message: "BroomSweepy가 파일을 스캔하려면 홈 폴더 접근 권한이 필요합니다.",
            prompt: "홈 폴더 허용",
            directoryURL: expectedHome
        ) else {
            return nil
        }

        let canonicalSelected = selected.standardizedFileURL.resolvingSymlinksInPath()
        guard canonicalSelected.path == expectedHome.path else {
            let alert = NSAlert()
            alert.messageText = "홈 폴더를 선택해 주세요"
            alert.informativeText = "다른 폴더 권한은 홈 폴더 권한으로 저장하지 않습니다."
            alert.alertStyle = .warning
            alert.runModal()
            return nil
        }

        guard saveHomeBookmark(for: selected) else { return nil }
        guard beginLeaseIfNeeded(for: selected) else {
            UserDefaults.standard.removeObject(forKey: homeBookmarkKey)
            return nil
        }
        activeHomeURL = canonicalSelected
        return canonicalSelected
    }

    // MARK: - Security-Scoped Bookmarks

    func loadBookmark() -> URL? {
        if let activeHomeURL { return activeHomeURL }

        let storedDefaults = UserDefaults.standard
        guard let data = storedDefaults.data(forKey: homeBookmarkKey)
                ?? storedDefaults.data(forKey: legacyBookmarkKey) else { return nil }

        var isStale = false
        guard let url = try? URL(
            resolvingBookmarkData: data,
            options: .withSecurityScope,
            relativeTo: nil,
            bookmarkDataIsStale: &isStale
        ) else { return nil }

        let expectedHome = actualUserHomeURL().path
        guard url.standardizedFileURL.resolvingSymlinksInPath().path == expectedHome else {
            storedDefaults.removeObject(forKey: homeBookmarkKey)
            storedDefaults.removeObject(forKey: legacyBookmarkKey)
            return nil
        }

        if (isStale || storedDefaults.data(forKey: homeBookmarkKey) == nil),
           !saveHomeBookmark(for: url) { return nil }
        storedDefaults.removeObject(forKey: legacyBookmarkKey)

        let canonicalURL = url.standardizedFileURL.resolvingSymlinksInPath()
        guard beginLeaseIfNeeded(for: url) else { return nil }
        activeHomeURL = canonicalURL
        return canonicalURL
    }

    /// Releases an arbitrary folder lease when its screen no longer needs it.
    /// The home lease is deliberately retained for the app lifetime.
    func releaseFolderAccess(_ url: URL) {
        let canonicalURL = url.standardizedFileURL.resolvingSymlinksInPath()
        let key = canonicalURL.path
        guard activeHomeURL?.path != key,
              let leasedURL = leasedURLsByCanonicalPath.removeValue(forKey: key) else { return }
        leasedURL.stopAccessingSecurityScopedResource()
        activeFolderURLs.removeAll { $0.path == key }
    }

    /// 앱 수명 동안 성공한 startAccessing 호출과 균형을 맞춘다.
    func stopAccessingResources() {
        for url in leasedURLsByCanonicalPath.values {
            url.stopAccessingSecurityScopedResource()
        }
        leasedURLsByCanonicalPath.removeAll()
        activeHomeURL = nil
        activeFolderURLs.removeAll()
    }

    private func chooseFolder(
        message: String,
        prompt: String,
        directoryURL: URL? = nil
    ) -> URL? {
        let panel = NSOpenPanel()
        panel.message = message
        panel.prompt = prompt
        panel.canChooseFiles = false
        panel.canChooseDirectories = true
        panel.allowsMultipleSelection = false
        panel.canCreateDirectories = false
        panel.directoryURL = directoryURL ?? actualUserHomeURL()

        guard panel.runModal() == .OK else { return nil }
        return panel.url?.standardizedFileURL
    }

    private func saveHomeBookmark(for url: URL) -> Bool {
        guard let data = try? url.bookmarkData(
            options: .withSecurityScope,
            includingResourceValuesForKeys: nil,
            relativeTo: nil
        ) else { return false }
        UserDefaults.standard.set(data, forKey: homeBookmarkKey)
        return true
    }

    private func beginLeaseIfNeeded(for url: URL) -> Bool {
        let key = url.standardizedFileURL.resolvingSymlinksInPath().path
        if leasedURLsByCanonicalPath[key] != nil { return true }
        guard url.startAccessingSecurityScopedResource() else { return false }
        leasedURLsByCanonicalPath[key] = url
        return true
    }
}
