import SwiftUI
import AppKit

struct PermissionManagerView: View {
    @Bindable var viewModel: CleanerViewModel
    @State private var permissions: [AppPermission] = []
    @State private var isScanning = false
    @State private var selectedType: AppPermission.PermissionType?

    private var groupedPermissions: [(type: AppPermission.PermissionType, items: [AppPermission])] {
        let grouped = Dictionary(grouping: filteredPermissions) { $0.permissionType }
        return AppPermission.PermissionType.allCases
            .compactMap { type in
                guard let items = grouped[type], !items.isEmpty else { return nil }
                return (type: type, items: items.sorted { $0.appName < $1.appName })
            }
    }

    private var filteredPermissions: [AppPermission] {
        if let selected = selectedType {
            return permissions.filter { $0.permissionType == selected }
        }
        return permissions
    }

    private var appPermissionCounts: [(appName: String, count: Int)] {
        let counts = Dictionary(grouping: permissions) { $0.appName }
            .map { (appName: $0.key, count: $0.value.count) }
            .sorted { $0.count > $1.count }
        return counts.filter { $0.count >= 3 }
    }

    var body: some View {
        VStack(spacing: 0) {
            header
            if isScanning {
                scanningState
            } else if permissions.isEmpty {
                emptyState
            } else {
                filterBar
                contentArea
            }
        }
    }

    // MARK: - Header

    private var header: some View {
        HStack(spacing: 12) {
            Text("앱 권한 사용 가능성")
                .font(.title2.bold())
            Spacer()
            Button("스캔") {
                Task { await runScan() }
            }
            .buttonStyle(.borderedProminent)
            .disabled(isScanning)
        }
        .padding(24)
    }

    // MARK: - Filter Bar

