import SwiftUI
import Charts

struct DashboardView: View {
    @Bindable var viewModel: CleanerViewModel
    @Binding var navigate: MainCategory?
    @State private var showContent = false
    @Binding var showSmartClean: Bool

    @State private var healthMonitor = HealthMonitor.shared

    var body: some View {
        ScrollView {
            VStack(spacing: 28) {
                // 데일리 브리핑
                if let briefing = healthMonitor.briefing {
                    dailyBriefingCard(briefing)
                }

                heroStorageRing
                quickActions
                if viewModel.summary.cacheSize > 0 || viewModel.summary.largeFilesSize > 0 {
                    categoryBreakdown
                }
            }
            .padding(32)
        }
        .overlay(alignment: .bottom) {
            if let msg = viewModel.toastMessage {
                toastView(msg)
            }
        }
        .onAppear {
            withAnimation(.spring(duration: 0.8, bounce: 0.3).delay(0.1)) {
                showContent = true
            }
            healthMonitor.generateBriefing()
        }
        // sheet 제거 — ContentView에서 페이지 교체로 처리
    }

    // MARK: - Hero Storage Ring

    private var heroStorageRing: some View {
        VStack(spacing: 24) {
            // Central ring visualization
            StorageHeroRing(
                viewModel: viewModel,
                showContent: showContent
            )

            // Buttons
            HStack(spacing: 16) {
                ScanButton(viewModel: viewModel)

                Button {
                    showSmartClean = true
                } label: {
                    HStack(spacing: 8) {
                        Image(systemName: "sparkles")
                            .font(.body.bold())
                        Text("원클릭 최적화")
                            .font(.body.bold())
                    }
                    .padding(.horizontal, 28)
                    .padding(.vertical, 14)
                    .background(
                        RoundedRectangle(cornerRadius: 14)
                            .fill(.green)
                    )
                    .foregroundStyle(.white)
                }
                .buttonStyle(.plain)
                .help("안전한 항목만 자동 선택하여 한번에 정리합니다")
            }

            // Progress (during scan)
            if viewModel.isScanning {
                VStack(spacing: 8) {
                    ProgressView(value: viewModel.scanProgress)
                        .frame(maxWidth: 360)
                        .tint(.blue)
                    Text(viewModel.scanMessage)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                .transition(.asymmetric(
                    insertion: .push(from: .bottom).combined(with: .opacity),
                    removal: .push(from: .top).combined(with: .opacity)
                ))
            }
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 32)
        .padding(.horizontal, 24)
        .background {
            RoundedRectangle(cornerRadius: 24)
                .fill(.ultraThinMaterial)
                .overlay(
                    RoundedRectangle(cornerRadius: 24)
                        .stroke(.linearGradient(
                            colors: [.white.opacity(0.2), .white.opacity(0.05), .clear],
                            startPoint: .topLeading,
                            endPoint: .bottomTrailing
                        ), lineWidth: 1)
                )
                .shadow(color: .black.opacity(0.1), radius: 20, y: 10)
        }
        .animation(.spring(duration: 0.5, bounce: 0.2), value: viewModel.isScanning)
    }

    // MARK: - Daily Briefing Card

    private func dailyBriefingCard(_ b: HealthMonitor.DailyBriefing) -> some View {
        VStack(spacing: 16) {
            // 인사 + 점수
            HStack(spacing: 16) {
                // 건강 점수 링
                ZStack {
                    Circle()
                        .stroke(b.scoreColor.opacity(0.15), lineWidth: 8)
                    Circle()
                        .trim(from: 0, to: CGFloat(b.healthScore) / 100)
                        .stroke(b.scoreColor, style: StrokeStyle(lineWidth: 8, lineCap: .round))
                        .rotationEffect(.degrees(-90))
                    VStack(spacing: 0) {
                        Text("\(b.healthScore)")
                            .font(.system(size: 22, weight: .bold, design: .rounded))
                            .contentTransition(.numericText())
                            .foregroundStyle(b.scoreColor)
                        Text(b.scoreLabel)
                            .font(.system(size: 9))
                            .foregroundStyle(.secondary)
                    }
                }
                .frame(width: 64, height: 64)

                VStack(alignment: .leading, spacing: 4) {
                    Text("\(b.greeting) 👋")
                        .font(.headline)
                    HStack(spacing: 12) {
                        Label("디스크 \(Int(b.diskUsagePercent))%", systemImage: "internaldrive.fill")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                        Label("메모리 \(Int(b.memoryUsagePercent))%", systemImage: "memorychip.fill")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                        Label("여유 \(b.diskFree)", systemImage: "arrow.up.circle")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                }

                Spacer()
            }

            // 추천사항
            if !b.recommendations.isEmpty {
                VStack(spacing: 6) {
                    ForEach(Array(b.recommendations.enumerated()), id: \.offset) { _, rec in
                        HStack(spacing: 10) {
                            Image(systemName: rec.icon)
                                .font(.caption)
                                .foregroundStyle(rec.color)
                                .frame(width: 18)
                            Text(rec.text)
                                .font(.system(size: 12))
                                .foregroundStyle(.secondary)
                            Spacer()
                        }
                    }
                }
            }
        }
        .padding(18)
        .background {
            RoundedRectangle(cornerRadius: 16)
                .fill(.ultraThinMaterial)
                .overlay(
                    RoundedRectangle(cornerRadius: 16)
                        .stroke(b.scoreColor.opacity(0.15), lineWidth: 1)
                )
        }
        .opacity(showContent ? 1 : 0)
        .offset(y: showContent ? 0 : 15)
        .animation(.spring(duration: 0.6, bounce: 0.2).delay(0.05), value: showContent)
    }

    // MARK: - Quick Actions

    private var quickActions: some View {
        HStack(spacing: 12) {
            QuickActionButton(icon: "internaldrive.fill", title: "캐시 정리", color: .red,
                              delay: 0.1, show: showContent) {
                withAnimation(.spring(duration: 0.4, bounce: 0.2)) { navigate = .space }
            }
            QuickActionButton(icon: "memorychip.fill", title: "메모리", color: .green,
                              delay: 0.15, show: showContent) {
                withAnimation(.spring(duration: 0.4, bounce: 0.2)) { navigate = .speed }
            }
            QuickActionButton(icon: "shield.checkered", title: "보안 검사", color: .orange,
                              delay: 0.2, show: showContent) {
                withAnimation(.spring(duration: 0.4, bounce: 0.2)) { navigate = .security }
            }
            QuickActionButton(icon: "wrench.and.screwdriver.fill", title: "유지보수", color: .purple,
                              delay: 0.25, show: showContent) {
                withAnimation(.spring(duration: 0.4, bounce: 0.2)) { navigate = .speed }
            }
        }
    }

    // MARK: - Category Breakdown (animated cards)

    private var categoryBreakdown: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("저장공간 분석")
                .font(.title3.bold())
                .padding(.leading, 4)

            HStack(spacing: 16) {
                AnimatedCategoryCard(
                    icon: "internaldrive.fill", title: "캐시/임시파일",
                    size: viewModel.summary.cacheSize,
                    value: viewModel.summary.cacheSizeFormatted,
                    count: "\(viewModel.summary.cacheCount)개",
                    color: .red, delay: 0.3, show: showContent
                ) {
                    withAnimation(.spring(duration: 0.4, bounce: 0.2)) { navigate = .space }
                }
                AnimatedCategoryCard(
                    icon: "doc.richtext.fill", title: "대용량 파일",
                    size: viewModel.summary.largeFilesSize,
                    value: viewModel.summary.largeFilesSizeFormatted,
                    count: "\(viewModel.summary.largeFilesCount)개",
                    color: .blue, delay: 0.4, show: showContent
                ) {
                    withAnimation(.spring(duration: 0.4, bounce: 0.2)) { navigate = .space }
                }
                AnimatedCategoryCard(
                    icon: "doc.on.doc.fill", title: "중복 파일",
                    size: viewModel.summary.duplicateWaste,
                    value: viewModel.summary.duplicateWasteFormatted,
                    count: "\(viewModel.summary.duplicateGroups)개 그룹",
                    color: .orange, delay: 0.5, show: showContent
                ) {
                    withAnimation(.spring(duration: 0.4, bounce: 0.2)) { navigate = .space }
                }
            }
        }
        .transition(.move(edge: .bottom).combined(with: .opacity))
    }

