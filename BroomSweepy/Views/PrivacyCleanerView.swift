import SwiftUI

struct PrivacyCleanerView: View {
    @Bindable var viewModel: CleanerViewModel
    @State private var browserItems: [BrowserData] = []
    @State private var selectedIDs: Set<UUID> = []
    @State private var isScanning = false
    @State private var showConfirm = false
    @State private var resultMessage: String?
    @State private var resultIsError = false

    private var totalSelectedSize: Int64 {
        browserItems
            .filter { selectedIDs.contains($0.id) }
            .reduce(0) { $0 + $1.size }
    }

    private var totalSize: Int64 {
        browserItems.reduce(0) { $0 + $1.size }
    }

    private var groupedItems: [(browser: String, icon: String, items: [BrowserData])] {
        let browsers = Dictionary(grouping: browserItems) { $0.browserName }
        return browsers
            .map { name, items in
                (browser: name, icon: items.first?.icon ?? "globe", items: items.sorted { $0.size > $1.size })
            }
            .sorted { $0.browser < $1.browser }
    }

    var body: some View {
        VStack(spacing: 0) {
            header

            if isScanning {
                scanningState
            } else if browserItems.isEmpty {
                emptyState
            } else {
                summaryBar
                itemList
            }
        }
        .alert("개인정보 데이터 정리", isPresented: $showConfirm) {
            Button("취소", role: .cancel) {}
            Button("정리", role: .destructive) { performClean() }
        } message: {
            Text("선택한 \(selectedIDs.count)개 항목 (\(ByteCountFormatter.string(fromByteCount: totalSelectedSize, countStyle: .file)))을 삭제하시겠습니까?\n\n브라우저에서 로그인 정보가 삭제될 수 있습니다.")
        }
    }

    // MARK: - Subviews

    private var header: some View {
        HStack(spacing: 12) {
            Text("브라우저 개인정보 정리")
                .font(.title2.bold())
            Spacer()
            if !browserItems.isEmpty {
                Button("전체 선택") {
                    if selectedIDs.count == browserItems.count {
                        selectedIDs.removeAll()
                    } else {
                        selectedIDs = Set(browserItems.map(\.id))
                    }
                }
                .buttonStyle(.bordered)
            }
            Button("스캔") {
                Task { await runScan() }
            }
            .buttonStyle(.bordered)
            .disabled(isScanning)
            Button("선택 항목 정리") { showConfirm = true }
                .buttonStyle(.borderedProminent)
                .tint(.red)
                .disabled(selectedIDs.isEmpty)
        }
        .padding(24)
    }

    private var summaryBar: some View {
        HStack(spacing: 24) {
            SummaryChip(
                label: "발견된 데이터",
                value: ByteCountFormatter.string(fromByteCount: totalSize, countStyle: .file),
                icon: "externaldrive.badge.exclamationmark",
                color: .orange
            )
            SummaryChip(
                label: "선택됨",
                value: ByteCountFormatter.string(fromByteCount: totalSelectedSize, countStyle: .file),
                icon: "checkmark.circle.fill",
                color: .blue
            )
            SummaryChip(
                label: "항목 수",
                value: "\(browserItems.count)개",
                icon: "list.bullet",
                color: .purple
            )
            Spacer()
            if let msg = resultMessage {
                HStack(spacing: 6) {
                    Image(systemName: resultIsError ? "exclamationmark.triangle" : "checkmark.circle.fill")
                        .foregroundStyle(resultIsError ? .orange : .green)
                    Text(msg)
                        .font(.callout)
                        .foregroundStyle(resultIsError ? .orange : .green)
                }
            }
        }
        .padding(.horizontal, 24)
        .padding(.vertical, 12)
        .background(.ultraThinMaterial)
    }

    private var itemList: some View {
        ScrollView {
            LazyVStack(spacing: 16, pinnedViews: []) {
                ForEach(groupedItems, id: \.browser) { group in
                    BrowserSection(
                        browserName: group.browser,
                        browserIcon: group.icon,
                        items: group.items,
                        selectedIDs: $selectedIDs
                    )
                }
            }
            .padding(20)
        }
    }

