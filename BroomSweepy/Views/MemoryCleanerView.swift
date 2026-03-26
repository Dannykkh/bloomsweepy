import SwiftUI

struct MemoryCleanerView: View {
    @Bindable var viewModel: CleanerViewModel
    @State private var memoryInfo = MemoryManager.shared.getMemoryInfo()
    @State private var isPurging = false
    @State private var freedAmount: String?
    @State private var purgeStatus = "정리 중..."
    @State private var timer: Timer?
    @State private var showContent = false
    @State private var celebrateBounce = 0

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Text("메모리 정리")
                    .font(.title2.bold())
                Spacer()
                Button {
                    refreshMemory()
                } label: {
                    Label("새로고침", systemImage: "arrow.clockwise")
                }
                .buttonStyle(.bordered)
            }
            .padding(24)

            ScrollView {
                VStack(spacing: 24) {

                    // Animated Memory Gauge
                    AnimatedMemoryGauge(
                        usagePercent: memoryInfo.usagePercent,
                        usedFormatted: memoryInfo.usedFormatted,
                        totalFormatted: memoryInfo.totalFormatted,
                        show: showContent
                    )

                    // Memory Details
                    HStack(spacing: 16) {
                        MemoryStatCard(title: "사용 중", value: memoryInfo.usedFormatted,
                                       icon: "memorychip.fill", color: .red)
                        MemoryStatCard(title: "여유", value: memoryInfo.freeFormatted,
                                       icon: "memorychip.fill", color: .green)
                        MemoryStatCard(
                            title: "고정 (커널)",
                            value: ByteCountFormatter.string(fromByteCount: Int64(memoryInfo.wired), countStyle: .memory),
                            icon: "lock.fill", color: .purple
                        )
                        MemoryStatCard(
                            title: "압축됨",
                            value: ByteCountFormatter.string(fromByteCount: Int64(memoryInfo.compressed), countStyle: .memory),
                            icon: "archivebox.fill", color: .blue
                        )
                    }
                    .offset(y: showContent ? 0 : 20)
                    .opacity(showContent ? 1 : 0)
                    .animation(.spring(duration: 0.6, bounce: 0.3).delay(0.3), value: showContent)

                    // Purge Button
                    purgeButton

                    // Result
                    if let freed = freedAmount {
                        HStack(spacing: 10) {
                            Image(systemName: "checkmark.circle.fill")
                                .font(.title3)
                                .foregroundStyle(.green)
                                .symbolEffect(.bounce, value: celebrateBounce)
                            Text(freed)
                                .font(.headline)
                                .foregroundStyle(.green)
                        }
                        .padding(16)
                        .background(.green.opacity(0.1), in: RoundedRectangle(cornerRadius: 12))
                        .transition(.asymmetric(
                            insertion: .push(from: .bottom).combined(with: .scale(scale: 0.8)),
                            removal: .opacity
                        ))
                    }
                }
                .padding(24)
            }
        }
        .onAppear {
            refreshMemory()
            withAnimation(.spring(duration: 0.8, bounce: 0.3).delay(0.1)) {
                showContent = true
            }
            timer = Timer.scheduledTimer(withTimeInterval: 3.0, repeats: true) { _ in
                if !isPurging { refreshMemory() }
            }
        }
        .onDisappear {
            timer?.invalidate()
            timer = nil
        }
    }

    private var purgeButton: some View {
        VStack(spacing: 10) {
        Button(action: purgeMemory) {
            HStack(spacing: 10) {
                if isPurging {
                    ProgressView()
                        .scaleEffect(0.8)
                        .padding(.trailing, 2)
                }
                Image(systemName: isPurging ? "arrow.triangle.2.circlepath" : "wand.and.stars")
                    .font(.title3)
                    .symbolEffect(.pulse, isActive: isPurging)
                Text(isPurging ? purgeStatus : "메모리 정리")
                    .font(.headline)
            }
            .frame(maxWidth: .infinity)
            .padding(.vertical, 14)
        }
        .buttonStyle(.borderedProminent)
        .tint(.accentColor)
        .disabled(isPurging)

        if isPurging {
            Button("취소") {
                isPurging = false
                freedAmount = "작업이 취소되었습니다"
            }
            .buttonStyle(.bordered)
        }
        } // VStack
    }

    private func refreshMemory() {
        withAnimation(.spring(duration: 0.6, bounce: 0.2)) {
            memoryInfo = MemoryManager.shared.getMemoryInfo()
        }
    }

    private func purgeMemory() {
        isPurging = true
        freedAmount = nil

        Task.detached {
            let result = MemoryManager.shared.purgeMemory { status in
                Task { @MainActor in
                    purgeStatus = status
                }
            }
            let freed = Int64(result.before.used) - Int64(result.after.used)

            await MainActor.run {
                withAnimation(.spring(duration: 0.5, bounce: 0.3)) {
                    isPurging = false
                    memoryInfo = result.after
                    if freed > 0 {
                        freedAmount = "\(ByteCountFormatter.string(fromByteCount: freed, countStyle: .memory)) 메모리 확보!"
                    } else {
                        freedAmount = "메모리가 이미 최적 상태입니다"
                    }
                    celebrateBounce += 1
                }
            }
        }
    }
}

