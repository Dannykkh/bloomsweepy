import SwiftUI

struct CloudStorageView: View {
    @Bindable var viewModel: CleanerViewModel
    @State private var providers: [CloudProvider] = []
    @State private var isScanning = false
    @State private var expandedProviderIDs: Set<UUID> = []
    @State private var selectedFileIDs: Set<UUID> = []
    @State private var showDeleteConfirm = false
    @State private var resultMessage: String?

    private var totalCloudSize: Int64 {
        providers.reduce(0) { $0 + $1.totalSize }
    }

    private var totalFileCount: Int {
        providers.reduce(0) { $0 + $1.fileCount }
    }

    private var selectedFiles: [CloudFile] {
        providers.flatMap(\.files).filter { selectedFileIDs.contains($0.id) }
    }

    private var selectedSize: Int64 {
        selectedFiles.reduce(0) { $0 + $1.size }
    }

    var body: some View {
        VStack(spacing: 0) {
            header

            if isScanning {
                scanningState
            } else if providers.isEmpty {
                emptyState
            } else {
                summaryBar
                providerList
            }
        }
        .alert("로컬에서만 삭제", isPresented: $showDeleteConfirm) {
            Button("취소", role: .cancel) {}
            Button("삭제", role: .destructive) { performDelete() }
        } message: {
            Text("선택한 \(selectedFileIDs.count)개 파일을 로컬에서 삭제합니다.\n클라우드의 원본은 유지됩니다.\n(\(ByteCountFormatter.string(fromByteCount: selectedSize, countStyle: .file)))")
        }
    }

    // MARK: - Header

    private var header: some View {
        HStack(spacing: 12) {
            Text("클라우드 스토리지 정리")
                .font(.title2.bold())
            Spacer()
            if !selectedFileIDs.isEmpty {
                Button("로컬에서만 삭제 (\(selectedFileIDs.count)개)") {
                    showDeleteConfirm = true
                }
                .buttonStyle(.borderedProminent)
                .tint(.red)
            }
            Button("스캔") {
                Task { await runScan() }
            }
            .buttonStyle(.bordered)
            .disabled(isScanning)
        }
        .padding(24)
    }

    // MARK: - Summary Bar

    private var summaryBar: some View {
        HStack(spacing: 24) {
            CloudSummaryChip(
                label: "전체 클라우드 사용량",
                value: ByteCountFormatter.string(fromByteCount: totalCloudSize, countStyle: .file),
                icon: "cloud.fill",
                color: .blue
            )
            CloudSummaryChip(
                label: "파일 수",
                value: "\(totalFileCount)개",
                icon: "doc.fill",
                color: .purple
            )
            CloudSummaryChip(
                label: "선택됨",
                value: ByteCountFormatter.string(fromByteCount: selectedSize, countStyle: .file),
                icon: "checkmark.circle.fill",
                color: .orange
            )
            Spacer()
            if let msg = resultMessage {
                HStack(spacing: 6) {
                    Image(systemName: "checkmark.circle.fill")
                        .foregroundStyle(.green)
                    Text(msg)
                        .font(.callout)
                        .foregroundStyle(.green)
                }
            }
        }
        .padding(.horizontal, 24)
        .padding(.vertical, 12)
        .background(.ultraThinMaterial)
    }

    // MARK: - Provider List

    private var providerList: some View {
        ScrollView {
            LazyVStack(spacing: 16) {
                ForEach(providers) { provider in
                    CloudProviderCard(
                        provider: provider,
                        isExpanded: expandedProviderIDs.contains(provider.id),
                        selectedFileIDs: $selectedFileIDs,
                        onToggleExpand: {
                            if expandedProviderIDs.contains(provider.id) {
                                expandedProviderIDs.remove(provider.id)
                            } else {
                                expandedProviderIDs.insert(provider.id)
                            }
                        }
                    )
                }
            }
            .padding(20)
        }
    }

    // MARK: - States

