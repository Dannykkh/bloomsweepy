import SwiftUI

@main
struct BroomSweepyApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) var appDelegate
    @State private var menuBarMonitor = MenuBarMonitor()
    @Environment(\.openWindow) private var openWindow

    var body: some Scene {
        WindowGroup(id: "main") {
            RootView()
        }
        .windowStyle(.titleBar)
        .defaultSize(width: 1100, height: 750)
        .commands {
            CommandGroup(after: .appInfo) {
                Button("설정...") {
                    // 설정은 사이드바에서 접근
                }
                .keyboardShortcut(",")
            }
        }

        MenuBarExtra {
            MenuBarPopover(monitor: menuBarMonitor, onOpen: {
                openMainWindow()
            })
        } label: {
            HStack(spacing: 4) {
                Image(systemName: "sparkles")
                Text("\(menuBarMonitor.ramUsage)%")
                    .font(.system(size: 11, weight: .medium, design: .rounded))
                    .monospacedDigit()
            }
        }
        .menuBarExtraStyle(.window)
    }

    private func openMainWindow() {
        // 기존 윈도우가 있으면 활성화, 없으면 새로 생성
        NSApplication.shared.activate(ignoringOtherApps: true)
        let mainWindows = NSApplication.shared.windows.filter {
            $0.canBecomeMain && $0.title.contains("BroomSweepy") || $0.identifier?.rawValue.contains("main") == true
        }
        if let window = mainWindows.first {
            window.makeKeyAndOrderFront(nil)
        } else {
            // WindowGroup에 새 윈도우 요청
            openWindow(id: "main")
        }
    }
}

// MARK: - AppDelegate (창 닫아도 상주)

final class AppDelegate: NSObject, NSApplicationDelegate {
    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        false  // 창 닫아도 메뉴바에 상주
    }

    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool) -> Bool {
        if !flag {
            // Dock 아이콘 클릭 시에도 윈도우 복원
            for window in sender.windows {
                if window.canBecomeMain {
                    window.makeKeyAndOrderFront(nil)
                    return true
                }
            }
        }
        return true
    }
}

// MARK: - Menu Bar Popover (리치 팝업)

struct MenuBarPopover: View {
    let monitor: MenuBarMonitor
    let onOpen: () -> Void
    @State private var showContent = false

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Image(systemName: "sparkles")
                    .font(.title3)
                    .symbolRenderingMode(.multicolor)
                    .foregroundStyle(.linearGradient(
                        colors: [.blue, .purple],
                        startPoint: .topLeading,
                        endPoint: .bottomTrailing
                    ))
                Text("BroomSweepy")
                    .font(.headline)
                Spacer()
                Text("v1.0")
                    .font(.caption)
                    .foregroundStyle(.tertiary)
            }
            .padding(.horizontal, 16)
            .padding(.top, 14)
            .padding(.bottom, 10)

            Divider()

            // Gauges
            VStack(spacing: 14) {
                MiniGaugeRow(
                    icon: "memorychip.fill", label: "메모리",
                    value: monitor.ramUsage, unit: "%",
                    color: monitor.ramUsage < 60 ? .green : monitor.ramUsage < 80 ? .orange : .red,
                    show: showContent, delay: 0.1
                )
                MiniGaugeRow(
                    icon: "cpu", label: "CPU",
                    value: monitor.cpuUsage, unit: "%",
                    color: monitor.cpuUsage < 50 ? .green : monitor.cpuUsage < 80 ? .orange : .red,
                    show: showContent, delay: 0.15
                )
                MiniDiskRow(
                    freeText: monitor.diskFree,
                    usagePercent: monitor.diskUsagePercent,
                    show: showContent, delay: 0.2
                )
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 12)

            Divider()

            // Actions
            VStack(spacing: 2) {
                Button {
                    onOpen()
                } label: {
                    HStack {
                        Image(systemName: "macwindow")
                        Text("BroomSweepy 열기")
                        Spacer()
                        Text("⌘O")
                            .font(.caption)
                            .foregroundStyle(.tertiary)
                    }
                    .padding(.horizontal, 10)
                    .padding(.vertical, 7)
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .keyboardShortcut("o")

                Button {
                    NSApplication.shared.terminate(nil)
                } label: {
                    HStack {
                        Image(systemName: "power")
                        Text("종료")
                        Spacer()
                        Text("⌘Q")
                            .font(.caption)
                            .foregroundStyle(.tertiary)
                    }
                    .padding(.horizontal, 10)
                    .padding(.vertical, 7)
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .keyboardShortcut("q")
            }
            .padding(.horizontal, 8)
            .padding(.vertical, 6)
        }
        .frame(width: 280)
        .onAppear {
            withAnimation(.spring(duration: 0.5, bounce: 0.2).delay(0.05)) {
                showContent = true
            }
        }
        .onDisappear {
            showContent = false
        }
    }
}

