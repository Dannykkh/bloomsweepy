import SwiftUI

// MARK: - Treemap Data

struct TreemapItem: Identifiable {
    let id = UUID()
    let name: String
    let path: String
    let size: Int64
    let isDirectory: Bool
    var color: Color

    var sizeFormatted: String { formatSize(size) }

    static let palette: [Color] = [
        .blue, .orange, .green, .red, .purple, .cyan, .pink, .mint, .indigo, .teal,
        .yellow, .brown, Color(red: 0.4, green: 0.7, blue: 0.3), Color(red: 0.9, green: 0.4, blue: 0.5),
        Color(red: 0.3, green: 0.5, blue: 0.9)
    ]

    static func colorAt(_ index: Int) -> Color {
        palette[index % palette.count]
    }
}

// MARK: - Storage Treemap View

struct StorageTreemapView: View {
    @State private var items: [TreemapItem] = []
    @State private var isScanning = false
    @State private var pathStack: [URL] = []
    @State private var hoveredItem: UUID?
    @State private var scanMessage = ""

    private var totalSize: Int64 { items.reduce(0) { $0 + $1.size } }

    var body: some View {
        VStack(spacing: 0) {
            header
            if !pathStack.isEmpty { breadcrumb }

            if isScanning {
                scanningState
            } else if items.isEmpty {
                emptyState
            } else {
                // 요약 바
                summaryBar

                // 트리맵 + 순위 리스트 (좌우 분할)
                HSplitView {
                    treemapArea
                        .frame(minWidth: 300)
                    rankingList
                        .frame(minWidth: 220, maxWidth: 280)
                }
            }
        }
    }

    // MARK: - Header

    private var header: some View {
        HStack(spacing: 12) {
            Text("저장공간 맵")
                .font(.title2.bold())
            Spacer()
            if !pathStack.isEmpty {
                Button { goBack() } label: {
                    Label("상위 폴더", systemImage: "chevron.left")
                }
                .buttonStyle(.bordered)
            }
            Button("폴더 선택 후 스캔") { Task { await scanFolder() } }
                .buttonStyle(.borderedProminent)
                .disabled(isScanning)
        }
        .padding(24)
    }

    // MARK: - Breadcrumb

    private var breadcrumb: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 4) {
                ForEach(Array(pathStack.enumerated()), id: \.offset) { i, url in
                    if i > 0 {
                        Image(systemName: "chevron.right")
                            .font(.caption2)
                            .foregroundStyle(.tertiary)
                    }
                    Button(url.lastPathComponent) { navigateTo(index: i) }
                        .font(.caption)
                        .foregroundStyle(i == pathStack.count - 1 ? .primary : .secondary)
                        .buttonStyle(.plain)
                }
            }
            .padding(.horizontal, 24)
            .padding(.bottom, 6)
        }
    }

    // MARK: - Summary Bar

    private var summaryBar: some View {
        HStack(spacing: 20) {
            Label(formatSize(totalSize), systemImage: "internaldrive.fill")
                .font(.callout.bold())
            Text("·")
                .foregroundStyle(.tertiary)
            Text("\(items.count)개 항목")
                .font(.callout)
                .foregroundStyle(.secondary)

            if let biggest = items.first {
                Text("·")
                    .foregroundStyle(.tertiary)
                Text("최대: \(biggest.name) (\(biggest.sizeFormatted))")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }
            Spacer()
        }
        .padding(.horizontal, 24)
        .padding(.vertical, 8)
        .background(.bar)
    }

    // MARK: - Treemap Area

    private var treemapArea: some View {
        GeometryReader { geo in
            let sorted = Array(items.sorted { $0.size > $1.size }.prefix(15))
            let total = sorted.reduce(Int64(0)) { $0 + $1.size }
            let rects = sliceLayout(items: sorted, totalSize: total, container: CGRect(origin: .zero, size: geo.size))

            ZStack(alignment: .topLeading) {
                ForEach(Array(zip(sorted, rects).enumerated()), id: \.element.0.id) { index, pair in
                    let (item, rect) = pair
                    let pct = total > 0 ? Int(Double(item.size) / Double(total) * 100) : 0

                    TreemapCellView(
                        item: item, index: index, pct: pct, rect: rect,
                        isHovered: hoveredItem == item.id,
                        onHover: { h in
                            withAnimation(.easeInOut(duration: 0.12)) { hoveredItem = h ? item.id : nil }
                        },
                        onTap: {
                            if item.isDirectory { drillDown(to: item) }
                            else { revealInFinder(item.path) }
                        },
                        onDrillDown: { drillDown(to: item) },
                        onReveal: { revealInFinder(item.path) },
                        onTrash: { trashItem(item) }
                    )
                }
            }
        }
        .padding(12)
    }

    private func cellFontSize(_ rect: CGRect, base: CGFloat) -> CGFloat {
        max(8, min(base, min(rect.width, rect.height) / 6))
    }
}