    // MARK: - Toast

    private func toastView(_ message: String) -> some View {
        HStack(spacing: 8) {
            Image(systemName: "checkmark.circle.fill")
                .symbolEffect(.bounce, value: message)
            Text(message)
                .font(.callout.bold())
        }
        .padding(.horizontal, 24)
        .padding(.vertical, 12)
        .background(.green, in: Capsule())
        .foregroundStyle(.white)
        .shadow(color: .green.opacity(0.3), radius: 12, y: 4)
        .padding(.bottom, 24)
        .transition(.asymmetric(
            insertion: .push(from: .bottom).combined(with: .scale(scale: 0.8)),
            removal: .push(from: .top).combined(with: .opacity)
        ))
        .onAppear {
            DispatchQueue.main.asyncAfter(deadline: .now() + 3) {
                withAnimation(.spring(duration: 0.4)) { viewModel.toastMessage = nil }
            }
        }
    }
}

// MARK: - Storage Hero Ring (Central visualization)

private struct StorageHeroRing: View {
    @Bindable var viewModel: CleanerViewModel
    let showContent: Bool

    @State private var ringProgress: Double = 0
    @State private var cleanableProgress: Double = 0
    @State private var animatedPercent: Int = 0

    private var disk: (used: Double, cleanable: Double) {
        let diskInfo = SystemMonitor.shared.getDiskInfo()
        let usedRatio = diskInfo.usagePercent / 100
        let totalCleanable = viewModel.summary.totalCleanable
        let totalDisk = max(diskInfo.totalSpace, 1)
        let cleanableRatio = Double(totalCleanable) / Double(totalDisk)
        return (usedRatio, cleanableRatio)
    }

