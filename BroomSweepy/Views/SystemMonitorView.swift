import SwiftUI

struct SystemMonitorView: View {
    @State private var cpuInfo = SystemMonitor.shared.getCPUInfo()
    @State private var batteryInfo = SystemMonitor.shared.getBatteryInfo()
    @State private var diskInfo = SystemMonitor.shared.getDiskInfo()
    @State private var timer: Timer?
    @State private var showContent = false

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Text("시스템 모니터")
                    .font(.title2.bold())
                Spacer()
                Button("새로고침") { refresh() }
                    .buttonStyle(.bordered)
            }
            .padding(24)


            ScrollView {
                VStack(spacing: 24) {
                    // CPU + Battery row
                    HStack(spacing: 16) {
                        cpuCard
                        batteryCard
                    }

                    // Disk card
                    diskCard
                }
                .padding(24)
            }
        }
        .onAppear {
            refresh()
            withAnimation(.spring(duration: 0.8, bounce: 0.3).delay(0.1)) {
                showContent = true
            }
            timer = Timer.scheduledTimer(withTimeInterval: 2.0, repeats: true) { _ in
                refresh()
            }
        }
        .onDisappear {
            timer?.invalidate()
            timer = nil
        }
    }

    // MARK: - CPU Card

    private var cpuCard: some View {
        VStack(spacing: 16) {
            Text("CPU 사용률")
                .font(.headline)

            ZStack {
                Circle()
                    .stroke(Color.secondary.opacity(0.2), lineWidth: 14)

                Circle()
                    .trim(from: 0, to: CGFloat(cpuInfo.usage / 100))
                    .stroke(cpuGaugeColor, style: StrokeStyle(lineWidth: 14, lineCap: .round))
                    .rotationEffect(.degrees(-90))
                    .animation(.easeInOut(duration: 0.8), value: cpuInfo.usage)

                VStack(spacing: 2) {
                    Text("\(Int(cpuInfo.usage))%")
                        .font(.system(size: 28, weight: .bold, design: .rounded))
                        .contentTransition(.numericText())
                        .foregroundColor(cpuGaugeColor)
                    Text("CPU")
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                }
            }
            .frame(width: 140, height: 140)

            HStack(spacing: 16) {
                MonitorStatItem(label: "사용자", value: "\(Int(cpuInfo.userUsage))%", color: .blue)
                MonitorStatItem(label: "시스템", value: "\(Int(cpuInfo.systemUsage))%", color: .orange)
            }
        }
        .frame(maxWidth: .infinity)
        .padding(20)
        .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: 16))
    }

    private var cpuGaugeColor: Color {
        if cpuInfo.usage < 50 { return .green }
        if cpuInfo.usage < 80 { return .orange }
        return .red
    }

    // MARK: - Battery Card

    private var batteryCard: some View {
        VStack(spacing: 16) {
            Text("배터리")
                .font(.headline)

            if batteryInfo.isPresent {
                ZStack {
                    Circle()
                        .stroke(Color.secondary.opacity(0.2), lineWidth: 14)

                    Circle()
                        .trim(from: 0, to: CGFloat(batteryInfo.percentage) / 100)
                        .stroke(batteryColor, style: StrokeStyle(lineWidth: 14, lineCap: .round))
                        .rotationEffect(.degrees(-90))
                        .animation(.easeInOut(duration: 0.8), value: batteryInfo.percentage)

                    VStack(spacing: 2) {
                        Image(systemName: batteryIcon)
                            .font(.title3)
                            .foregroundColor(batteryColor)
                        Text("\(batteryInfo.percentage)%")
                            .font(.system(size: 22, weight: .bold, design: .rounded))
                            .foregroundColor(batteryColor)
                        if batteryInfo.isCharging {
                            Text("충전 중")
                                .font(.caption2)
                                .foregroundColor(.green)
                        }
                    }
                }
                .frame(width: 140, height: 140)

                HStack(spacing: 16) {
                    MonitorStatItem(label: "상태", value: batteryInfo.health, color: .green)
                    MonitorStatItem(label: "충전 횟수", value: "\(batteryInfo.cycleCount)", color: .purple)
                    if let remaining = batteryInfo.timeRemaining {
                        MonitorStatItem(label: "남은 시간", value: "\(remaining)분", color: .blue)
                    }
                }
            } else {
                VStack(spacing: 8) {
                    Image(systemName: "bolt.slash")
                        .font(.system(size: 40))
                        .foregroundStyle(.secondary)
                    Text("배터리 없음")
                        .font(.callout)
                        .foregroundStyle(.secondary)
                    Text("데스크탑 Mac")
                        .font(.caption)
                        .foregroundStyle(.tertiary)
                }
                .frame(height: 140)
            }
        }
        .frame(maxWidth: .infinity)
        .padding(20)
        .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: 16))
    }

    private var batteryIcon: String {
        if batteryInfo.isCharging { return "battery.100.bolt" }
        if batteryInfo.percentage > 75 { return "battery.100" }
        if batteryInfo.percentage > 50 { return "battery.75" }
        if batteryInfo.percentage > 25 { return "battery.50" }
        return "battery.25"
    }

    private var batteryColor: Color {
        if batteryInfo.isCharging { return .green }
        if batteryInfo.percentage > 20 { return .green }
        if batteryInfo.percentage > 10 { return .orange }
        return .red
    }

    // MARK: - Disk Card

    private var diskCard: some View {
        VStack(alignment: .leading, spacing: 16) {
            HStack {
                Image(systemName: "internaldrive")
                    .font(.title3)
                    .foregroundColor(.accentColor)
                Text("디스크 저장공간")
                    .font(.headline)
                Spacer()
                Text("\(Int(diskInfo.usagePercent))% 사용")
                    .font(.callout.bold())
                    .foregroundColor(diskGaugeColor)
            }

            // Progress bar
            GeometryReader { geo in
                ZStack(alignment: .leading) {
                    RoundedRectangle(cornerRadius: 8)
                        .fill(Color.secondary.opacity(0.2))

                    RoundedRectangle(cornerRadius: 8)
                        .fill(diskGaugeColor)
                        .frame(width: geo.size.width * CGFloat(diskInfo.usagePercent / 100))
                        .animation(.easeInOut(duration: 0.8), value: diskInfo.usagePercent)
                }
            }
            .frame(height: 20)

            HStack(spacing: 24) {
                MonitorStatItem(label: "전체", value: diskInfo.totalFormatted, color: .secondary)
                MonitorStatItem(label: "사용 중", value: diskInfo.usedFormatted, color: diskGaugeColor)
                MonitorStatItem(label: "여유", value: diskInfo.freeFormatted, color: .green)
            }
        }
        .padding(20)
        .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: 16))
    }

    private var diskGaugeColor: Color {
        if diskInfo.usagePercent < 70 { return .green }
        if diskInfo.usagePercent < 90 { return .orange }
        return .red
    }

    // MARK: - Refresh

    private func refresh() {
        withAnimation(.spring(duration: 0.6, bounce: 0.15)) {
            cpuInfo = SystemMonitor.shared.getCPUInfo()
            batteryInfo = SystemMonitor.shared.getBatteryInfo()
            diskInfo = SystemMonitor.shared.getDiskInfo()
        }
    }
}

// MARK: - Stat Item

struct MonitorStatItem: View {
    let label: String
    let value: String
    let color: Color

    var body: some View {
        VStack(spacing: 4) {
            Text(value)
                .font(.callout.bold())
                .foregroundColor(color)
            Text(label)
                .font(.caption2)
                .foregroundStyle(.secondary)
        }
    }
}