// MARK: - Mini Gauge Row

private struct MiniGaugeRow: View {
    let icon: String
    let label: String
    let value: Int
    let unit: String
    let color: Color
    let show: Bool
    let delay: Double

    @State private var animatedProgress: CGFloat = 0

    var body: some View {
        HStack(spacing: 12) {
            Image(systemName: icon)
                .font(.body)
                .foregroundStyle(color)
                .frame(width: 20)

            Text(label)
                .font(.callout)
                .frame(width: 48, alignment: .leading)

            GeometryReader { geo in
                ZStack(alignment: .leading) {
                    Capsule().fill(color.opacity(0.12))
                    Capsule().fill(color.gradient)
                        .frame(width: geo.size.width * animatedProgress)
                }
            }
            .frame(height: 6)

            Text("\(value)\(unit)")
                .font(.system(size: 13, weight: .bold, design: .rounded))
                .monospacedDigit()
                .contentTransition(.numericText())
                .foregroundStyle(color)
                .frame(width: 42, alignment: .trailing)
        }
        .opacity(show ? 1 : 0)
        .offset(x: show ? 0 : -10)
        .animation(.spring(duration: 0.4, bounce: 0.2).delay(delay), value: show)
        .onAppear {
            withAnimation(.spring(duration: 0.8, bounce: 0.2).delay(delay + 0.1)) {
                animatedProgress = CGFloat(value) / 100.0
            }
        }
        .onChange(of: value) { _, newVal in
            withAnimation(.spring(duration: 0.5, bounce: 0.15)) {
                animatedProgress = CGFloat(newVal) / 100.0
            }
        }
    }
}

// MARK: - Mini Disk Row

private struct MiniDiskRow: View {
    let freeText: String
    let usagePercent: Int
    let show: Bool
    let delay: Double

    @State private var animatedProgress: CGFloat = 0

    var color: Color {
        usagePercent > 90 ? .red : usagePercent > 70 ? .orange : .green
    }

    var body: some View {
        HStack(spacing: 12) {
            Image(systemName: "internaldrive.fill")
                .font(.body)
                .foregroundStyle(color)
                .frame(width: 20)

            Text("디스크")
                .font(.callout)
                .frame(width: 48, alignment: .leading)

            GeometryReader { geo in
                ZStack(alignment: .leading) {
                    Capsule().fill(color.opacity(0.12))
                    Capsule().fill(color.gradient)
                        .frame(width: geo.size.width * animatedProgress)
                }
            }
            .frame(height: 6)

            Text(freeText)
                .font(.system(size: 11, weight: .medium, design: .rounded))
                .foregroundStyle(.secondary)
                .frame(width: 52, alignment: .trailing)
                .lineLimit(1)
                .minimumScaleFactor(0.8)
        }
        .opacity(show ? 1 : 0)
        .offset(x: show ? 0 : -10)
        .animation(.spring(duration: 0.4, bounce: 0.2).delay(delay), value: show)
        .onAppear {
            withAnimation(.spring(duration: 0.8, bounce: 0.2).delay(delay + 0.1)) {
                animatedProgress = CGFloat(usagePercent) / 100.0
            }
        }
        .onChange(of: usagePercent) { _, newVal in
            withAnimation(.spring(duration: 0.5, bounce: 0.15)) {
                animatedProgress = CGFloat(newVal) / 100.0
            }
        }
    }
}

// MARK: - Menu Bar Monitor

@Observable
final class MenuBarMonitor {
    var ramUsage: Int = 0
    var cpuUsage: Int = 0
    var diskFree: String = "—"
    var diskUsagePercent: Int = 0

    private var timer: Timer?

    init() {
        refresh()
        timer = Timer.scheduledTimer(withTimeInterval: 10.0, repeats: true) { [weak self] _ in
            self?.refresh()
        }
    }

    deinit {
        timer?.invalidate()
    }

    private func refresh() {
        let mem = MemoryManager.shared.getMemoryInfo()
        ramUsage = Int(mem.usagePercent)

        let cpu = SystemMonitor.shared.getCPUInfo()
        cpuUsage = Int(cpu.usage)

        let disk = SystemMonitor.shared.getDiskInfo()
        diskFree = disk.freeFormatted
        diskUsagePercent = Int(disk.usagePercent)
    }
}
