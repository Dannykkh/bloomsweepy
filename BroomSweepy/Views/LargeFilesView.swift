import SwiftUI

struct LargeFilesView: View {
    @Bindable var viewModel: CleanerViewModel
    @State private var selectedCategory: LargeFile.FileCategory?
    @State private var showConfirm = false
    @State private var hasScanned = false

    private var filteredFiles: [LargeFile] {
        guard let cat = selectedCategory else { return viewModel.largeFiles }
        return viewModel.largeFiles.filter { $0.category == cat }
    }

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Text("대용량 파일 탐색")
                    .font(.title2.bold())
                Spacer()
                if !viewModel.largeFiles.isEmpty {
                    Button("다시 스캔") { Task { await viewModel.scanLargeFiles(); hasScanned = true } }
                        .buttonStyle(.bordered)
                        .disabled(viewModel.isScanning)
                }
                Button("선택 항목 삭제") { showConfirm = true }
                    .buttonStyle(.borderedProminent)
                    .tint(.red)
                    .disabled(viewModel.selectedLargeFileIDs.isEmpty)
                    .help("선택한 파일을 휴지통으로 이동합니다")
            }
            .padding(24)


            // Category Filters
            if !viewModel.largeFiles.isEmpty {
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 8) {
                        FilterChip(title: "전체", isActive: selectedCategory == nil) {
                            selectedCategory = nil
                        }
                        ForEach(availableCategories, id: \.self) { cat in
                            FilterChip(title: cat.rawValue, isActive: selectedCategory == cat) {
                                selectedCategory = cat
                            }
                        }
                    }
                    .padding(.horizontal, 24)
                }
                .padding(.bottom, 12)
            }

            // List
            if viewModel.isScanning {
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
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .transition(.opacity)
            } else if viewModel.largeFiles.isEmpty {
                VStack(spacing: 20) {
                    if hasScanned {
                        Image(systemName: "checkmark.circle.fill")
                            .font(.system(size: 48))
                            .foregroundStyle(.green)
                        VStack(spacing: 6) {
                            Text("깨끗합니다!")
                                .font(.title3.bold())
                                .foregroundStyle(.green)
                            Text("50MB 이상의 대용량 파일이 없습니다")
                                .font(.callout)
                                .foregroundStyle(.secondary)
                        }
                        VStack(spacing: 8) {
                            Text("다음 추천")
                                .font(.caption.bold())
                                .foregroundStyle(.secondary)
                            HStack(spacing: 12) {
                                MiniNextButton(icon: "doc.on.doc.fill", title: "중복 파일", color: .orange)
                                MiniNextButton(icon: "hand.raised.fill", title: "개인정보", color: .purple)
                                MiniNextButton(icon: "shield.checkered", title: "보안 검사", color: .orange)
                            }
                        }
                        .padding(.top, 16)
                    } else {
                        Button {
                            Task { await viewModel.scanLargeFiles(); hasScanned = true }
                        } label: {
                            VStack(spacing: 16) {
                                ZStack {
                                    Circle()
                                        .fill(.blue.opacity(0.1))
                                        .frame(width: 88, height: 88)
                                    Image(systemName: "doc.richtext.fill")
                                        .font(.system(size: 36))
                                        .foregroundStyle(.blue)
                                }
                                VStack(spacing: 8) {
                                    Text("클릭하여 대용량 파일 스캔")
                                        .font(.title3.bold())
                                    Text("50MB 이상의 큰 파일을 찾아줍니다\n오래된 설치파일, 압축파일, 동영상 등을 확인하세요")
                                        .font(.callout)
                                        .foregroundStyle(.secondary)
                                        .multilineTextAlignment(.center)
                                    Text("✅ 휴지통으로 이동 — 실수해도 복구 가능")
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
            } else {
                List(filteredFiles) { file in
                    LargeFileRow(file: file,
                                 isSelected: viewModel.selectedLargeFileIDs.contains(file.id)) {
                        if viewModel.selectedLargeFileIDs.contains(file.id) {
                            viewModel.selectedLargeFileIDs.remove(file.id)
                        } else {
                            viewModel.selectedLargeFileIDs.insert(file.id)
                        }
                    }
                }
                .listStyle(.inset(alternatesRowBackgrounds: true))
            }
        }
        .alert("파일 삭제", isPresented: $showConfirm) {
            Button("취소", role: .cancel) {}
            Button("삭제", role: .destructive) { Task { await viewModel.deleteSelectedLargeFiles() } }
        } message: {
            Text("\(viewModel.selectedLargeFileIDs.count)개 파일을 삭제하시겠습니까?")
        }
    }

    private var availableCategories: [LargeFile.FileCategory] {
        Array(Set(viewModel.largeFiles.map(\.category))).sorted { $0.rawValue < $1.rawValue }
    }
}

struct LargeFileRow: View {
    let file: LargeFile
    let isSelected: Bool
    let onToggle: () -> Void

    private var safetyLevel: SafetyLevel {
        if file.ageDays > 180 { return .review }
        if file.category == .installer || file.category == .backup { return .caution }
        return .review
    }

    var body: some View {
        HStack(spacing: 14) {
            Toggle("", isOn: Binding(get: { isSelected }, set: { _ in onToggle() }))
                .toggleStyle(.checkbox)
                .labelsHidden()

            Image(systemName: file.category.icon)
                .font(.title2)
                .foregroundStyle(.blue)
                .frame(width: 32)

            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Text(file.name).font(.headline).lineLimit(1)
                    SafetyBadge(level: safetyLevel)
                }
                Text("\(file.category.rawValue) · \(file.ageDays)일 전 · \(shortenPath(file.path))")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }

            Spacer()

            VStack(alignment: .trailing) {
                Text(file.sizeFormatted)
                    .font(.headline)
                    .foregroundStyle(.blue)
                Text(file.ext)
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            }
            .frame(minWidth: 80)
        }
        .padding(.vertical, 4)
        .contentShape(Rectangle())
        .onTapGesture { onToggle() }
        .contextMenu {
            Button { NSWorkspace.shared.activateFileViewerSelecting([URL(fileURLWithPath: file.path)]) } label: {
                Label("Finder에서 보기", systemImage: "magnifyingglass")
            }
            Divider()
            Button(role: .destructive) {
                try? FileManager.default.trashItem(at: URL(fileURLWithPath: file.path), resultingItemURL: nil)
            } label: {
                Label("휴지통으로 이동", systemImage: "trash")
            }
        }
    }

    private func shortenPath(_ path: String) -> String {
        let home = NSHomeDirectory()
        if path.hasPrefix(home) {
            return "~" + path.dropFirst(home.count)
        }
        return path
    }
}

struct FilterChip: View {
    let title: String
    let isActive: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Text(title)
                .font(.caption.bold())
                .padding(.horizontal, 12)
                .padding(.vertical, 6)
                .background(isActive ? Color.accentColor.opacity(0.2) : Color.secondary.opacity(0.1),
                            in: Capsule())
                .foregroundColor(isActive ? .accentColor : .secondary)
        }
        .buttonStyle(.plain)
    }
}