    var body: some View {
        ZStack {
            // Glow behind ring
            Circle()
                .fill(
                    RadialGradient(
                        colors: [.blue.opacity(0.12), .clear],
                        center: .center,
                        startRadius: 60,
                        endRadius: 160
                    )
                )
                .frame(width: 320, height: 320)
                .blur(radius: 20)
                .opacity(showContent ? 1 : 0)

            // Background ring
            Circle()
                .stroke(Color.secondary.opacity(0.12), lineWidth: 22)
                .frame(width: 240, height: 240)

            // Used storage ring (blue-purple gradient)
            Circle()
                .trim(from: 0, to: ringProgress)
                .stroke(
                    AngularGradient(
                        colors: [.blue, .indigo, .purple, .blue],
                        center: .center,
                        startAngle: .degrees(-90),
                        endAngle: .degrees(270)
                    ),
                    style: StrokeStyle(lineWidth: 22, lineCap: .round)
                )
                .frame(width: 240, height: 240)
                .rotationEffect(.degrees(-90))

            // Cleanable overlay (pulsing orange/red)
            if cleanableProgress > 0 {
                Circle()
                    .trim(from: max(0, ringProgress - cleanableProgress), to: ringProgress)
                    .stroke(
                        Color.orange.opacity(0.85),
                        style: StrokeStyle(lineWidth: 22, lineCap: .round)
                    )
                    .frame(width: 240, height: 240)
                    .rotationEffect(.degrees(-90))
            }

            // Center content
            VStack(spacing: 6) {
                if viewModel.isScanning {
                    Image(systemName: "magnifyingglass")
                        .font(.system(size: 28))
                        .symbolEffect(.bounce, value: viewModel.scanProgress)
                        .foregroundStyle(.blue)

                    Text("스캔 중...")
                        .font(.callout)
                        .foregroundStyle(.secondary)
                } else if viewModel.summary.totalCleanable > 0 {
                    Text(viewModel.summary.totalCleanableFormatted)
                        .font(.system(size: 36, weight: .bold, design: .rounded))
                        .contentTransition(.numericText())
                        .foregroundStyle(.orange)

                    Text("정리 가능")
                        .font(.callout)
                        .foregroundStyle(.secondary)
                } else {
                    let mem = MemoryManager.shared.getMemoryInfo()

                    Image(systemName: "sparkles")
                        .font(.system(size: 28))
                        .foregroundStyle(.linearGradient(
                            colors: [.blue, .purple],
                            startPoint: .topLeading,
                            endPoint: .bottomTrailing
                        ))
                        .symbolEffect(.bounce, value: showContent)

                    Text("Mac을 깨끗하게")
                        .font(.headline)
                    Text("메모리 \(Int(mem.usagePercent))%")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }
            .animation(.spring(duration: 0.5), value: viewModel.isScanning)
        }
        .onAppear { animateRing() }
        .onChange(of: viewModel.summary.totalCleanable) { _, _ in animateRing() }
    }

    private func animateRing() {
        let d = disk
        withAnimation(.spring(duration: 1.5, bounce: 0.2).delay(0.2)) {
            ringProgress = d.used
        }
        withAnimation(.spring(duration: 1.2, bounce: 0.3).delay(0.9)) {
            cleanableProgress = d.cleanable
        }
    }
}

// MARK: - Scan Button (breathing glow CTA)

private struct ScanButton: View {
    @Bindable var viewModel: CleanerViewModel
    @State private var glowOpacity: Double = 0
    @State private var isPressed = false
    @State private var bounceVal = 0