// MARK: - Treemap Cell View (정확한 클릭 영역)

private struct TreemapCellView: View {
    let item: TreemapItem
    let index: Int
    let pct: Int
    let rect: CGRect
    let isHovered: Bool
    let onHover: (Bool) -> Void
    let onTap: () -> Void
    let onDrillDown: () -> Void
    let onReveal: () -> Void
    let onTrash: () -> Void

    private func fontSize(_ base: CGFloat) -> CGFloat {
        max(8, min(base, min(rect.width, rect.height) / 6))
    }

    var body: some View {
        RoundedRectangle(cornerRadius: 6)
            .fill(TreemapItem.colorAt(index).gradient.opacity(isHovered ? 1.0 : 0.75))
            .overlay {
                VStack(spacing: 2) {
                    if rect.height > 30 {
                        Image(systemName: item.isDirectory ? "folder.fill" : "doc.fill")
                            .font(.system(size: fontSize(16)))
                    }
                    if rect.width > 45 {
                        Text(item.name)
                            .font(.system(size: fontSize(12), weight: .semibold))
                            .lineLimit(1)
                            .truncationMode(.middle)
                    }
                    if rect.width > 35 && rect.height > 35 {
                        Text("\(pct)%")
                            .font(.system(size: fontSize(11), weight: .bold, design: .rounded))
                            .opacity(0.9)
                    }
                    if rect.width > 55 && rect.height > 50 {
                        Text(item.sizeFormatted)
                            .font(.system(size: fontSize(10)))
                            .opacity(0.7)
                    }
                }
                .foregroundStyle(.white)
                .padding(3)
            }
            .overlay(
                RoundedRectangle(cornerRadius: 6)
                    .stroke(.white.opacity(isHovered ? 0.6 : 0.1), lineWidth: isHovered ? 2 : 0.5)
            )
            .frame(width: max(1, rect.width), height: max(1, rect.height))
            .contentShape(Rectangle())
            .onHover(perform: onHover)
            .onTapGesture(perform: onTap)
            .contextMenu {
                if item.isDirectory {
                    Button { onDrillDown() } label: {
                        Label("하위 폴더 탐색", systemImage: "folder.badge.magnifyingglass")
                    }
                }
                Button { onReveal() } label: {
                    Label("Finder에서 보기", systemImage: "magnifyingglass")
                }
                Divider()
                Button(role: .destructive) { onTrash() } label: {
                    Label("휴지통으로 이동", systemImage: "trash")
                }
            }
            .offset(x: rect.minX, y: rect.minY)
    }
}

// MARK: - StorageTreemapView extensions

private extension StorageTreemapView {

    // MARK: - Ranking List (오른쪽 패널)

