import SwiftUI

struct SimilarImagesView: View {
    @State private var groups: [SimilarImageGroup] = []
    @State private var selectedImageIDs: Set<UUID> = []
    @State private var isScanning = false
    @State private var scanMessage = ""
    @State private var showConfirm = false
    @State private var toastMessage: String?

    private var totalWasted: Int64 {
        groups.reduce(Int64(0)) { $0 + $1.wastedSize }
    }

    var body: some View {
        VStack(spacing: 0) {
            header
            content
        }
        .overlay(alignment: .bottom) {
            if let msg = toastMessage {
                toastView(msg)
            }
        }
        .alert("유사 이미지 삭제", isPresented: $showConfirm) {
            Button("취소", role: .cancel) {}
            Button("휴지통으로 이동", role: .destructive) { deleteSelected() }
        } message: {
            Text("\(selectedImageIDs.count)개 이미지를 휴지통으로 이동하시겠습니까?")
        }
    }

    // MARK: - Header

    private var header: some View {
        HStack(spacing: 12) {
            VStack(alignment: .leading, spacing: 2) {
                Text("유사 이미지 탐색")
                    .font(.title2.bold())
                if !groups.isEmpty {
                    Text("\(groups.count)개 그룹 · 낭비 용량: \(formatSize(totalWasted))")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }

            Spacer()

            Button("폴더 선택 후 스캔") {
                Task { await scanFolder() }
            }
            .buttonStyle(.bordered)
            .disabled(isScanning)

            Button("선택 항목 삭제") { showConfirm = true }
                .buttonStyle(.borderedProminent)
                .tint(.red)
                .disabled(selectedImageIDs.isEmpty)
        }
        .padding(24)
    }

    // MARK: - Content

    @ViewBuilder
    private var content: some View {
        if isScanning {
            VStack(spacing: 16) {
                ProgressView()
                    .scaleEffect(1.5)
                    .progressViewStyle(.circular)
                Text(scanMessage.isEmpty ? "스캔 중..." : scanMessage)
                    .font(.headline)
                    .foregroundStyle(.secondary)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .transition(.opacity)
        } else if groups.isEmpty {
            VStack(spacing: 12) {
                Image(systemName: "photo.on.rectangle.angled")
                    .font(.system(size: 40))
                    .foregroundStyle(.secondary)
                Text("'폴더 선택 후 스캔' 버튼을 눌러\n유사 이미지를 탐색하세요")
                    .multilineTextAlignment(.center)
                    .foregroundStyle(.secondary)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        } else {
            ScrollView {
                LazyVStack(spacing: 16) {
                    ForEach(groups) { group in
                        SimilarImageGroupCard(
                            group: group,
                            selectedIDs: $selectedImageIDs
                        )
                    }
                }
                .padding(24)
            }
        }
    }

    // MARK: - Actions

    @MainActor
    private func scanFolder() async {
        guard let folderURL = FileAccessManager.shared.requestFolderAccess(
            message: "유사 이미지를 검색할 폴더를 선택하세요"
        ) else { return }

        isScanning = true
        scanMessage = "이미지 스캔 중..."
        selectedImageIDs.removeAll()

        groups = await Task.detached {
            SimilarImageFinder.shared.scan(folderURL: folderURL) { msg, _ in
                Task { @MainActor in
                    scanMessage = msg
                }
            }
        }.value

        isScanning = false

        if groups.isEmpty {
            toastMessage = "유사 이미지를 찾지 못했습니다"
        } else {
            toastMessage = "\(groups.count)개 유사 이미지 그룹을 발견했습니다"
        }
    }

    private func deleteSelected() {
        let paths = selectedImageIDs.compactMap { id in
            groups.flatMap(\.images).first { $0.id == id }?.path
        }
        guard !paths.isEmpty else { return }

        var totalFreed: Int64 = 0
        let fm = FileManager.default
        for path in paths {
            do {
                let attrs = try fm.attributesOfItem(atPath: path)
                let size = (attrs[.size] as? Int64) ?? 0
                try fm.trashItem(at: URL(fileURLWithPath: path), resultingItemURL: nil)
                totalFreed += size
            } catch { }
        }

        selectedImageIDs.removeAll()
        toastMessage = "삭제 완료! \(formatSize(totalFreed)) 확보"

        // Re-scan to refresh
        groups = groups.compactMap { group in
            let remaining = group.images.filter { img in
                !paths.contains(img.path)
            }
            guard remaining.count >= 2 else { return nil }
            return SimilarImageGroup(images: remaining)
        }
    }

    // MARK: - Toast

    private func toastView(_ message: String) -> some View {
        Text(message)
            .font(.callout.bold())
            .padding(.horizontal, 24)
            .padding(.vertical, 12)
            .background(.green, in: Capsule())
            .foregroundStyle(.white)
            .padding(.bottom, 24)
            .transition(.move(edge: .bottom).combined(with: .opacity))
            .onAppear {
                DispatchQueue.main.asyncAfter(deadline: .now() + 3) {
                    withAnimation { toastMessage = nil }
                }
            }
    }
}

// MARK: - Similar Image Group Card

struct SimilarImageGroupCard: View {
    let group: SimilarImageGroup
    @Binding var selectedIDs: Set<UUID>

    var body: some View {
        VStack(spacing: 0) {
            // Group header
            HStack {
                VStack(alignment: .leading, spacing: 2) {
                    Text("\(group.images.count)개 유사 이미지")
                        .font(.headline)
                    Text("낭비 용량: \(group.wastedSizeFormatted)")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                Spacer()
                Text(group.wastedSizeFormatted)
                    .font(.headline)
                    .foregroundColor(.orange)
            }
            .padding(16)
            .background(.ultraThinMaterial)

            Divider()

            // Image grid
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 12) {
                    ForEach(Array(group.images.enumerated()), id: \.element.id) { index, image in
                        SimilarImageCell(
                            image: image,
                            isOriginal: index == 0,
                            isSelected: selectedIDs.contains(image.id),
                            canSelect: index > 0
                        ) {
                            guard index > 0 else { return }
                            if selectedIDs.contains(image.id) {
                                selectedIDs.remove(image.id)
                            } else {
                                selectedIDs.insert(image.id)
                            }
                        }
                    }
                }
                .padding(16)
            }
        }
        .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: 12))
        .overlay(RoundedRectangle(cornerRadius: 12).stroke(.quaternary))
    }
}

