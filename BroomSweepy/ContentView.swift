import SwiftUI

// MARK: - Root View (온보딩 → 메인)

struct RootView: View {
    @State private var onboardingComplete = UserDefaults.standard.bool(forKey: "onboardingCompleted")

    var body: some View {
        if onboardingComplete {
            ContentView()
        } else {
            OnboardingView(isComplete: $onboardingComplete)
        }
    }
}

// MARK: - Main Category (5 + 대시보드)

enum MainCategory: String, CaseIterable, Identifiable {
    case dashboard
    case space
    case speed
    case security
    case privacy
    case files
    case settings

    var label: String {
        switch self {
        case .dashboard: return String(localized: "dashboard")
        case .space: return String(localized: "space_cleanup")
        case .speed: return String(localized: "speed_optimization")
        case .security: return String(localized: "security")
        case .privacy: return String(localized: "privacy")
        case .files: return String(localized: "file_management")
        case .settings: return String(localized: "settings")
        }
    }

    var id: String { String(describing: self) }

    var icon: String {
        switch self {
        case .dashboard: return "house.fill"
        case .space: return "internaldrive.fill"
        case .speed: return "gauge.with.dots.needle.67percent"
        case .security: return "shield.checkered"
        case .privacy: return "hand.raised.fill"
        case .files: return "folder.fill"
        case .settings: return "gearshape.fill"
        }
    }

    var description: String {
        switch self {
        case .dashboard: return ""
        case .space: return "캐시, 대용량, 중복 파일"
        case .speed: return "메모리, 시작프로그램"
        case .security: return "의심 항목, 앱 권한"
        case .privacy: return "브라우저, 클라우드"
        case .files: return "정리, 규칙, 앱 관리"
        case .settings: return "스캔 설정, 자동 관리"
        }
    }

    var color: Color {
        switch self {
        case .dashboard: return .blue
        case .space: return .red
        case .speed: return .green
        case .security: return .orange
        case .privacy: return .purple
        case .files: return .cyan
        case .settings: return .gray
        }
    }
}

// MARK: - ContentView

struct ContentView: View {
    @State private var selection: MainCategory? = .dashboard
    @State private var viewModel = CleanerViewModel()
    @State private var showSmartClean = false
    @State private var bounceValue = 0

