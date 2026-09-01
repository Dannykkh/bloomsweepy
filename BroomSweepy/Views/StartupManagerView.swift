import SwiftUI

struct StartupManagerView: View {
    @Bindable var viewModel: CleanerViewModel
    @State private var showDisableAllConfirm = false

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Text("시작프로그램 관리")
                    .font(.title2.bold())

                Spacer()

                if !viewModel.loginItems.isEmpty {
                    Button("전체 비활성화") { showDisableAllConfirm = true }
                        .buttonStyle(.bordered)
                        .tint(.orange)
                }

                Button("스캔") { Task { await viewModel.scanLoginItems() } }
                    .buttonStyle(.bordered)
                    .disabled(viewModel.isScanning)
            }
            .padding(24)


            // Summary bar
            if !viewModel.loginItems.isEmpty && !viewModel.isScanning {
                HStack {
                    let enabledCount = viewModel.loginItems.filter(\.isEnabled).count
                    let total = viewModel.loginItems.count

                    Label(
                        "\(enabledCount) / \(total) 활성화됨",
                        systemImage: "power"
                    )
                    .font(.caption)
                    .foregroundStyle(.secondary)

                    Spacer()
                }
                .padding(.horizontal, 24)
                .padding(.bottom, 8)
            }

            // Content
            if viewModel.isScanning {
                scanningState
            } else if viewModel.loginItems.isEmpty {
                emptyState
            } else {
                List(viewModel.loginItems) { item in
                    LoginItemRow(item: item) {
                        viewModel.toggleLoginItem(id: item.id)
                    }
                }
                .listStyle(.inset(alternatesRowBackgrounds: true))
            }
        }
        .alert("전체 비활성화", isPresented: $showDisableAllConfirm) {
            Button("취소", role: .cancel) {}
            Button("비활성화", role: .destructive) {
                viewModel.disableAllLoginItems()
            }
        } message: {
            Text("모든 시작프로그램을 비활성화하시겠습니까? 시스템 필수 에이전트도 포함될 수 있습니다.")
        }
    }

    // MARK: - Sub-states

    private var scanningState: some View {
        VStack(spacing: 16) {
            ProgressView()
                .scaleEffect(1.5)
                .progressViewStyle(.circular)
            Text(viewModel.scanMessage.isEmpty ? "스캔 중..." : viewModel.scanMessage)
                .font(.headline)
                .foregroundStyle(.secondary)
            Button("취소") { viewModel.cancelCurrentTask() }
                .buttonStyle(.bordered)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .transition(.opacity)
    }

    private var emptyState: some View {
        VStack(spacing: 12) {
            Image(systemName: "power")
                .font(.system(size: 40))
                .foregroundStyle(.secondary)
            Text("Mac을 켤 때 자동 실행되는 프로그램을 관리합니다\n불필요한 항목을 끄면 부팅이 빨라집니다")
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
            Text("✅ 비활성화만 — 프로그램이 삭제되지 않음")
                .font(.caption)
                .foregroundStyle(.green)
                .padding(.top, 4)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

// MARK: - Login Item Row

struct LoginItemRow: View {
    let item: LoginItem
    let onToggle: () -> Void

    var body: some View {
        HStack(spacing: 14) {
            // Icon based on type
            Image(systemName: item.type == .launchAgent ? "gearshape" : "app.badge")
                .font(.title2)
                .foregroundStyle(.secondary)
                .frame(width: 32)

            // Name + path
            VStack(alignment: .leading, spacing: 3) {
                Text(item.name)
                    .font(.headline)
                    .lineLimit(1)

                Text(shortenPath(item.path))
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                    .truncationMode(.middle)
            }

            Spacer()

            // Type badge
            Text(item.type.rawValue)
                .font(.system(size: 10, weight: .bold))
                .padding(.horizontal, 6)
                .padding(.vertical, 2)
                .background(typeBadgeColor(item.type).opacity(0.15), in: Capsule())
                .foregroundStyle(typeBadgeColor(item.type))

            // Enable/disable toggle
            Toggle("", isOn: Binding(
                get: { item.isEnabled },
                set: { _ in onToggle() }
            ))
            .toggleStyle(.switch)
            .labelsHidden()
        }
        .padding(.vertical, 4)
        .opacity(item.isEnabled ? 1.0 : 0.55)
        .animation(.easeInOut(duration: 0.15), value: item.isEnabled)
    }

    private func typeBadgeColor(_ type: LoginItem.LoginItemType) -> Color {
        switch type {
        case .launchAgent: return .blue
        case .loginItem:   return .purple
        }
    }

    private func shortenPath(_ path: String) -> String {
        let home = NSHomeDirectory()
        return path.hasPrefix(home) ? "~" + path.dropFirst(home.count) : path
    }
}
