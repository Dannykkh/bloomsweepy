import Foundation
import AppKit

// MARK: - AppPermission Model

struct AppPermission: Identifiable, Sendable {
    let id = UUID()
    let appName: String
    let bundleId: String
    let permissionType: PermissionType
    let evidence: Evidence
    let appPath: String?

    @MainActor
    var icon: NSImage? {
        guard let path = appPath else { return nil }
        return NSWorkspace.shared.icon(forFile: path)
    }

    enum Evidence: String, Sendable {
        case commonlyUses = "일반적으로 사용하는 권한"
        case declaresUsage = "앱 설정에 사용 목적 선언"
    }

    enum PermissionType: String, CaseIterable, Sendable {
        case camera = "카메라"
        case microphone = "마이크"
        case location = "위치"
        case contacts = "연락처"
        case photos = "사진"
        case fullDiskAccess = "전체 디스크 접근"
        case accessibility = "접근성"
        case screenRecording = "화면 녹화"

        var icon: String {
            switch self {
            case .camera: return "camera.fill"
            case .microphone: return "mic.fill"
            case .location: return "location.fill"
            case .contacts: return "person.crop.circle.fill"
            case .photos: return "photo.fill"
            case .fullDiskAccess: return "internaldrive.fill"
            case .accessibility: return "figure.stand"
            case .screenRecording: return "rectangle.dashed.badge.record"
            }
        }

        var color: String {
            switch self {
            case .camera: return "red"
            case .microphone: return "orange"
            case .location: return "blue"
            case .contacts: return "green"
            case .photos: return "purple"
            case .fullDiskAccess: return "gray"
            case .accessibility: return "teal"
            case .screenRecording: return "indigo"
            }
        }
    }
}

// MARK: - PermissionManager

final class PermissionManager {
    static let shared = PermissionManager()
    private let fm = FileManager.default

    private init() {}

    // MARK: - Scan

    func scan(homeURL: URL? = nil, progressCallback: ((String, Double) -> Void)? = nil) -> [AppPermission] {
        let home = homeURL?.path ?? ("/Users/" + NSUserName())
        var permissions: [AppPermission] = []

        progressCallback?("앱의 권한 사용 근거 수집 중...", 0.1)

        // Gather all installed apps
        let apps = gatherInstalledApps()

        progressCallback?("권한 데이터베이스 분석 중...", 0.3)

        // Check known permission-requiring paths
        permissions.append(contentsOf: checkAccessibilityClients(apps: apps))
        progressCallback?("접근성 권한 확인 중...", 0.5)

        permissions.append(contentsOf: checkScreenRecordingApps(apps: apps))
        progressCallback?("화면 녹화 권한 확인 중...", 0.6)

        permissions.append(contentsOf: checkFullDiskAccessApps(apps: apps))
        progressCallback?("전체 디스크 접근 확인 중...", 0.7)

        permissions.append(contentsOf: checkCameraAndMicApps(apps: apps, home: home))
        progressCallback?("카메라/마이크 권한 확인 중...", 0.85)

        progressCallback?("스캔 완료", 1.0)
        return permissions.sorted { $0.appName < $1.appName }
    }

    // MARK: - Open System Settings

    @MainActor
    func openSystemSettings(for permissionType: AppPermission.PermissionType) {
        let urlString: String
        switch permissionType {
        case .camera:
            urlString = "x-apple.systempreferences:com.apple.preference.security?Privacy_Camera"
        case .microphone:
            urlString = "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone"
        case .location:
            urlString = "x-apple.systempreferences:com.apple.preference.security?Privacy_LocationServices"
        case .contacts:
            urlString = "x-apple.systempreferences:com.apple.preference.security?Privacy_Contacts"
        case .photos:
            urlString = "x-apple.systempreferences:com.apple.preference.security?Privacy_Photos"
        case .fullDiskAccess:
            urlString = "x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles"
        case .accessibility:
            urlString = "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"
        case .screenRecording:
            urlString = "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture"
        }

        if let url = URL(string: urlString) {
            NSWorkspace.shared.open(url)
        }
    }

    // MARK: - Private Methods

    private struct AppInfo {
        let name: String
        let bundleId: String
        let path: String
    }

    private func gatherInstalledApps() -> [AppInfo] {
        var apps: [AppInfo] = []
        let appDirs = ["/Applications", "/System/Applications"]

        for dir in appDirs {
            guard let contents = try? fm.contentsOfDirectory(atPath: dir) else { continue }
            for item in contents where item.hasSuffix(".app") {
                let appPath = "\(dir)/\(item)"
                guard let bundle = Bundle(path: appPath) else { continue }
                let bundleId = bundle.bundleIdentifier ?? ""
                let name = bundle.infoDictionary?["CFBundleDisplayName"] as? String
                    ?? bundle.infoDictionary?["CFBundleName"] as? String
                    ?? item.replacingOccurrences(of: ".app", with: "")
                apps.append(AppInfo(name: name, bundleId: bundleId, path: appPath))
            }
        }
        return apps
    }