    var body: some View {
        NavigationSplitView {
            sidebar
        } detail: {
            if showSmartClean {
                SmartCleanView(viewModel: viewModel, isPresented: $showSmartClean)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                detailView
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
    }

    // MARK: - Sidebar (5개 카테고리)

    private var sidebar: some View {
        List(selection: $selection) {
            ForEach(MainCategory.allCases) { category in
                if category == .dashboard {
                    SidebarCategoryRow(category: category, isSelected: selection == category)
                        .tag(category)
                } else {
                    SidebarCategoryRow(category: category, isSelected: selection == category)
                        .tag(category)
                }
            }
        }
        .navigationSplitViewColumnWidth(min: 180, ideal: 220)
        .listStyle(.sidebar)
        .safeAreaInset(edge: .top) {
            sidebarHeader
        }
        .safeAreaInset(edge: .bottom) {
            sidebarFooter
        }
        .onChange(of: selection) { _, _ in
            bounceValue += 1
            showSmartClean = false
        }
        .onReceive(NotificationCenter.default.publisher(for: .navigateTo)) { notif in
            guard let target = notif.object as? String else { return }
            switch target {
            case "dashboard": selection = .dashboard
            case "space": selection = .space
            case "speed": selection = .speed
            case "security": selection = .security
            case "privacy": selection = .privacy
            case "files": selection = .files
            case "settings": selection = .settings
            case "smartclean": selection = .dashboard; showSmartClean = true
            case "scan": selection = .dashboard
                // scanAll은 DashboardView에서 처리
            default: break
            }
        }
    }

    private var sidebarHeader: some View {
        HStack(spacing: 8) {
            Image(systemName: "sparkles")
                .font(.title2)
                .foregroundStyle(.linearGradient(
                    colors: [.blue, .purple],
                    startPoint: .topLeading,
                    endPoint: .bottomTrailing
                ))
                .symbolEffect(.bounce, value: bounceValue)
            Text("BroomSweepy")
                .font(.headline)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.horizontal, 16)
        .padding(.vertical, 10)
    }

    private var sidebarFooter: some View {
        VStack(spacing: 6) {
            let disk = SystemMonitor.shared.getDiskInfo()
            HStack(spacing: 6) {
                Image(systemName: "internaldrive.fill")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                Text("\(disk.freeFormatted) 여유")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                Spacer()
                Text("\(Int(disk.usagePercent))%")
                    .font(.caption.bold())
                    .foregroundStyle(disk.usagePercent > 90 ? .red : disk.usagePercent > 70 ? .orange : .green)
            }
            GeometryReader { geo in
                ZStack(alignment: .leading) {
                    Capsule()
                        .fill(.secondary.opacity(0.15))
                    Capsule()
                        .fill(.linearGradient(
                            colors: disk.usagePercent > 90
                                ? [.red, .orange]
                                : disk.usagePercent > 70
                                    ? [.orange, .yellow]
                                    : [.green, .cyan],
                            startPoint: .leading,
                            endPoint: .trailing
                        ))
                        .frame(width: geo.size.width * CGFloat(disk.usagePercent / 100))
                }
            }
            .frame(height: 4)
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 10)
    }

    // MARK: - Detail View

    @ViewBuilder
    private var detailView: some View {
        switch selection {
        case .dashboard:
            DashboardView(viewModel: viewModel, navigate: $selection, showSmartClean: $showSmartClean)
        case .space:
            SpaceCategoryView(viewModel: viewModel)
        case .speed:
            SpeedCategoryView(viewModel: viewModel)
        case .security:
            SecurityCategoryView(viewModel: viewModel)
        case .privacy:
            PrivacyCategoryView(viewModel: viewModel)
        case .files:
            FilesCategoryView(viewModel: viewModel)
        case .settings:
            SettingsView()
        case nil:
            DashboardView(viewModel: viewModel, navigate: $selection, showSmartClean: $showSmartClean)
        }
    }
}

// MARK: - Sidebar Category Row (색상 아이콘 + 라이브 지표)

private struct SidebarCategoryRow: View {
    let category: MainCategory
    let isSelected: Bool
    @State private var bounceCount = 0

    var body: some View {
        HStack(spacing: 12) {
            // 색상 아이콘 원형 배경
            ZStack {
                Circle()
                    .fill(category.color.opacity(isSelected ? 0.2 : 0.1))
                    .frame(width: 32, height: 32)
                Image(systemName: category.icon)
                    .font(.system(size: 14))
                    .foregroundStyle(category.color)
                    .symbolEffect(.bounce, value: bounceCount)
            }

            if category == .dashboard {
                Text(category.label)
                    .font(.system(size: 13, weight: .medium))
            } else {
                VStack(alignment: .leading, spacing: 2) {
                    Text(category.label)
                        .font(.system(size: 13, weight: .medium))
                    Text(category.description)
                        .font(.system(size: 10))
                        .foregroundStyle(.tertiary)
                        .lineLimit(1)
                }
            }

            Spacer(minLength: 0)

            // 라이브 지표
            liveIndicator
        }
        .onChange(of: isSelected) { _, newValue in
            if newValue { bounceCount += 1 }
        }
        .help(category.description.isEmpty ? category.label : category.description)
    }

    @ViewBuilder
    private var liveIndicator: some View {
        switch category {
        case .space:
            let disk = SystemMonitor.shared.getDiskInfo()
            Text("\(Int(disk.usagePercent))%")
                .font(.system(size: 10, weight: .bold, design: .rounded))
                .monospacedDigit()
                .foregroundStyle(disk.usagePercent > 90 ? .red : disk.usagePercent > 70 ? .orange : .green)
        case .speed:
            let mem = MemoryManager.shared.getMemoryInfo()
            Text("\(Int(mem.usagePercent))%")
                .font(.system(size: 10, weight: .bold, design: .rounded))
                .monospacedDigit()
                .foregroundStyle(mem.usagePercent > 80 ? .red : mem.usagePercent > 60 ? .orange : .green)
        case .security:
            Image(systemName: "checkmark")
                .font(.system(size: 9, weight: .bold))
                .foregroundStyle(.green)
        default:
            EmptyView()
        }
    }
}
