import Foundation

struct LoginItem: Identifiable, Sendable {
    let id = UUID()
    let name: String
    let path: String
    let bundleIdentifier: String?
    var isEnabled: Bool
    let type: LoginItemType

    enum LoginItemType: String, Sendable {
        case launchAgent = "백그라운드 에이전트"
        case loginItem = "로그인 항목"
    }
}

final class StartupManager {
    static let shared = StartupManager()
    private let fm = FileManager.default

    // MARK: - Scan

    func scanLoginItems(
        homeURL: URL? = nil,
        shouldCancel: () -> Bool = { false }
    ) -> [LoginItem] {
        let home = homeURL?.path ?? ("/Users/" + NSUserName())
        var items: [LoginItem] = []

        let dirs = [
            "\(home)/Library/LaunchAgents",
            "/Library/LaunchAgents",
        ]

        for dir in dirs {
            guard !shouldCancel() else { return [] }
            guard let files = try? fm.contentsOfDirectory(atPath: dir) else { continue }
            for file in files where file.hasSuffix(".plist") {
                guard !shouldCancel() else { return [] }
                let path = "\(dir)/\(file)"
                let plist = NSDictionary(contentsOfFile: path)
                let isDisabled = plist?["Disabled"] as? Bool ?? false
                let label = plist?["Label"] as? String
                    ?? file.replacingOccurrences(of: ".plist", with: "")

                items.append(LoginItem(
                    name: label,
                    path: path,
                    bundleIdentifier: label,
                    isEnabled: !isDisabled,
                    type: .launchAgent
                ))
            }
        }

        return items.sorted {
            $0.name.localizedCaseInsensitiveCompare($1.name) == .orderedAscending
        }
    }

    // MARK: - Toggle

    /// Writes the Disabled key into the plist on disk.
    /// Returns true when the write succeeds.
    @discardableResult
    func setEnabled(_ enabled: Bool, for item: LoginItem) -> Bool {
        guard let dict = NSMutableDictionary(contentsOfFile: item.path) else { return false }
        dict["Disabled"] = !enabled
        return dict.write(toFile: item.path, atomically: true)
    }

    @discardableResult
    func disableItem(_ item: LoginItem) -> Bool { setEnabled(false, for: item) }

    @discardableResult
    func enableItem(_ item: LoginItem) -> Bool { setEnabled(true, for: item) }
}