    private var rankingList: some View {
        VStack(spacing: 0) {
            HStack {
                Text("크기 순위")
                    .font(.headline)
                Spacer()
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 10)
            .background(.bar)

            ScrollView {
                VStack(spacing: 2) {
                    let sorted = items.sorted { $0.size > $1.size }
                    ForEach(Array(sorted.enumerated()), id: \.element.id) { index, item in
                        HStack(spacing: 10) {
                            Circle()
                                .fill(TreemapItem.colorAt(index))
                                .frame(width: 10, height: 10)

                            Image(systemName: item.isDirectory ? "folder.fill" : "doc.fill")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                                .frame(width: 14)

                            Text(item.name)
                                .font(.system(size: 12))
                                .lineLimit(1)
                                .truncationMode(.middle)

                            Spacer(minLength: 4)

                            let pct = totalSize > 0 ? Int(Double(item.size) / Double(totalSize) * 100) : 0
                            Text("\(pct)%")
                                .font(.system(size: 10, weight: .bold, design: .rounded))
                                .foregroundStyle(.secondary)
                                .frame(width: 28, alignment: .trailing)

                            Text(item.sizeFormatted)
                                .font(.system(size: 11, weight: .medium, design: .rounded))
                                .monospacedDigit()
                                .frame(width: 60, alignment: .trailing)
                        }
                        .padding(.horizontal, 16)
                        .padding(.vertical, 6)
                        .background(hoveredItem == item.id ? TreemapItem.colorAt(index).opacity(0.1) : .clear)
                        .contentShape(Rectangle())
                        .onHover { h in
                            withAnimation(.easeInOut(duration: 0.12)) { hoveredItem = h ? item.id : nil }
                        }
                        .onTapGesture {
                            if item.isDirectory { drillDown(to: item) }
                            else { revealInFinder(item.path) }
                        }
                        .contextMenu {
                            if item.isDirectory {
                                Button { drillDown(to: item) } label: {
                                    Label("하위 폴더 탐색", systemImage: "folder.badge.magnifyingglass")
                                }
                            }
                            Button { revealInFinder(item.path) } label: {
                                Label("Finder에서 보기", systemImage: "magnifyingglass")
                            }
                            Divider()
                            Button(role: .destructive) { trashItem(item) } label: {
                                Label("휴지통으로 이동", systemImage: "trash")
                            }
                        }
                    }
                }
                .padding(.vertical, 6)
            }
        }
    }

    // MARK: - Slice Layout

    private func sliceLayout(items: [TreemapItem], totalSize: Int64, container: CGRect) -> [CGRect] {
        guard !items.isEmpty, totalSize > 0 else { return [] }
        var rects: [CGRect] = []
        var remaining = container
        let gap: CGFloat = 3

        for (i, item) in items.enumerated() {
            let ratio = CGFloat(item.size) / CGFloat(totalSize)

            if i == items.count - 1 {
                rects.append(remaining.insetBy(dx: gap / 2, dy: gap / 2))
            } else if remaining.width >= remaining.height {
                let w = remaining.width * ratio
                rects.append(CGRect(x: remaining.minX + gap / 2, y: remaining.minY + gap / 2,
                                    width: max(1, w - gap), height: max(1, remaining.height - gap)))
                remaining = CGRect(x: remaining.minX + w, y: remaining.minY,
                                   width: remaining.width - w, height: remaining.height)
            } else {
                let h = remaining.height * ratio
                rects.append(CGRect(x: remaining.minX + gap / 2, y: remaining.minY + gap / 2,
                                    width: max(1, remaining.width - gap), height: max(1, h - gap)))
                remaining = CGRect(x: remaining.minX, y: remaining.minY + h,
                                   width: remaining.width, height: remaining.height - h)
            }
        }
        return rects
    }

    // MARK: - States