    private var filterBar: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 8) {
                PermFilterChip(label: "전체", icon: "square.grid.2x2", isSelected: selectedType == nil) {
                    selectedType = nil
                }
                ForEach(AppPermission.PermissionType.allCases, id: \.self) { type in
                    let count = permissions.filter { $0.permissionType == type }.count
                    if count > 0 {
                        PermFilterChip(
                            label: "\(type.rawValue) (\(count))",
                            icon: type.icon,
                            isSelected: selectedType == type
                        ) {
                            selectedType = selectedType == type ? nil : type
                        }
                    }
                }
            }
            .padding(.horizontal, 24)
            .padding(.vertical, 8)
        }
        .background(.ultraThinMaterial)
    }

    // MARK: - Content

    private var contentArea: some View {
        ScrollView {
            LazyVStack(spacing: 16) {
                // Warning for apps with many permissions
                if !appPermissionCounts.isEmpty && selectedType == nil {
                    warningSection
                }

                ForEach(groupedPermissions, id: \.type) { group in
                    PermissionSection(
                        permissionType: group.type,
                        items: group.items
                    )
                }
            }
            .padding(20)
        }
    }

    // MARK: - Warning Section

    private var warningSection: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 8) {
                Image(systemName: "exclamationmark.triangle.fill")
                    .foregroundColor(.orange)
                Text("다수의 권한을 보유한 앱")
                    .font(.headline)
            }

            ForEach(appPermissionCounts, id: \.appName) { item in
                HStack(spacing: 10) {
                    Image(systemName: "app.fill")
                        .foregroundStyle(.secondary)
                        .frame(width: 20)
                    Text(item.appName)
                        .font(.callout)
                    Spacer()
                    Text("\(item.count)개 권한")
                        .font(.caption)
                        .padding(.horizontal, 8)
                        .padding(.vertical, 3)
                        .background(.orange.opacity(0.15), in: Capsule())
                        .foregroundStyle(.orange)
                }
            }
        }
        .padding(16)
        .background(.orange.opacity(0.05))
        .clipShape(RoundedRectangle(cornerRadius: 12))
        .overlay(
            RoundedRectangle(cornerRadius: 12)
                .stroke(Color.orange.opacity(0.2), lineWidth: 1)
        )
    }

    // MARK: - States

    private var scanningState: some View {
        VStack(spacing: 16) {
            ProgressView()
                .scaleEffect(1.5)
                .progressViewStyle(.circular)
            Text("앱의 권한 사용 근거를 수집 중...")
                .font(.headline)
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .transition(.opacity)
    }

    private var emptyState: some View {
        VStack(spacing: 12) {
            Image(systemName: "lock.shield")
                .font(.system(size: 48))
                .foregroundStyle(.secondary)
            Text("'스캔' 버튼을 눌러 권한 사용 가능성을 확인하세요")
                .font(.headline)
                .foregroundStyle(.secondary)
            Text("실제 허용 상태는 macOS 시스템 설정에서 확인해야 합니다")
                .font(.caption)
                .foregroundStyle(.tertiary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    // MARK: - Actions

    @MainActor
    private func runScan() async {
        isScanning = true
        let homeURL = FileAccessManager.shared.loadBookmark()
        permissions = await Task.detached {
            PermissionManager.shared.scan(homeURL: homeURL, progressCallback: nil)
        }.value
        isScanning = false
        viewModel.toastMessage = "\(permissions.count)개 권한 사용 근거를 찾았습니다"
    }
}

// MARK: - Permission Section

private struct PermissionSection: View {
    let permissionType: AppPermission.PermissionType
    let items: [AppPermission]

    private var typeColor: Color {
        switch permissionType {
        case .camera: return .red
        case .microphone: return .orange
        case .location: return .blue
        case .contacts: return .green
        case .photos: return .purple
        case .fullDiskAccess: return .gray
        case .accessibility: return .teal
        case .screenRecording: return .indigo
        }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            // Section header
            HStack(spacing: 10) {
                Image(systemName: permissionType.icon)
                    .font(.title3)
                    .foregroundStyle(typeColor)
                Text(permissionType.rawValue)
                    .font(.headline)
                Text("\(items.count)개 앱")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                Spacer()
                Button("시스템 설정 열기") {
                    PermissionManager.shared.openSystemSettings(for: permissionType)
                }
                .buttonStyle(.bordered)
                .controlSize(.small)
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 12)
            .background(.ultraThinMaterial)

            Divider()

            ForEach(items) { item in
                PermissionAppRow(permission: item, typeColor: typeColor)
                if item.id != items.last?.id {
                    Divider().padding(.leading, 56)
                }
            }
        }
        .background(.background)
        .clipShape(RoundedRectangle(cornerRadius: 12))
        .overlay(
            RoundedRectangle(cornerRadius: 12)
                .stroke(Color.secondary.opacity(0.15), lineWidth: 1)
        )
    }
}

// MARK: - Permission App Row

private struct PermissionAppRow: View {
    let permission: AppPermission
    let typeColor: Color

    var body: some View {
        HStack(spacing: 14) {
            // App icon
            if let icon = permission.icon {
                Image(nsImage: icon)
                    .resizable()
                    .interpolation(.high)
                    .frame(width: 28, height: 28)
            } else {
                Image(systemName: "app.fill")
                    .font(.title3)
                    .foregroundStyle(.secondary)
                    .frame(width: 28, height: 28)
            }

            // App info
            VStack(alignment: .leading, spacing: 3) {
                Text(permission.appName)
                    .font(.callout.weight(.medium))
                Text(permission.bundleId)
                    .font(.caption2)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                    .truncationMode(.middle)
            }

            Spacer()

            // Status badge
            HStack(spacing: 4) {
                Image(systemName: "info.circle.fill")
                Text(permission.evidence.rawValue)
            }
            .font(.caption)
            .padding(.horizontal, 8)
            .padding(.vertical, 4)
            .background(
                Color.orange.opacity(0.1),
                in: Capsule()
            )
            .foregroundStyle(.orange)
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 10)
    }
}

// MARK: - Filter Chip

private struct PermFilterChip: View {
    let label: String
    let icon: String
    let isSelected: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: 6) {
                Image(systemName: icon)
                    .font(.caption)
                Text(label)
                    .font(.caption)
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 6)
            .background(isSelected ? Color.accentColor.opacity(0.15) : Color.secondary.opacity(0.08),
                        in: Capsule())
            .foregroundStyle(isSelected ? Color.accentColor : .secondary)
        }
        .buttonStyle(.plain)
    }
}
