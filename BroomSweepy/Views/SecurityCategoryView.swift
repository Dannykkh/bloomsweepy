import SwiftUI

struct SecurityCategoryView: View {
    @Bindable var viewModel: CleanerViewModel
    @State private var selectedTab: Tab = .malware

    enum Tab: String, CaseIterable, Identifiable {
        case malware = "의심 항목 검토"
        case permissions = "앱 권한"
        var id: String { rawValue }
    }

    var body: some View {
        VStack(spacing: 0) {
            categoryTabBar

            switch selectedTab {
            case .malware:
                MalwareScannerView(viewModel: viewModel)
            case .permissions:
                PermissionManagerView(viewModel: viewModel)
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