// MARK: - Similar Image Cell

struct SimilarImageCell: View {
    let image: SimilarImage
    let isOriginal: Bool
    let isSelected: Bool
    let canSelect: Bool
    let onToggle: () -> Void

    var body: some View {
        VStack(spacing: 8) {
            ZStack(alignment: .topLeading) {
                // Thumbnail
                if let thumb = image.thumbnail {
                    Image(nsImage: thumb)
                        .resizable()
                        .aspectRatio(contentMode: .fill)
                        .frame(width: 120, height: 120)
                        .clipShape(RoundedRectangle(cornerRadius: 8))
                } else {
                    RoundedRectangle(cornerRadius: 8)
                        .fill(.secondary.opacity(0.2))
                        .frame(width: 120, height: 120)
                        .overlay {
                            Image(systemName: "photo")
                                .font(.title)
                                .foregroundStyle(.secondary)
                        }
                }

                // Badge
                if isOriginal {
                    Text("원본")
                        .font(.system(size: 10, weight: .bold))
                        .padding(.horizontal, 6)
                        .padding(.vertical, 2)
                        .background(.green.opacity(0.85), in: Capsule())
                        .foregroundColor(.white)
                        .padding(4)
                } else if isSelected {
                    Image(systemName: "checkmark.circle.fill")
                        .font(.title3)
                        .foregroundColor(.accentColor)
                        .padding(4)
                }
            }
            .overlay(
                RoundedRectangle(cornerRadius: 8)
                    .stroke(isSelected ? Color.accentColor : Color.clear, lineWidth: 2)
            )

            // Info
            Text(image.name)
                .font(.caption)
                .lineLimit(1)
                .truncationMode(.middle)
                .frame(width: 120)

            Text(image.sizeFormatted)
                .font(.caption2)
                .foregroundStyle(.secondary)
        }
        .contentShape(Rectangle())
        .onTapGesture { onToggle() }
        .opacity(canSelect ? 1.0 : 0.9)
    }
}