    var body: some View {
        Button {
            bounceVal += 1
            Task { await viewModel.scanAll() }
        } label: {
            HStack(spacing: 10) {
                Image(systemName: viewModel.isScanning ? "progress.indicator" : "magnifyingglass")
                    .font(.body.bold())
                    .symbolEffect(.bounce, value: bounceVal)
                Text(viewModel.isScanning ? "스캔 중..." : "전체 스캔 시작")
                    .font(.body.bold())
            }
            .padding(.horizontal, 36)
            .padding(.vertical, 14)
            .background(
                RoundedRectangle(cornerRadius: 14)
                    .fill(Color.accentColor)
                    .shadow(color: .accentColor.opacity(glowOpacity), radius: 16, y: 6)
            )
            .foregroundStyle(.white)
            .scaleEffect(isPressed ? 0.95 : 1.0)
        }
        .buttonStyle(.plain)
        .disabled(viewModel.isScanning)
        .onLongPressGesture(minimumDuration: .infinity, pressing: { pressing in
            withAnimation(.spring(response: 0.3, dampingFraction: 0.6)) {
                isPressed = pressing
            }
        }, perform: {})
        .onHover { hovering in
            withAnimation(.easeInOut(duration: 0.4)) {
                glowOpacity = hovering ? 0.5 : 0.15
            }
        }
        .onAppear {
            glowOpacity = 0.15
        }
    }
}

// MARK: - Quick Action Button (staggered entrance)

private struct QuickActionButton: View {
    let icon: String
    let title: String
    let color: Color
    let delay: Double
    let show: Bool
    let action: () -> Void

    @State private var isHovering = false
    @State private var tapBounce = 0