    private var scanningState: some View {
        VStack(spacing: 16) {
            ProgressView()
                .scaleEffect(1.5)
                .progressViewStyle(.circular)
            Text("클라우드 스토리지 스캔 중...")
                .font(.headline)
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .transition(.opacity)
    }

    private var emptyState: some View {
        VStack(spacing: 12) {
            Image(systemName: "cloud")
                .font(.system(size: 48))
                .foregroundStyle(.secondary)
            Text("'스캔' 버튼을 눌러 클라우드 스토리지를 분석하세요")
                .font(.headline)
                .foregroundStyle(.secondary)
            Text("iCloud, Google Drive, OneDrive, Dropbox를 지원합니다")
                .font(.caption)
                .foregroundStyle(.tertiary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    // MARK: - Actions

    @MainActor
    private func runScan() async {
        isScanning = true
        selectedFileIDs.removeAll()
        resultMessage = nil

        let homeURL = FileAccessManager.shared.loadBookmark()
        providers = await Task.detached {
            CloudStorageCleaner.shared.scan(homeURL: homeURL, progressCallback: nil)
        }.value

        isScanning = false
        viewModel.toastMessage = providers.isEmpty
            ? "클라우드 스토리지를 찾지 못했습니다"
            : "\(providers.count)개 클라우드 서비스 발견"
    }

    private func performDelete() {
        let paths = selectedFiles.map(\.path)
        Task {
            let result = await Task.detached {
                CloudStorageCleaner.shared.deleteLocalOnly(paths: paths)
            }.value
            selectedFileIDs.removeAll()

            if result.errors.isEmpty {
                resultMessage = "\(ByteCountFormatter.string(fromByteCount: result.freed, countStyle: .file)) 정리 완료"
            } else {
                resultMessage = "일부 항목 실패 (\(result.errors.count)건)"
            }
            viewModel.toastMessage = resultMessage
            await runScan()
        }
    }
}

// MARK: - Cloud Provider Card

private struct CloudProviderCard: View {
    let provider: CloudProvider
    let isExpanded: Bool
    @Binding var selectedFileIDs: Set<UUID>
    let onToggleExpand: () -> Void

    private var providerColor: Color {
        switch provider.name {
        case "iCloud Drive": return .blue
        case "Google Drive": return .green
        case "OneDrive": return .cyan
        case "Dropbox": return .blue
        default: return .gray
        }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            // Provider header — always visible
            HStack(spacing: 14) {
                Image(systemName: provider.icon)
                    .font(.title2)
                    .foregroundColor(providerColor)
                    .frame(width: 36)

                VStack(alignment: .leading, spacing: 3) {
                    Text(provider.name)
                        .font(.headline)
                    Text("\(provider.fileCount)개 파일")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }

                Spacer()

                Text(provider.totalSizeFormatted)
                    .font(.title3.bold())
                    .foregroundStyle(providerColor)

                Button {
                    onToggleExpand()
                } label: {
                    Image(systemName: isExpanded ? "chevron.up" : "chevron.down")
                        .font(.callout)
                        .foregroundStyle(.secondary)
                }
                .buttonStyle(.plain)
                .frame(width: 28)
            }
            .padding(16)
            .contentShape(Rectangle())
            .onTapGesture { onToggleExpand() }

            // Expanded file list
            if isExpanded && !provider.files.isEmpty {
                Divider()

                // Select all for this provider
                HStack {
                    let allSelected = provider.files.allSatisfy { selectedFileIDs.contains($0.id) }
                    Toggle("전체 선택", isOn: Binding(
                        get: { allSelected },
                        set: { select in
                            if select {
                                provider.files.forEach { selectedFileIDs.insert($0.id) }
                            } else {
                                provider.files.forEach { selectedFileIDs.remove($0.id) }
                            }
                        }
                    ))
                    .toggleStyle(.checkbox)
                    .font(.caption)

                    Spacer()

                    let oldCount = provider.files.filter(\.isOld).count
                    if oldCount > 0 {
                        Button("오래된 파일만 선택 (\(oldCount)개)") {
                            provider.files.filter(\.isOld).forEach { selectedFileIDs.insert($0.id) }
                        }
                        .buttonStyle(.bordered)
                        .controlSize(.small)
                    }
                }
                .padding(.horizontal, 16)
                .padding(.vertical, 8)
                .background(.ultraThinMaterial)

                ForEach(provider.files) { file in
                    CloudFileRow(
                        file: file,
                        isSelected: selectedFileIDs.contains(file.id),
                        providerColor: providerColor
                    ) {
                        if selectedFileIDs.contains(file.id) {
                            selectedFileIDs.remove(file.id)
                        } else {
                            selectedFileIDs.insert(file.id)
                        }
                    }
                    if file.id != provider.files.last?.id {
                        Divider().padding(.leading, 56)
                    }
                }
            }
        }
        .background(.background)
        .clipShape(RoundedRectangle(cornerRadius: 12))
        .overlay(
            RoundedRectangle(cornerRadius: 12)
                .stroke(providerColor.opacity(0.2), lineWidth: 1)
        )
    }
}

// MARK: - Cloud File Row

private struct CloudFileRow: View {
    let file: CloudFile
    let isSelected: Bool
    let providerColor: Color
    let onToggle: () -> Void

    var body: some View {
        HStack(spacing: 14) {
            Toggle("", isOn: Binding(get: { isSelected }, set: { _ in onToggle() }))
                .toggleStyle(.checkbox)
                .labelsHidden()

            Image(systemName: fileIcon)
                .font(.title3)
                .foregroundStyle(.secondary)
                .frame(width: 24)

            VStack(alignment: .leading, spacing: 3) {
                HStack(spacing: 6) {
                    Text(file.name)
                        .font(.callout)
                        .lineLimit(1)
                        .truncationMode(.middle)
                    if file.isOld {
                        Text("\(file.ageDays)일 전")
                            .font(.system(size: 9, weight: .bold))
                            .padding(.horizontal, 5)
                            .padding(.vertical, 2)
                            .background(.orange.opacity(0.15), in: Capsule())
                            .foregroundStyle(.orange)
                    }
                }
                Text(file.path)
                    .font(.caption2)
                    .foregroundStyle(.tertiary)
                    .lineLimit(1)
                    .truncationMode(.middle)
            }

            Spacer()

            Text(file.sizeFormatted)
                .font(.callout.bold())
                .foregroundStyle(providerColor)
                .frame(minWidth: 72, alignment: .trailing)
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
        .contentShape(Rectangle())
        .onTapGesture { onToggle() }
        .background(isSelected ? providerColor.opacity(0.05) : .clear)
    }

    private var fileIcon: String {
        let ext = (file.name as NSString).pathExtension.lowercased()
        switch ext {
        case "pdf": return "doc.text"
        case "jpg", "jpeg", "png", "heic", "gif": return "photo"
        case "mp4", "mov", "avi", "mkv": return "film"
        case "mp3", "wav", "m4a": return "music.note"
        case "zip", "rar", "7z": return "archivebox"
        case "doc", "docx", "pages": return "doc.richtext"
        case "xls", "xlsx", "numbers": return "tablecells"
        case "ppt", "pptx", "keynote": return "rectangle.split.3x1"
        default: return "doc"
        }
    }
}

// MARK: - Cloud Summary Chip

private struct CloudSummaryChip: View {
    let label: String
    let value: String
    let icon: String
    let color: Color

    var body: some View {
        HStack(spacing: 8) {
            Image(systemName: icon)
                .foregroundStyle(color)
            VStack(alignment: .leading, spacing: 1) {
                Text(value)
                    .font(.callout.bold())
                Text(label)
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            }
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
        .background(color.opacity(0.08), in: RoundedRectangle(cornerRadius: 8))
    }
}
