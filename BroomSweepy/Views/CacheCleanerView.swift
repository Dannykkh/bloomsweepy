import SwiftUI
import AppKit

struct CacheCleanerView: View {
    @Bindable var viewModel: CleanerViewModel
    @State private var showConfirm = false
    @State private var emptyBounce = 0
    @State private var hasScanned = false

    private var selectedSize: Int64 {
        viewModel.cacheItems
            .filter { viewModel.selectedCacheIDs.contains($0.id) }
            .reduce(Int64(0)) { $0 + $1.size }
    }

    private var movableIDs: Set<UUID> {
        Set(viewModel.cacheItems.filter { $0.snapshot.kind == .regularFile }.map(\.id))
    }

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Text("캐시 / 임시파일 정리")
                    .font(.title2.bold())
                Spacer()
                if !viewModel.cacheItems.isEmpty {
                    Button("다시 스캔") { Task { await viewModel.scanCache(); hasScanned = true } }
                        .buttonStyle(.bordered)
                        .disabled(viewModel.isScanning)
                }
                Button("선택 파일 휴지통으로 이동") { showConfirm = true }
                    .buttonStyle(.borderedProminent)
                    .tint(.red)
                    .disabled(viewModel.selectedCacheIDs.isEmpty || viewModel.isScanning)
                    .help("선택한 캐시를 휴지통으로 이동합니다")
            }
            .padding(24)


            // Select All
            if !viewModel.cacheItems.isEmpty {
                HStack {
                    Toggle("전체 선택", isOn: Binding(
                        get: {
                            !movableIDs.isEmpty && movableIDs.isSubset(of: viewModel.selectedCacheIDs)
                        },
                        set: { selectAll in
                            if selectAll {
                                viewModel.selectedCacheIDs.formUnion(movableIDs)
                            } else {
                                viewModel.selectedCacheIDs.subtract(movableIDs)
                            }
                        }
                    ))
                    .toggleStyle(.checkbox)
                    .disabled(movableIDs.isEmpty)
                    Spacer()
                }
                .padding(.horizontal, 24)
                .padding(.bottom, 8)
            }

            // List
            if viewModel.isScanning {
                scanningState
            } else if viewModel.cacheItems.isEmpty {
                emptyState
            } else {
                let maxSize = viewModel.cacheItems.map(\.size).max() ?? 1
                List(viewModel.cacheItems) { item in
                    CacheRow(item: item, maxSize: maxSize,
                             isSelected: viewModel.selectedCacheIDs.contains(item.id)) {
                        if viewModel.selectedCacheIDs.contains(item.id) {
                            viewModel.selectedCacheIDs.remove(item.id)
                        } else {
                            viewModel.selectedCacheIDs.insert(item.id)
                        }
                    }
                }
                .listStyle(.inset(alternatesRowBackgrounds: true))

                // 휴지통 이동 실패 안내
                if !viewModel.cleanErrors.isEmpty {
                    VStack(alignment: .leading, spacing: 8) {
                        HStack(spacing: 6) {
                            Image(systemName: "exclamationmark.triangle.fill")
                                .foregroundStyle(.orange)
                            Text("\(viewModel.cleanErrors.count)개 항목을 휴지통으로 이동하지 못했습니다")
                                .font(.callout.bold())
                                .foregroundStyle(.orange)
                        }
                        Text(viewModel.cleanErrors[0])
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                    .padding(16)
                    .background(.orange.opacity(0.05), in: RoundedRectangle(cornerRadius: 10))
                    .overlay(
                        RoundedRectangle(cornerRadius: 10)
                            .stroke(.orange.opacity(0.15), lineWidth: 1)
                    )
                    .padding(.horizontal, 24)
                    .padding(.bottom, 12)
                }
            }
        }
        .alert("휴지통으로 이동하기 전 최종 확인", isPresented: $showConfirm) {
            Button("취소", role: .cancel) {}
            Button("휴지통으로 이동", role: .destructive) {
                Task { await viewModel.cleanSelectedCache() }
            }
        } message: {
            Text(
                "\(viewModel.selectedCacheIDs.count)개 캐시, \(formatSize(selectedSize))를 휴지통으로 이동합니다.\n\n" +
                "휴지통을 비우기 전에는 복원할 수 있으며 디스크 여유는 늘어나지 않습니다."
            )
        }
    }

    private var scanningState: some View {
        VStack(spacing: 16) {
            ProgressView()
                .scaleEffect(1.5)
                .progressViewStyle(.circular)
            Text(viewModel.scanMessage.isEmpty ? "스캔 중..." : viewModel.scanMessage)
                .font(.headline)
                .foregroundStyle(.secondary)
            if viewModel.scanProgress > 0 {
                ProgressView(value: viewModel.scanProgress)
                    .frame(maxWidth: 300)
            }
            Button("취소") {
                viewModel.cancelCurrentTask()
            }
            .buttonStyle(.bordered)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .transition(.opacity)
    }

    private var emptyState: some View {
        VStack(spacing: 20) {
            if hasScanned {
                // 스캔 완료 + 깨끗함
                Image(systemName: "checkmark.circle.fill")
                    .font(.system(size: 48))
                    .foregroundStyle(.green)
                    .symbolEffect(.bounce, value: emptyBounce)
                VStack(spacing: 6) {
                    Text("깨끗합니다!")
                        .font(.title3.bold())
                        .foregroundStyle(.green)
                    Text("정리할 캐시 파일이 없습니다")
                        .font(.callout)
                        .foregroundStyle(.secondary)
                }
                // 다음 추천
                VStack(spacing: 8) {
                    Text("다음 추천")
                        .font(.caption.bold())
                        .foregroundStyle(.secondary)
                    HStack(spacing: 12) {
                        MiniNextButton(icon: "doc.richtext.fill", title: "대용량 파일", color: .blue)
                        MiniNextButton(icon: "doc.on.doc.fill", title: "중복 파일", color: .orange)
                        MiniNextButton(icon: "gauge.with.dots.needle.67percent", title: "메모리 정리", color: .green)
                    }
                }
                .padding(.top, 16)
            } else {
                // 스캔 전 — 아이콘 클릭으로 바로 스캔
                Button {
                    Task { await viewModel.scanCache(); hasScanned = true }
                } label: {
                    VStack(spacing: 16) {
                        ZStack {
                            Circle()
                                .fill(.blue.opacity(0.1))
                                .frame(width: 88, height: 88)
                            Image(systemName: "internaldrive.fill")
                                .font(.system(size: 36))
                                .foregroundStyle(.blue)
                                .symbolEffect(.bounce, value: emptyBounce)
                        }
                        VStack(spacing: 8) {
                            Text("클릭하여 캐시 스캔")
                                .font(.title3.bold())
                            Text("앱들이 빠른 실행을 위해 임시 저장하는 데이터입니다\n개별 파일만 자동 이동하며 캐시 폴더는 Finder에서 검토합니다")
                                .font(.callout)
                                .foregroundStyle(.secondary)
                                .multilineTextAlignment(.center)
                            Text("휴지통에서 복원할 수 있으며, 비워야 디스크 여유가 늘어납니다")
                                .font(.caption)
                                .foregroundStyle(.green)
                        }
                    }
                }
                .buttonStyle(.plain)
                .onHover { h in
                    if h { NSCursor.pointingHand.push() } else { NSCursor.pop() }
                }
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .onAppear { emptyBounce += 1 }
    }
}

struct CacheRow: View {
    let item: CacheItem
    let maxSize: Int64
    let isSelected: Bool
    let onToggle: () -> Void

    private var canMoveAutomatically: Bool {
        item.snapshot.kind == .regularFile
    }

    var body: some View {
        HStack(spacing: 14) {
            Toggle("", isOn: Binding(get: { isSelected }, set: { _ in onToggle() }))
                .toggleStyle(.checkbox)
                .labelsHidden()
                .disabled(!canMoveAutomatically)

            Image(systemName: item.icon)
                .font(.title2)
                .foregroundColor(.accentColor)
                .frame(width: 32)

            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Text(item.name).font(.headline)
                    SafetyBadge(level: canMoveAutomatically ? .safe : .review)
                }
                Text(
                    canMoveAutomatically
                        ? "\(item.description)"
                        : "\(item.description) (\(item.fileCount)개 파일) · 폴더는 Finder 검토 전용"
                )
                    .font(.caption)
                    .foregroundStyle(.secondary)
                GeometryReader { geo in
                    RoundedRectangle(cornerRadius: 2)
                        .fill(Color.accentColor.opacity(0.3))
                        .frame(width: geo.size.width * CGFloat(item.size) / CGFloat(maxSize))
                }
                .frame(height: 4)
            }

            Spacer()

            if !canMoveAutomatically {
                Button("Finder에서 보기") {
                    NSWorkspace.shared.activateFileViewerSelecting([
                        URL(fileURLWithPath: item.path)
                    ])
                }
                .buttonStyle(.borderless)
            }

            Text(item.sizeFormatted)
                .font(.headline)
                .foregroundStyle(.red)
                .frame(minWidth: 80, alignment: .trailing)
        }
        .padding(.vertical, 4)
        .contentShape(Rectangle())
        .onTapGesture {
            if canMoveAutomatically { onToggle() }
        }
    }
}

// MARK: - Safety Badge

enum SafetyLevel: String {
    case safe = "안전"
    case review = "검토"
    case caution = "주의"

    var color: Color {
        switch self {
        case .safe: return .green
        case .review: return .orange
        case .caution: return .red
        }
    }
}

struct SafetyBadge: View {
    let level: SafetyLevel

    var body: some View {
        Text(level.rawValue)
            .font(.system(size: 10, weight: .bold))
            .padding(.horizontal, 6)
            .padding(.vertical, 2)
            .background(level.color.opacity(0.15), in: Capsule())
            .foregroundStyle(level.color)
    }
}