    private var emptyState: some View {
        Button {
            Task { await scanFolder() }
        } label: {
            VStack(spacing: 16) {
                ZStack {
                    Circle()
                        .fill(.blue.opacity(0.1))
                        .frame(width: 88, height: 88)
                    Image(systemName: "square.grid.3x3.topleft.filled")
                        .font(.system(size: 36))
                        .foregroundStyle(.blue)
                }
                VStack(spacing: 8) {
                    Text("클릭하여 저장공간 분석")
                        .font(.title3.bold())
                    Text("폴더를 선택하면 파일/폴더 크기를\n색상 블록으로 한눈에 보여드립니다")
                        .font(.callout)
                        .foregroundStyle(.secondary)
                        .multilineTextAlignment(.center)
                }
            }
        }
        .buttonStyle(.plain)
        .onHover { h in
            if h { NSCursor.pointingHand.push() } else { NSCursor.pop() }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var scanningState: some View {
        VStack(spacing: 16) {
            ProgressView().scaleEffect(1.5)
            Text(scanMessage.isEmpty ? "폴더 분석 중..." : scanMessage)
                .font(.headline)
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    // MARK: - Actions

    @MainActor
    private func scanFolder() async {
        guard let url = FileAccessManager.shared.requestFolderAccess(message: "분석할 폴더를 선택하세요") else { return }
        isScanning = true
        scanMessage = "폴더 구조 분석 중..."
        pathStack = [url]
        items = await Task.detached { Self.scanDir(url: url) }.value
        isScanning = false
    }

    private func drillDown(to item: TreemapItem) {
        let url = URL(fileURLWithPath: item.path)
        pathStack.append(url)
        Task {
            isScanning = true
            scanMessage = "\(item.name) 분석 중..."
            items = await Task.detached { Self.scanDir(url: url) }.value
            withAnimation(.spring(duration: 0.3)) { isScanning = false }
        }
    }

    private func goBack() {
        guard pathStack.count > 1 else { return }
        pathStack.removeLast()
        guard let p = pathStack.last else { return }
        Task {
            isScanning = true
            items = await Task.detached { Self.scanDir(url: p) }.value
            isScanning = false
        }
    }

    private func navigateTo(index: Int) {
        pathStack = Array(pathStack.prefix(index + 1))
        guard let t = pathStack.last else { return }
        Task {
            isScanning = true
            items = await Task.detached { Self.scanDir(url: t) }.value
            isScanning = false
        }
    }

    // MARK: - Item Actions

    private func revealInFinder(_ path: String) {
        NSWorkspace.shared.activateFileViewerSelecting([URL(fileURLWithPath: path)])
    }

    private func trashItem(_ item: TreemapItem) {
        Task {
            let url = URL(fileURLWithPath: item.path)
            do {
                try FileManager.default.trashItem(at: url, resultingItemURL: nil)
                withAnimation(.spring(duration: 0.3)) {
                    items.removeAll { $0.id == item.id }
                }
            } catch {
                // 권한 부족
            }
        }
    }

    // MARK: - Scanner

    private static func scanDir(url: URL) -> [TreemapItem] {
        let fm = FileManager.default
        guard let contents = try? fm.contentsOfDirectory(
            at: url, includingPropertiesForKeys: [.fileSizeKey, .isDirectoryKey],
            options: [.skipsHiddenFiles]
        ) else { return [] }

        var result: [TreemapItem] = []
        for itemURL in contents {
            guard let v = try? itemURL.resourceValues(forKeys: [.isDirectoryKey, .fileSizeKey]) else { continue }
            let isDir = v.isDirectory ?? false
            let name = itemURL.lastPathComponent

            if isDir {
                let size = dirSize(itemURL)
                guard size > 0 else { continue }
                result.append(TreemapItem(name: name, path: itemURL.path, size: size,
                                          isDirectory: true, color: .blue))
            } else {
                let size = Int64(v.fileSize ?? 0)
                guard size > 10_000 else { continue }
                result.append(TreemapItem(name: name, path: itemURL.path, size: size,
                                          isDirectory: false, color: .blue))
            }
        }
        return result.sorted { $0.size > $1.size }
    }

    private static func dirSize(_ url: URL) -> Int64 {
        var total: Int64 = 0
        guard let e = FileManager.default.enumerator(
            at: url, includingPropertiesForKeys: [.fileSizeKey, .isDirectoryKey],
            options: [.skipsHiddenFiles]
        ) else { return 0 }
        for case let f as URL in e {
            guard let v = try? f.resourceValues(forKeys: [.fileSizeKey, .isDirectoryKey]),
                  v.isDirectory == false, let s = v.fileSize else { continue }
            total += Int64(s)
        }
        return total
    }
}