    /// Check accessibility clients by examining known TCC-related paths
    private func checkAccessibilityClients(apps: [AppInfo]) -> [AppPermission] {
        var results: [AppPermission] = []

        // Well-known apps that typically request accessibility
        let knownAccessibilityBundles: Set<String> = [
            "com.hegenberg.BetterTouchTool", "com.lwouis.alt-tab-macos",
            "org.hammerspoon.Hammerspoon", "com.knollsoft.Rectangle",
            "com.googlecode.iterm2", "com.sublimetext.4",
            "com.microsoft.VSCode", "com.jetbrains.intellij",
        ]

        for app in apps {
            if knownAccessibilityBundles.contains(app.bundleId) {
                results.append(AppPermission(
                    appName: app.name,
                    bundleId: app.bundleId,
                    permissionType: .accessibility,
                    evidence: .commonlyUses,
                    appPath: app.path
                ))
            }
        }
        return results
    }

    /// Check for apps that typically use screen recording
    private func checkScreenRecordingApps(apps: [AppInfo]) -> [AppPermission] {
        var results: [AppPermission] = []

        let knownScreenRecordingBundles: Set<String> = [
            "com.loom.desktop", "us.zoom.xos", "com.microsoft.teams",
            "com.tinyspeck.slackmacgap", "com.obsproject.obs-studio",
            "com.crowdcafe.windowmagnet", "com.getcleanshot.app",
        ]

        for app in apps {
            if knownScreenRecordingBundles.contains(app.bundleId) {
                results.append(AppPermission(
                    appName: app.name,
                    bundleId: app.bundleId,
                    permissionType: .screenRecording,
                    evidence: .commonlyUses,
                    appPath: app.path
                ))
            }
        }
        return results
    }

    /// Check apps that typically have full disk access
    private func checkFullDiskAccessApps(apps: [AppInfo]) -> [AppPermission] {
        var results: [AppPermission] = []

        let knownFDABundles: Set<String> = [
            "com.apple.Terminal", "com.googlecode.iterm2",
            "com.microsoft.VSCode", "com.sublimetext.4",
        ]

        for app in apps {
            if knownFDABundles.contains(app.bundleId) {
                results.append(AppPermission(
                    appName: app.name,
                    bundleId: app.bundleId,
                    permissionType: .fullDiskAccess,
                    evidence: .commonlyUses,
                    appPath: app.path
                ))
            }
        }
        return results
    }

    /// Check for apps with camera/microphone usage by inspecting their Info.plist
    private func checkCameraAndMicApps(apps: [AppInfo], home: String) -> [AppPermission] {
        var results: [AppPermission] = []

        for app in apps {
            let infoPlistPath = "\(app.path)/Contents/Info.plist"
            guard let plist = NSDictionary(contentsOfFile: infoPlistPath) else { continue }

            if plist["NSCameraUsageDescription"] != nil {
                results.append(AppPermission(
                    appName: app.name,
                    bundleId: app.bundleId,
                    permissionType: .camera,
                    evidence: .declaresUsage,
                    appPath: app.path
                ))
            }

            if plist["NSMicrophoneUsageDescription"] != nil {
                results.append(AppPermission(
                    appName: app.name,
                    bundleId: app.bundleId,
                    permissionType: .microphone,
                    evidence: .declaresUsage,
                    appPath: app.path
                ))
            }

            if plist["NSLocationUsageDescription"] != nil || plist["NSLocationWhenInUseUsageDescription"] != nil {
                results.append(AppPermission(
                    appName: app.name,
                    bundleId: app.bundleId,
                    permissionType: .location,
                    evidence: .declaresUsage,
                    appPath: app.path
                ))
            }

            if plist["NSContactsUsageDescription"] != nil {
                results.append(AppPermission(
                    appName: app.name,
                    bundleId: app.bundleId,
                    permissionType: .contacts,
                    evidence: .declaresUsage,
                    appPath: app.path
                ))
            }

            if plist["NSPhotoLibraryUsageDescription"] != nil {
                results.append(AppPermission(
                    appName: app.name,
                    bundleId: app.bundleId,
                    permissionType: .photos,
                    evidence: .declaresUsage,
                    appPath: app.path
                ))
            }
        }

        return results
    }
}
