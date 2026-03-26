import SwiftUI
import AppKit

struct AppUninstallerView: View {
    @Bindable var viewModel: CleanerViewModel
    @State private var showConfirm = false
    @State private var expandedAppIDs: Set<UUID> = []

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack(alignment: .center, spacing: 12) {
                Text("앱 완전 삭제")
                    .font(.title2.bold())

                Spacer()

                Picker("필터", selection: $viewModel.appFilter) {
                    ForEach(CleanerViewModel.AppFilter.allCases) { filter in
                        Text(filter.rawValue).tag(filter)
                    }
                }
                .pickerStyle(.segmented)
                .frame(maxWidth: 300)

                Button("스캔") { Task { await viewModel.scanApps() } }
                    .buttonStyle(.bordered)
                    .disabled(viewModel.isScanning)

                Button("선택 앱 삭제") { showConfirm = true }
                    .buttonStyle(.borderedProminent)
                    .tint(.red)
                    .disabled(viewModel.selectedAppIDs.isEmpty)
            }
            .padding(24)


            // Select-all row
            if !viewModel.filteredApps.isEmpty && !viewModel.isScanning {
                HStack {
                    Toggle("전체 선택", isOn: Binding(
                        get: {
                            let ids = Set(viewModel.filteredApps.map(\.id))
                            return !ids.isEmpty && ids.isSubset(of: viewModel.selectedAppIDs)
                        },
                        set: { selectAll in
                            if selectAll {
                                viewModel.selectedAppIDs.formUnion(viewModel.filteredApps.map(\.id))
                            } else {
                                viewModel.selectedAppIDs.subtract(viewModel.filteredApps.map(\.id))
                            }
                        }
                    ))
                    .toggleStyle(.checkbox)

                    Spacer()

                    let totalSelected = viewModel.filteredApps
                        .filter { viewModel.selectedAppIDs.contains($0.id) }
                        .reduce(Int64(0)) { $0 + $1.totalSize }

                    if totalSelected > 0 {
                        Text("선택: \(ByteCountFormatter.string(fromByteCount: totalSelected, countStyle: .file))")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                }
                .padding(.horizontal, 24)
                .padding(.bottom, 8)
            }

            // Content
            if viewModel.isScanning {
                scanningState
            } else if viewModel.installedApps.isEmpty {
                emptyState
            } else if viewModel.filteredApps.isEmpty {
                VStack(spacing: 12) {
                    Image(systemName: "checkmark.seal")
                        .font(.system(size: 40))
                        .foregroundStyle(.secondary)
                    Text("이 필터에 해당하는 앱이 없습니다")
                        .foregroundStyle(.secondary)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                List(viewModel.filteredApps) { app in
                    AppRow(
                        app: app,
                        isSelected: viewModel.selectedAppIDs.contains(app.id),
                        isExpanded: expandedAppIDs.contains(app.id),
                        onToggleSelect: {
                            if viewModel.selectedAppIDs.contains(app.id) {
                                viewModel.selectedAppIDs.remove(app.id)
                            } else {
                                viewModel.selectedAppIDs.insert(app.id)
                            }
                        },
                        onToggleExpand: {
                            if expandedAppIDs.contains(app.id) {
                                expandedAppIDs.remove(app.id)
                            } else {
                                expandedAppIDs.insert(app.id)
                            }
                        }
                    )
                }
                .listStyle(.inset(alternatesRowBackgrounds: true))
            }
        }
        .alert("앱 삭제 확인", isPresented: $showConfirm) {
            Button("취소", role: .cancel) {}
            Button("휴지통으로 이동", role: .destructive) {
                Task { await viewModel.uninstallSelectedApps() }
            }
        } message: {
            let count = viewModel.selectedAppIDs.count
            let size = viewModel.filteredApps
                .filter { viewModel.selectedAppIDs.contains($0.id) }
                .reduce(Int64(0)) { $0 + $1.totalSize }
            Text("\(count)개 앱과 관련 파일을 휴지통으로 이동합니다. (\(ByteCountFormatter.string(fromByteCount: size, countStyle: .file)))")
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
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .transition(.opacity)
    }

    private var emptyState: some View {
        VStack(spacing: 12) {
            Image(systemName: "square.stack.3d.up")
                .font(.system(size: 40))
                .foregroundStyle(.secondary)
            Text("앱 삭제 시 남는 설정파일, 캐시, 찌꺼기까지\n함께 정리합니다 (일반 삭제 시 수백 MB 잔존)")
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
            Text("✅ 휴지통으로 이동 — Finder에서 복구 가능")
                .font(.caption)
                .foregroundStyle(.green)
                .padding(.top, 4)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

// MARK: - App Row

struct AppRow: View {
    let app: InstalledApp
    let isSelected: Bool
    let isExpanded: Bool
    let onToggleSelect: () -> Void
    let onToggleExpand: () -> Void

    private var appSizeFormatted: String {
        ByteCountFormatter.string(fromByteCount: app.size, countStyle: .file)
    }
    private var relatedSizeFormatted: String {
        ByteCountFormatter.string(fromByteCount: app.relatedFilesSize, countStyle: .file)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            // Main row
            HStack(spacing: 12) {
                Toggle("", isOn: Binding(get: { isSelected }, set: { _ in onToggleSelect() }))
                    .toggleStyle(.checkbox)
                    .labelsHidden()

                // App icon
                Image(nsImage: app.icon ?? NSImage())
                    .resizable()
                    .interpolation(.high)
                    .frame(width: 32, height: 32)

                // Name + metadata
                VStack(alignment: .leading, spacing: 3) {
                    HStack(spacing: 6) {
                        Text(app.name)
                            .font(.headline)
                        if app.isUnused {
                            Text("미사용")
                                .font(.system(size: 10, weight: .bold))
                                .padding(.horizontal, 6)
                                .padding(.vertical, 2)
                                .background(.orange.opacity(0.15), in: Capsule())
                                .foregroundStyle(.orange)
                        }
                    }
                    Text(app.bundleIdentifier)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                }

                Spacer()

                // Size columns
                VStack(alignment: .trailing, spacing: 3) {
                    Text(app.sizeFormatted)
                        .font(.headline)
                        .foregroundStyle(.red)
                    HStack(spacing: 4) {
                        Text("앱 \(appSizeFormatted)")
                            .font(.caption2)
                            .foregroundStyle(.secondary)
                        if app.relatedFilesSize > 0 {
                            Text("+관련 \(relatedSizeFormatted)")
                                .font(.caption2)
                                .foregroundStyle(.secondary)
                        }
                    }
                }
                .frame(minWidth: 120, alignment: .trailing)

                // Expand button (only when there are related files)
                if !app.relatedFiles.isEmpty {
                    Button {
                        onToggleExpand()
                    } label: {
                        Image(systemName: isExpanded ? "chevron.up" : "chevron.down")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                    .buttonStyle(.plain)
                    .frame(width: 20)
                }
            }
            .padding(.vertical, 6)
            .contentShape(Rectangle())
            .onTapGesture { onToggleSelect() }

            // Expandable related files list
            if isExpanded && !app.relatedFiles.isEmpty {
                VStack(alignment: .leading, spacing: 4) {
                    Divider()
                        .padding(.leading, 80)

                    Text("관련 파일")
                        .font(.caption.bold())
                        .foregroundStyle(.secondary)
                        .padding(.leading, 80)
                        .padding(.top, 6)

                    ForEach(app.relatedFiles, id: \.self) { path in
                        HStack(spacing: 8) {
                            Image(systemName: "doc")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                                .frame(width: 14)
                            Text(shortenPath(path))
                                .font(.caption)
                                .foregroundStyle(.secondary)
                                .lineLimit(1)
                                .truncationMode(.middle)
                        }
                        .padding(.leading, 80)
                    }
                }
                .padding(.bottom, 8)
            }
        }
    }

    private func shortenPath(_ path: String) -> String {
        let home = NSHomeDirectory()
        return path.hasPrefix(home) ? "~" + path.dropFirst(home.count) : path
    }
}
