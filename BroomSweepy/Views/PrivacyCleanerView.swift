import SwiftUI
import AppKit

struct PrivacyCleanerView: View {
    @Bindable var viewModel: CleanerViewModel
    @State private var browserItems: [BrowserData] = []
    @State private var selectedIDs: Set<UUID> = []
    @State private var isScanning = false
    @State private var showConfirm = false
    @State private var resultMessage: String?
    @State private var resultIsError = false
    @State private var runningBrowserNames: Set<String> = []

    private var totalSelectedSize: Int64 {
        browserItems
            .filter { selectedIDs.contains($0.id) }
            .reduce(0) { $0 + $1.size }
    }

    private var totalSize: Int64 {
        browserItems.reduce(0) { $0 + $1.size }
    }

    private var movableIDs: Set<UUID> {
        Set(browserItems.filter { $0.snapshot.kind == .regularFile }.map(\.id))
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
        .alert("휴지통으로 이동하기 전 최종 확인", isPresented: $showConfirm) {
            Button("취소", role: .cancel) {}
            Button("휴지통으로 이동", role: .destructive) { performClean() }
        } message: {
            Text(
                "선택한 \(selectedIDs.count)개 항목, " +
                "\(ByteCountFormatter.string(fromByteCount: totalSelectedSize, countStyle: .file))를 휴지통으로 이동합니다.\n\n" +
                (runningBrowserNames.isEmpty
                    ? ""
                    : "실행 중인 브라우저(\(runningBrowserNames.sorted().joined(separator: ", ")))의 방문 기록·쿠키·세션·프로필 데이터는 이동하지 않습니다.\n\n") +
                "캐시는 다시 만들어질 수 있지만 방문 기록·쿠키·세션은 로그인과 작업 상태를 잃게 할 수 있습니다. 휴지통을 비우기 전에는 파일을 복원할 수 있으며 디스크 여유는 늘어나지 않습니다."
            )
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
                    if !movableIDs.isEmpty && movableIDs.isSubset(of: selectedIDs) {
                        selectedIDs.subtract(movableIDs)
                    } else {
                        selectedIDs.formUnion(movableIDs)
                    }
                }
                .buttonStyle(.bordered)
                .disabled(movableIDs.isEmpty)
            }
            Button("스캔") {
                Task { await runScan() }
            }
            .buttonStyle(.bordered)
            .disabled(isScanning)
            Button("선택 항목 휴지통으로 이동") { prepareCleanConfirmation() }
                .buttonStyle(.borderedProminent)
                .tint(.red)
                .disabled(selectedIDs.isEmpty || isScanning)
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
            Text("브라우저에 저장된 방문 기록·쿠키 파일을 검토합니다\n폴더 항목은 Finder 검토 전용이며 자동으로 이동하지 않습니다")
                .font(.callout)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
            Text("웹사이트 로그인이 해제될 수 있으며, 휴지통을 비워야 디스크 여유가 늘어납니다")
                .font(.caption)
                .foregroundStyle(.orange)
                .padding(.top, 4)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    // MARK: - Actions

    @MainActor
    private func runScan() async {
        guard !isScanning else { return }
        guard let homeURL = FileAccessManager.shared.loadBookmark()
                ?? FileAccessManager.shared.requestHomeAccess() else {
            resultMessage = "홈 폴더 접근 권한이 필요합니다"
            resultIsError = true
            return
        }
        isScanning = true
        selectedIDs.removeAll()
        resultMessage = nil
        browserItems = await Task.detached {
            PrivacyCleaner.shared.scan(homeURL: homeURL)
        }.value
        isScanning = false
    }

    @MainActor
    private func prepareCleanConfirmation() {
        runningBrowserNames = detectRunningBrowsers()
        showConfirm = true
    }

    @MainActor
    private func performClean() {
        guard !isScanning else { return }
        let items = browserItems.filter { selectedIDs.contains($0.id) }
        guard !items.isEmpty else { return }
        let running = detectRunningBrowsers()
        isScanning = true
        Task {
            let result = await Task.detached {
                PrivacyCleaner.shared.clean(items: items, runningBrowsers: running)
            }.value
            selectedIDs.subtract(result.movedIDs)
            browserItems.removeAll { result.movedIDs.contains($0.id) }
            if result.freed > 0 {
                HealthMonitor.shared.recordClean()
                CleanHistory.shared.record(freed: result.freed, type: "manual")
            }
            let message: String
            let isError: Bool
            if result.errors.isEmpty {
                message = "휴지통으로 이동한 논리 용량: \(ByteCountFormatter.string(fromByteCount: result.freed, countStyle: .file))"
                isError = false
            } else {
                message = "휴지통으로 이동한 논리 용량: \(ByteCountFormatter.string(fromByteCount: result.freed, countStyle: .file)) · " +
                    "\(result.errors.count)개 실패: \(result.errors[0])"
                isError = true
            }
            isScanning = false
            await runScan()
            resultMessage = message
            resultIsError = isError
            viewModel.toastMessage = message
        }
    }

    @MainActor
    private func detectRunningBrowsers() -> Set<String> {
        let identifiers: [String: String] = [
            "com.google.Chrome": "Chrome",
            "com.apple.Safari": "Safari",
            "org.mozilla.firefox": "Firefox",
            "com.microsoft.edgemac": "Edge",
            "company.thebrowser.Browser": "Arc",
            "com.brave.Browser": "Brave",
        ]
        return Set(NSWorkspace.shared.runningApplications.compactMap { app in
            guard let bundleID = app.bundleIdentifier else { return nil }
            return identifiers[bundleID]
        })
    }
}

// MARK: - Browser Section

private struct BrowserSection: View {
    let browserName: String
    let browserIcon: String
    let items: [BrowserData]
    @Binding var selectedIDs: Set<UUID>

    private var movableItems: [BrowserData] {
        items.filter { $0.snapshot.kind == .regularFile }
    }

    private var allSelected: Bool {
        !movableItems.isEmpty && movableItems.allSatisfy { selectedIDs.contains($0.id) }
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
                            movableItems.forEach { selectedIDs.insert($0.id) }
                        } else {
                            movableItems.forEach { selectedIDs.remove($0.id) }
                        }
                    }
                ))
                .toggleStyle(.checkbox)
                .labelsHidden()
                .disabled(movableItems.isEmpty)
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

    private var canMoveAutomatically: Bool {
        item.snapshot.kind == .regularFile
    }

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
                .disabled(!canMoveAutomatically)

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
                if !canMoveAutomatically {
                    Text("폴더 · Finder 검토 전용")
                        .font(.caption2)
                        .foregroundStyle(.orange)
                }
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
        .onTapGesture {
            if canMoveAutomatically { onToggle() }
        }
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
