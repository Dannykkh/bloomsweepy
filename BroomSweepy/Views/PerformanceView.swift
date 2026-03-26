import SwiftUI
import AppKit

struct PerformanceView: View {
    @State private var runningApps: [RunningAppInfo] = []
    @State private var showConfirm = false
    @State private var appToQuit: RunningAppInfo?
    @State private var sortBy: SortOption = .name

    enum SortOption: String, CaseIterable {
        case name = "이름순"
        case memory = "메모리순"
    }

    var body: some View {
        VStack(spacing: 0) {
            header
            summaryBar
            appList
        }
        .onAppear { refreshApps() }
        .alert("앱 종료", isPresented: $showConfirm) {
            Button("취소", role: .cancel) {}
            Button("종료", role: .destructive) {
                if let app = appToQuit { terminateApp(app) }
            }
        } message: {
            if let app = appToQuit {
                Text("\(app.name)을(를) 종료하시겠습니까?\n저장하지 않은 데이터가 있을 수 있습니다.")
            }
        }
    }

    // MARK: - Header

    private var header: some View {
        HStack(spacing: 12) {
            Text("실행 중인 앱")
                .font(.title2.bold())
            Spacer()

            Picker("정렬", selection: $sortBy) {
                ForEach(SortOption.allCases, id: \.self) { Text($0.rawValue) }
            }
            .frame(width: 110)

            Button("새로고침") { refreshApps() }
                .buttonStyle(.bordered)
        }
        .padding(24)
    }

    // MARK: - Summary

    private var summaryBar: some View {
        HStack(spacing: 20) {
            let mem = MemoryManager.shared.getMemoryInfo()
            let cpu = SystemMonitor.shared.getCPUInfo()

            HStack(spacing: 8) {
                Circle()
                    .fill(cpu.usage > 80 ? .red : cpu.usage > 50 ? .orange : .green)
                    .frame(width: 8, height: 8)
                Text("CPU \(Int(cpu.usage))%")
                    .font(.callout.bold())
            }

            HStack(spacing: 8) {
                Circle()
                    .fill(mem.usagePercent > 80 ? .red : mem.usagePercent > 60 ? .orange : .green)
                    .frame(width: 8, height: 8)
                Text("메모리 \(Int(mem.usagePercent))%")
                    .font(.callout.bold())
            }

            Text("·")
                .foregroundStyle(.tertiary)

            Text("\(runningApps.count)개 앱 실행 중")
                .font(.callout)
                .foregroundStyle(.secondary)

            Spacer()
        }
        .padding(.horizontal, 24)
        .padding(.vertical, 10)
        .background(.bar)
    }

    // MARK: - App List

    private var appList: some View {
        let sorted: [RunningAppInfo] = {
            switch sortBy {
            case .name: return runningApps.sorted { $0.name < $1.name }
            case .memory: return runningApps.sorted { $0.isHeavy && !$1.isHeavy }
            }
        }()

        return List(sorted) { app in
            HStack(spacing: 14) {
                // 앱 아이콘
                Image(nsImage: app.icon)
                    .resizable()
                    .interpolation(.high)
                    .frame(width: 32, height: 32)

                // 이름 + 번들 ID
                VStack(alignment: .leading, spacing: 2) {
                    HStack(spacing: 6) {
                        Text(app.name)
                            .font(.callout.weight(.medium))
                        if app.isHeavy {
                            Text("리소스 높음")
                                .font(.system(size: 9, weight: .bold))
                                .padding(.horizontal, 5)
                                .padding(.vertical, 2)
                                .background(.orange.opacity(0.15), in: Capsule())
                                .foregroundStyle(.orange)
                        }
                        if app.isHidden {
                            Text("숨김")
                                .font(.system(size: 9))
                                .foregroundStyle(.tertiary)
                        }
                    }
                    Text(app.bundleId)
                        .font(.caption2)
                        .foregroundStyle(.tertiary)
                        .lineLimit(1)
                }

                Spacer()

                // PID
                Text("PID \(app.pid)")
                    .font(.caption)
                    .foregroundStyle(.tertiary)
                    .frame(width: 60)

                // 종료 버튼
                if app.canTerminate {
                    Button {
                        appToQuit = app
                        showConfirm = true
                    } label: {
                        Image(systemName: "xmark.circle")
                            .foregroundStyle(.red.opacity(0.7))
                    }
                    .buttonStyle(.plain)
                    .help("\(app.name) 종료")
                }
            }
            .padding(.vertical, 4)
        }
        .listStyle(.inset(alternatesRowBackgrounds: true))
    }

    // MARK: - Actions

    private func refreshApps() {
        let workspace = NSWorkspace.shared
        let apps = workspace.runningApplications

        // 알려진 리소스 많이 쓰는 앱
        let heavyBundleIds: Set<String> = [
            "com.google.Chrome", "com.google.Chrome.helper",
            "com.docker.docker", "com.microsoft.VSCode",
            "com.apple.dt.Xcode", "com.spotify.client",
            "com.tinyspeck.slackmacgap", "us.zoom.xos",
            "com.microsoft.teams2", "com.adobe.Photoshop",
            "com.figma.Desktop", "com.electron",
        ]

        runningApps = apps.compactMap { app -> RunningAppInfo? in
            guard let name = app.localizedName,
                  let bundleId = app.bundleIdentifier,
                  app.activationPolicy == .regular // GUI 앱만
            else { return nil }

            // 자기 자신 제외
            if bundleId == "com.broomsweepy.app" { return nil }

            return RunningAppInfo(
                name: name,
                bundleId: bundleId,
                pid: app.processIdentifier,
                icon: app.icon ?? NSImage(),
                isHidden: app.isHidden,
                isHeavy: heavyBundleIds.contains(bundleId),
                canTerminate: !bundleId.hasPrefix("com.apple."),
                nsApp: app
            )
        }
    }

    private func terminateApp(_ app: RunningAppInfo) {
        app.nsApp.terminate()
        // 잠시 후 목록 갱신
        DispatchQueue.main.asyncAfter(deadline: .now() + 1) {
            refreshApps()
        }
    }
}

// MARK: - Running App Info

struct RunningAppInfo: Identifiable {
    let id = UUID()
    let name: String
    let bundleId: String
    let pid: pid_t
    let icon: NSImage
    let isHidden: Bool
    let isHeavy: Bool
    let canTerminate: Bool
    let nsApp: NSRunningApplication
}