    var body: some View {
        Button {
            tapBounce += 1
            action()
        } label: {
            VStack(spacing: 10) {
                ZStack {
                    Circle()
                        .fill(color.opacity(isHovering ? 0.2 : 0.1))
                        .frame(width: 48, height: 48)
                    Image(systemName: icon)
                        .font(.title3)
                        .foregroundColor(color)
                        .symbolEffect(.bounce, value: tapBounce)
                }
                Text(title)
                    .font(.caption.bold())
                    .foregroundStyle(.primary)
            }
            .frame(maxWidth: .infinity)
            .padding(.vertical, 18)
            .background {
                RoundedRectangle(cornerRadius: 16)
                    .fill(.ultraThinMaterial)
                    .overlay(
                        RoundedRectangle(cornerRadius: 16)
                            .stroke(color.opacity(isHovering ? 0.3 : 0.08), lineWidth: 1)
                    )
                    .shadow(color: color.opacity(isHovering ? 0.15 : 0), radius: 12, y: 4)
            }
            .scaleEffect(isHovering ? 1.04 : 1.0)
        }
        .buttonStyle(.plain)
        .onHover { hovering in
            withAnimation(.spring(response: 0.25, dampingFraction: 0.7)) {
                isHovering = hovering
            }
        }
        .offset(y: show ? 0 : 30)
        .opacity(show ? 1 : 0)
        .animation(.spring(duration: 0.6, bounce: 0.3).delay(delay), value: show)
    }
}

// MARK: - Animated Category Card

private struct AnimatedCategoryCard: View {
    let icon: String
    let title: String
    let size: Int64
    let value: String
    let count: String
    let color: Color
    let delay: Double
    let show: Bool
    let action: () -> Void

    @State private var isHovering = false
    @State private var barProgress: CGFloat = 0

    var body: some View {
        Button(action: action) {
            VStack(alignment: .leading, spacing: 10) {
                HStack {
                    Image(systemName: icon)
                        .font(.title3)
                        .foregroundStyle(color)
                    Spacer()
                    Text(count)
                        .font(.system(size: 10))
                        .foregroundStyle(.tertiary)
                }

                Text(value)
                    .font(.title2.bold())
                    .contentTransition(.numericText())
                    .foregroundStyle(color)

                Text(title)
                    .font(.caption)
                    .foregroundStyle(.secondary)

                // Mini progress bar
                GeometryReader { geo in
                    ZStack(alignment: .leading) {
                        Capsule().fill(color.opacity(0.12))
                        Capsule().fill(color.gradient)
                            .frame(width: geo.size.width * barProgress)
                    }
                }
                .frame(height: 4)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(18)
            .background {
                RoundedRectangle(cornerRadius: 16)
                    .fill(.ultraThinMaterial)
                    .overlay(
                        RoundedRectangle(cornerRadius: 16)
                            .stroke(color.opacity(isHovering ? 0.35 : 0.1), lineWidth: 1)
                    )
                    .shadow(color: color.opacity(isHovering ? 0.15 : 0.05), radius: isHovering ? 16 : 6, y: isHovering ? 6 : 2)
            }
            .scaleEffect(isHovering ? 1.03 : 1.0)
        }
        .buttonStyle(.plain)
        .onHover { hovering in
            withAnimation(.spring(response: 0.25, dampingFraction: 0.7)) {
                isHovering = hovering
            }
        }
        .offset(y: show ? 0 : 30)
        .opacity(show ? 1 : 0)
        .animation(.spring(duration: 0.6, bounce: 0.3).delay(delay), value: show)
        .onAppear {
            withAnimation(.spring(duration: 1.0, bounce: 0.2).delay(delay + 0.3)) {
                barProgress = min(1.0, CGFloat(size) / max(1, CGFloat(size) + 1_000_000_000))
            }
        }
        .onChange(of: size) { _, newVal in
            withAnimation(.spring(duration: 0.8, bounce: 0.2)) {
                barProgress = min(1.0, CGFloat(newVal) / max(1, CGFloat(newVal) + 1_000_000_000))
            }
        }
    }
}

// MARK: - Summary Card (kept for backward compat)

struct SummaryCard: View {
    let icon: String
    let title: String
    let value: String
    var count: String = ""
    let color: Color
    var action: (() -> Void)? = nil

    var body: some View {
        Button { action?() } label: {
            VStack(alignment: .leading, spacing: 8) {
                Image(systemName: icon)
                    .font(.title2)
                    .foregroundStyle(color)
                Text(value)
                    .font(.title2.bold())
                    .foregroundStyle(color)
                Text(title)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                if !count.isEmpty {
                    Text(count)
                        .font(.system(size: 10))
                        .foregroundStyle(.tertiary)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(20)
            .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: 16))
        }
        .buttonStyle(.plain)
    }
}
