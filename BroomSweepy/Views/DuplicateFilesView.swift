import SwiftUI

struct DuplicateFilesView: View {
    @Bindable var viewModel: CleanerViewModel
    @State private var showConfirm = false
    @State private var hasScanned = false

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Text("중복 파일 탐색")
                    .font(.title2.bold())
                Spacer()
                if !viewModel.duplicateGroups.isEmpty {
                    Button("다시 스캔") { Task { await viewModel.scanDuplicates(); hasScanned = true } }
                        .buttonStyle(.bordered)
                        .disabled(viewModel.isScanning)
                }
                Button("선택 항목 휴지통으로 이동") { showConfirm = true }
                    .buttonStyle(.borderedProminent)
                    .tint(.red)
                    .disabled(viewModel.selectedDuplicateFileIDs.isEmpty || viewModel.isScanning)
                    .help("경로순으로 정한 보관할 파일을 남기고 선택한 복사본만 휴지통으로 이동합니다")
            }
            .padding(24)


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
                    Button("취소") { viewModel.cancelCurrentTask() }
                        .buttonStyle(.bordered)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .transition(.opacity)
            } else if viewModel.duplicateGroups.isEmpty {
                VStack(spacing: 20) {
                    if hasScanned {
                        Image(systemName: "checkmark.circle.fill")
                            .font(.system(size: 48))
                            .foregroundStyle(.green)
                        VStack(spacing: 6) {
                            Text("깨끗합니다!")
                                .font(.title3.bold())
                                .foregroundStyle(.green)
                            Text("중복 파일이 발견되지 않았습니다")
                                .font(.callout)
                                .foregroundStyle(.secondary)
                        }
                        VStack(spacing: 8) {
                            Text("다음 추천")
                                .font(.caption.bold())
                                .foregroundStyle(.secondary)
                            HStack(spacing: 12) {
                                MiniNextButton(icon: "shield.checkered", title: "보안 검사", color: .orange)
                                MiniNextButton(icon: "folder.fill", title: "파일 관리", color: .cyan)
                                MiniNextButton(icon: "memorychip.fill", title: "메모리", color: .green)
                            }
                        }
                        .padding(.top, 16)
                    } else {
                        Button {
                            Task { await viewModel.scanDuplicates(); hasScanned = true }
                        } label: {
                            VStack(spacing: 16) {
                                ZStack {
                                    Circle()
                                        .fill(.orange.opacity(0.1))
                                        .frame(width: 88, height: 88)
                                    Image(systemName: "doc.on.doc.fill")
                                        .font(.system(size: 36))
                                        .foregroundStyle(.orange)
                                }
                                VStack(spacing: 8) {
                                    Text("클릭하여 중복 파일 스캔")
                                        .font(.title3.bold())
                                    Text("파일 전체 내용이 같은지 확인합니다\n경로순 보관할 파일을 남기고 복사본만 휴지통으로 이동할 수 있습니다")
                                        .font(.callout)
                                        .foregroundStyle(.secondary)
                                        .multilineTextAlignment(.center)
                                    Text("보관할 파일 자동 보호 — 선택한 복사본만 휴지통으로 이동")
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
                ScrollView {
                    LazyVStack(spacing: 16) {
                        ForEach(viewModel.duplicateGroups) { group in
                            DuplicateGroupCard(group: group,
                                               selectedIDs: $viewModel.selectedDuplicateFileIDs)
                        }
                    }
                    .padding(24)
                }
            }
        }
        .alert("휴지통으로 이동하기 전 최종 확인", isPresented: $showConfirm) {
            Button("취소", role: .cancel) {}
            Button("휴지통으로 이동", role: .destructive) {
                Task { await viewModel.deleteSelectedDuplicates() }
            }
        } message: {
            let selectedSize = viewModel.duplicateGroups
                .flatMap(\.files)
                .filter { viewModel.selectedDuplicateFileIDs.contains($0.id) }
                .reduce(Int64(0)) { $0 + $1.size }
            Text(
                "\(viewModel.selectedDuplicateFileIDs.count)개 복사본, \(formatSize(selectedSize))를 휴지통으로 이동합니다.\n\n" +
                "경로순으로 정한 보관할 파일과 선택 파일의 전체 내용을 이동 직전에 다시 확인합니다. " +
                "휴지통을 비우기 전에는 복원할 수 있으며 디스크 여유는 늘어나지 않습니다."
            )
        }
    }
}

struct DuplicateGroupCard: View {
    let group: DuplicateGroup
    @Binding var selectedIDs: Set<UUID>

    private var orderedFiles: [DuplicateFile] {
        group.files.sorted { $0.path < $1.path }
    }

    var body: some View {
        VStack(spacing: 0) {
            // Group Header
            HStack {
                VStack(alignment: .leading, spacing: 2) {
                    Text(orderedFiles.first?.name ?? "")
                        .font(.headline)
                    Text("\(group.count)개 복사본 · 각 \(group.eachSizeFormatted)")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                Spacer()
                Text("\(group.wastedSizeFormatted) 낭비")
                    .font(.headline)
                    .foregroundStyle(.orange)
            }
            .padding(16)
            .background(.ultraThinMaterial)

            Divider()

            // Files
            VStack(spacing: 0) {
                ForEach(Array(orderedFiles.enumerated()), id: \.element.id) { index, file in
                    HStack(spacing: 12) {
                        if index > 0 {
                            Toggle("", isOn: Binding(
                                get: { selectedIDs.contains(file.id) },
                                set: { selected in
                                    if selected { selectedIDs.insert(file.id) }
                                    else { selectedIDs.remove(file.id) }
                                }
                            ))
                            .toggleStyle(.checkbox)
                            .labelsHidden()
                        } else {
                            Image(systemName: "checkmark.shield.fill")
                                .foregroundStyle(.green)
                                .frame(width: 16)
                        }

                        Text(shortenPath(file.path))
                            .font(.caption)
                            .foregroundStyle(.secondary)
                            .lineLimit(1)
                            .truncationMode(.middle)

                        Spacer()

                        if index == 0 {
                            Text("보관할 파일")
                                .font(.system(size: 10, weight: .bold))
                                .padding(.horizontal, 6)
                                .padding(.vertical, 2)
                                .background(.green.opacity(0.15), in: Capsule())
                                .foregroundStyle(.green)
                        }
                    }
                    .padding(.horizontal, 16)
                    .padding(.vertical, 8)
                    .contentShape(Rectangle())
                    .onTapGesture {
                        guard index > 0 else { return }
                        if selectedIDs.contains(file.id) {
                            selectedIDs.remove(file.id)
                        } else {
                            selectedIDs.insert(file.id)
                        }
                    }

                    if index < orderedFiles.count - 1 {
                        Divider().padding(.leading, 44)
                    }
                }
            }
        }
        .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: 12))
        .overlay(RoundedRectangle(cornerRadius: 12).stroke(.quaternary))
    }

    private func shortenPath(_ path: String) -> String {
        let home = NSHomeDirectory()
        if path.hasPrefix(home) {
            return "~" + path.dropFirst(home.count)
        }
        return path
    }
}