// MARK: - Animated Memory Gauge (spring-animated ring)

private struct AnimatedMemoryGauge: View {
    let usagePercent: Double
    let usedFormatted: String
    let totalFormatted: String
    let show: Bool

    @State private var animatedProgress: Double = 0

    private var gaugeColor: Color {
        if animatedProgress * 100 < 60 { return .green }
        if animatedProgress * 100 < 80 { return .orange }
        return .red
    }

    var body: some View {
        ZStack {
            // Glow
            Circle()
                .fill(
                    RadialGradient(
                        colors: [gaugeColor.opacity(0.1), .clear],
                        center: .center,
                        startRadius: 40,
                        endRadius: 140
                    )
                )
                .frame(width: 280, height: 280)
                .blur(radius: 15)

            // Background ring
            Circle()
                .stroke(Color.secondary.opacity(0.15), lineWidth: 20)

            // Animated usage ring
            Circle()
                .trim(from: 0, to: animatedProgress)
                .stroke(
                    AngularGradient(
                        colors: [gaugeColor, gaugeColor.opacity(0.7), gaugeColor],
                        center: .center
                    ),
                    style: StrokeStyle(lineWidth: 20, lineCap: .round)
                )
                .rotationEffect(.degrees(-90))

            // Center text
            VStack(spacing: 6) {
                Text("\(Int(animatedProgress * 100))%")
                    .font(.system(size: 44, weight: .bold, design: .rounded))
                    .contentTransition(.numericText())
                    .foregroundStyle(gaugeColor)
                Text("메모리 사용 중")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                Text("\(usedFormatted) / \(totalFormatted)")
                    .font(.caption2)
                    .foregroundStyle(.tertiary)
            }
        }
        .frame(width: 220, height: 220)
        .padding()
        .scaleEffect(show ? 1 : 0.8)
        .opacity(show ? 1 : 0)
        .animation(.spring(duration: 0.8, bounce: 0.3).delay(0.1), value: show)
        .onAppear {
            withAnimation(.spring(duration: 1.5, bounce: 0.25).delay(0.3)) {
                animatedProgress = usagePercent / 100
            }
        }
        .onChange(of: usagePercent) { _, newVal in
            withAnimation(.spring(duration: 0.8, bounce: 0.2)) {
                animatedProgress = newVal / 100
            }
        }
    }
}

// MARK: - Memory Stat Card

struct MemoryStatCard: View {
    let title: String
    let value: String
    let icon: String
    let color: Color

    @State private var isHovering = false

    var body: some View {
        VStack(spacing: 6) {
            Image(systemName: icon)
                .font(.title3)
                .foregroundStyle(color)
            Text(value)
                .font(.callout.bold())
                .contentTransition(.numericText())
            Text(title)
                .font(.caption2)
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity)
        .padding(16)
        .background {
            RoundedRectangle(cornerRadius: 12)
                .fill(.ultraThinMaterial)
                .shadow(color: color.opacity(isHovering ? 0.12 : 0), radius: 8, y: 3)
        }
        .scaleEffect(isHovering ? 1.04 : 1.0)
        .onHover { hovering in
            withAnimation(.spring(response: 0.25, dampingFraction: 0.7)) {
                isHovering = hovering
            }
        }
    }
}
