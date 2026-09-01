import SwiftUI

struct SimilarImagesView: View {
    @State private var groups: [SimilarImageGroup] = []
    @State private var selectedImageIDs: Set<UUID> = []
    @State private var isScanning = false
    @State private var isMoving = false
    @State private var scanMessage = ""
    @State private var showConfirm = false
    @State private var toastMessage: String?
    @State private var toastIsError = false
    @State private var leasedFolderURL: URL?
    @State private var isViewActive = false

    private var totalWasted: Int64 {
        groups.reduce(Int64(0)) { $0 + $1.wastedSize }
    }

    private var selectedSize: Int64 {
        groups.flatMap(\.images)
            .filter { selectedImageIDs.contains($0.id) }
            .reduce(Int64(0)) { $0 + $1.size }
    }

    var body: some View {
        VStack(spacing: 0) {
            header
            content
        }
        .overlay(alignment: .bottom) {
            if let msg = toastMessage {
                toastView(msg, isError: toastIsError)
            }
        }
        .alert("휴지통으로 이동하기 전 최종 확인", isPresented: $showConfirm) {
            Button("취소", role: .cancel) {}
            Button("휴지통으로 이동", role: .destructive) {
                Task { await deleteSelected() }
            }
        } message: {
            Text(
                "\(selectedImageIDs.count)개 이미지, \(formatSize(selectedSize))를 휴지통으로 이동합니다. " +
                "휴지통을 비우기 전에는 복원할 수 있으며 디스크 여유는 늘어나지 않습니다."
            )
        }
        .onAppear { isViewActive = true }
        .onDisappear {
            isViewActive = false
            releaseFolderLeaseIfInactive()
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
            .disabled(isScanning || isMoving)

            Button("선택 항목 휴지통으로 이동") { showConfirm = true }
                .buttonStyle(.borderedProminent)
                .tint(.red)
                .disabled(selectedImageIDs.isEmpty || isScanning || isMoving)
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
        guard !isScanning, !isMoving else { return }
        guard let folderURL = FileAccessManager.shared.requestFolderAccess(
            message: "유사 이미지를 검색할 폴더를 선택하세요"
        ) else { return }
        if let previous = leasedFolderURL, previous.path != folderURL.path {
            FileAccessManager.shared.releaseFolderAccess(previous)
        }
        leasedFolderURL = folderURL

        isScanning = true
        defer {
            isScanning = false
            releaseFolderLeaseIfInactive()
        }
        scanMessage = "이미지 스캔 중..."
        selectedImageIDs.removeAll()

        groups = await Task.detached {
            SimilarImageFinder.shared.scan(folderURL: folderURL) { msg, _ in
                Task { @MainActor in
                    scanMessage = msg
                }
            }
        }.value

        toastIsError = false
        if groups.isEmpty {
            toastMessage = "유사 이미지를 찾지 못했습니다"
        } else {
            toastMessage = "\(groups.count)개 유사 이미지 그룹을 발견했습니다"
        }
    }

    @MainActor
    private func deleteSelected() async {
        guard !isMoving else { return }
        let candidates = selectedImageIDs.compactMap { id -> SimilarImageTrashCandidate? in
            guard let image = groups.flatMap(\.images).first(where: { $0.id == id }) else {
                return nil
            }
            return SimilarImageTrashCandidate(
                id: image.id,
                name: image.name,
                path: image.path,
                size: image.size,
                snapshot: image.snapshot
            )
        }
        guard !candidates.isEmpty else { return }
        isMoving = true
        defer {
            isMoving = false
            releaseFolderLeaseIfInactive()
        }

        let result = await Task.detached { () -> SimilarImageTrashResult in
            var movedIDs: [UUID] = []
            var movedPaths: [String] = []
            var movedSize: Int64 = 0
            var errors: [String] = []

            for candidate in candidates {
                guard candidate.snapshot.kind == .regularFile,
                      candidate.snapshot.size == candidate.size,
                      candidate.snapshot.exactlyMatches(path: candidate.path) else {
                    errors.append("\(candidate.name): 스캔 뒤 파일이 변경되어 이동하지 않았습니다")
                    continue
                }
                let move = VerifiedFileMover.shared.moveToTrash(
                    path: candidate.path,
                    expectedSnapshot: candidate.snapshot
                )
                if move.succeeded {
                    movedIDs.append(candidate.id)
                    movedPaths.append(candidate.path)
                    movedSize += candidate.size
                } else {
                    errors.append("\(candidate.name): \(move.error ?? "휴지통으로 이동하지 못했습니다")")
                }
            }

            return SimilarImageTrashResult(
                movedIDs: movedIDs,
                movedPaths: movedPaths,
                movedSize: movedSize,
                errors: errors
            )
        }.value

        selectedImageIDs.subtract(Set(result.movedIDs))
        if result.movedSize > 0 {
            HealthMonitor.shared.recordClean()
            CleanHistory.shared.record(freed: result.movedSize, type: "manual")
        }

        if result.errors.isEmpty {
            toastIsError = false
            toastMessage = "휴지통으로 이동한 논리 용량: \(formatSize(result.movedSize)) (\(result.movedIDs.count)개 이미지)"
        } else {
            toastIsError = true
            toastMessage = "휴지통으로 이동한 논리 용량: \(formatSize(result.movedSize)) · \(result.errors.count)개 실패: \(result.errors[0])"
        }

        // 성공한 항목만 목록에서 제거하고 실패한 항목은 선택 상태로 남긴다.
        groups = groups.compactMap { group in
            let remaining = group.images.filter { img in
                !result.movedPaths.contains(img.path)
            }
            guard remaining.count >= 2 else { return nil }
            return SimilarImageGroup(images: remaining)
        }
    }

    @MainActor
    private func releaseFolderLeaseIfInactive() {
        guard !isViewActive, !isScanning, !isMoving else { return }
        if let leasedFolderURL {
            FileAccessManager.shared.releaseFolderAccess(leasedFolderURL)
            self.leasedFolderURL = nil
        }
        groups = []
        selectedImageIDs.removeAll()
    }

    // MARK: - Toast

    private func toastView(_ message: String, isError: Bool) -> some View {
        Text(message)
            .font(.callout.bold())
            .padding(.horizontal, 24)
            .padding(.vertical, 12)
            .background(isError ? Color.red : Color.green, in: Capsule())
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

private struct SimilarImageTrashCandidate: Sendable {
    let id: UUID
    let name: String
    let path: String
    let size: Int64
    let snapshot: FileIdentitySnapshot
}

private struct SimilarImageTrashResult: Sendable {
    let movedIDs: [UUID]
    let movedPaths: [String]
    let movedSize: Int64
    let errors: [String]
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
                RoundedRectangle(cornerRadius: 8)
                    .fill(.secondary.opacity(0.2))
                    .frame(width: 120, height: 120)
                    .overlay {
                        Image(systemName: "photo")
                            .font(.title)
                            .foregroundStyle(.secondary)
                    }

                // Badge
                if isOriginal {
                    Text("보관할 파일")
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