    private var scanningState: some View {
        VStack(spacing: 16) {
            ProgressView()
                .scaleEffect(1.5)
                .progressViewStyle(.circular)
            Text("브라우저 데이터 스캔 중...")
                .font(.headline)
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var emptyState: some View {
        VStack(spacing: 12) {
            Image(systemName: "hand.raised.slash")
                .font(.system(size: 48))
                .foregroundStyle(.secondary)
            Text("브라우저에 저장된 방문 기록, 쿠키, 캐시를 삭제합니다\nSafari, Chrome, Firefox, Edge, Arc 지원")
                .font(.callout)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
            Text("⚠️ 삭제 후 웹사이트 로그인이 해제될 수 있음")
                .font(.caption)
                .foregroundStyle(.orange)
                .padding(.top, 4)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    // MARK: - Actions

    @MainActor
    private func runScan() async {
        isScanning = true
        selectedIDs.removeAll()
        resultMessage = nil
        browserItems = await Task.detached {
            PrivacyCleaner.shared.scan()
        }.value
        isScanning = false
    }

    private func performClean() {
        let items = browserItems.filter { selectedIDs.contains($0.id) }
        Task {
            let result = await Task.detached {
                PrivacyCleaner.shared.clean(items: items)
            }.value
            selectedIDs.removeAll()
            browserItems = browserItems.filter { FileManager.default.fileExists(atPath: $0.path) || $0.size == 0 }
            if result.errors.isEmpty {
                resultMessage = "\(ByteCountFormatter.string(fromByteCount: result.freed, countStyle: .file)) 정리 완료"
                resultIsError = false
            } else {
                resultMessage = "일부 항목 실패 (\(result.errors.count)건)"
                resultIsError = true
            }
            viewModel.toastMessage = resultMessage
            await runScan()
        }
    }
}

// MARK: - Browser Section

private struct BrowserSection: View {
    let browserName: String
    let browserIcon: String
    let items: [BrowserData]
    @Binding var selectedIDs: Set<UUID>

    private var allSelected: Bool {
        items.allSatisfy { selectedIDs.contains($0.id) }
    }

    private var sectionTotal: Int64 {
        items.reduce(0) { $0 + $1.size }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            // Section header
            HStack(spacing: 10) {
                Image(systemName: browserIcon)
                    .font(.title3)
                    .foregroundStyle(.secondary)
                Text(browserName)
                    .font(.headline)
                Spacer()
                Text(ByteCountFormatter.string(fromByteCount: sectionTotal, countStyle: .file))
                    .font(.callout)
                    .foregroundStyle(.secondary)
                Toggle("", isOn: Binding(
                    get: { allSelected },
                    set: { select in
                        if select {
                            items.forEach { selectedIDs.insert($0.id) }
                        } else {
                            items.forEach { selectedIDs.remove($0.id) }
                        }
                    }
                ))
                .toggleStyle(.checkbox)
                .labelsHidden()
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 12)
            .background(.ultraThinMaterial)

            Divider()

            ForEach(items) { item in
                PrivacyDataRow(item: item, isSelected: selectedIDs.contains(item.id)) {
                    if selectedIDs.contains(item.id) {
                        selectedIDs.remove(item.id)
                    } else {
                        selectedIDs.insert(item.id)
                    }
                }
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

// MARK: - Privacy Data Row

private struct PrivacyDataRow: View {
    let item: BrowserData
    let isSelected: Bool
    let onToggle: () -> Void

    private var typeColor: Color {
        switch item.dataType {
        case .history: return .red
        case .cookies: return .orange
        case .cache: return .blue
        case .downloads: return .purple
        case .localStorage: return .teal
        case .sessions: return .indigo
        }
    }

    var body: some View {
        HStack(spacing: 14) {
            Toggle("", isOn: Binding(get: { isSelected }, set: { _ in onToggle() }))
                .toggleStyle(.checkbox)
                .labelsHidden()

            Image(systemName: item.dataType.icon)
                .font(.title3)
                .foregroundStyle(typeColor)
                .frame(width: 28)

            VStack(alignment: .leading, spacing: 3) {
                Text(item.dataType.rawValue)
                    .font(.callout.weight(.medium))
                Text(item.path)
                    .font(.caption2)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                    .truncationMode(.middle)
            }

            Spacer()

            Text(item.sizeFormatted)
                .font(.callout.bold())
                .foregroundStyle(typeColor)
                .frame(minWidth: 72, alignment: .trailing)
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 10)
        .contentShape(Rectangle())
        .onTapGesture { onToggle() }
        .background(isSelected ? typeColor.opacity(0.05) : .clear)
    }
}

// MARK: - Summary Chip

private struct SummaryChip: View {
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
