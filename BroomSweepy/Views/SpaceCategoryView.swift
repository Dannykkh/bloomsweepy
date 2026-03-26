import SwiftUI

struct SpaceCategoryView: View {
    @Bindable var viewModel: CleanerViewModel
    @State private var selectedTab: Tab = .cache

    enum Tab: String, CaseIterable, Identifiable {
        case cache = "캐시"
        case largeFiles = "대용량"
        case duplicates = "중복"
        case storageMap = "저장공간 맵"
        case tools = "정리 도구"
        var id: String { rawValue }
    }

    var body: some View {
        VStack(spacing: 0) {
            // Sub-tab bar
            categoryTabBar

            // Content
            switch selectedTab {
            case .cache:
                CacheCleanerView(viewModel: viewModel)
            case .largeFiles:
                LargeFilesView(viewModel: viewModel)
            case .duplicates:
                DuplicateFilesView(viewModel: viewModel)
            case .storageMap:
                StorageTreemapView()
            case .tools:
                CleanupToolsView()
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
