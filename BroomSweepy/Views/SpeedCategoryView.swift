import SwiftUI

struct SpeedCategoryView: View {
    @Bindable var viewModel: CleanerViewModel
    @State private var selectedTab: Tab = .memory

    enum Tab: String, CaseIterable, Identifiable {
        case memory = "메모리"
        case startup = "시작프로그램"
        case monitor = "시스템 모니터"
        var id: String { rawValue }
    }

    var body: some View {
        VStack(spacing: 0) {
            categoryTabBar

            switch selectedTab {
            case .memory:
                MemoryCleanerView(viewModel: viewModel)
            case .startup:
                StartupManagerView(viewModel: viewModel)
            case .monitor:
                SystemMonitorView()
            }
        }
    }

    private var categoryTabBar: some View {
        HStack(spacing: 0) {
            ForEach(Tab.allCases) { tab in
                Button {
                    withAnimation(.spring(duration: 0.25, bounce: 0.15)) {
                        selectedTab = tab
                    }
                } label: {
                    Text(tab.rawValue)
                        .font(.system(size: 12, weight: selectedTab == tab ? .semibold : .regular))
                        .foregroundStyle(selectedTab == tab ? .primary : .secondary)
                        .padding(.horizontal, 14)
                        .padding(.vertical, 8)
                        .background(
                            selectedTab == tab
                                ? Color.accentColor.opacity(0.12)
                                : Color.clear,
                            in: Capsule()
                        )
                }
                .buttonStyle(.plain)
            }
            Spacer()
        }
        .padding(.horizontal, 20)
        .padding(.vertical, 6)
        .background(.bar)
    }
}
