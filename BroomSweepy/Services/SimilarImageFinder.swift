import Foundation
import CoreImage

// MARK: - Models

struct SimilarImageGroup: Identifiable, Sendable {
    let id = UUID()
    let images: [SimilarImage]
    var wastedSize: Int64 { images.dropFirst().reduce(Int64(0)) { $0 + $1.size } }
    var wastedSizeFormatted: String { formatSize(wastedSize) }
}

struct SimilarImage: Identifiable, Hashable, Sendable {
    let id = UUID()
    let path: String
    let name: String
    let size: Int64
    let snapshot: FileIdentitySnapshot

    var sizeFormatted: String { formatSize(size) }

    func hash(into hasher: inout Hasher) {
        hasher.combine(id)
    }

    static func == (lhs: SimilarImage, rhs: SimilarImage) -> Bool {
        lhs.id == rhs.id
    }
}

// MARK: - Finder

final class SimilarImageFinder {
    static let shared = SimilarImageFinder()
    private let fm = FileManager.default

    /// CIContext 재사용 (생성 비용이 높으므로 한 번만 생성)
    private lazy var ciContext = CIContext(options: [.useSoftwareRenderer: false])

    private let imageExtensions: Set<String> = ["jpg", "jpeg", "png", "heic", "tiff", "bmp", "gif"]

    func scan(folderURL: URL, threshold: Int = 10,
              progressCallback: ((String, Double) -> Void)? = nil) -> [SimilarImageGroup] {

        progressCallback?("이미지 파일 검색 중...", 0.0)

        let rootPath = folderURL.standardizedFileURL.path
        let trashPath = actualUserHomeURL().appendingPathComponent(".Trash").standardizedFileURL.path
        guard !isSameOrDescendant(rootPath, of: trashPath),
              let rootSnapshot = FileIdentitySnapshot.capture(path: rootPath),
              rootSnapshot.kind == .directory else { return [] }

        // 1. Find all image files
        var imageURLs: [URL] = []
        let enumerator = fm.enumerator(
            at: URL(fileURLWithPath: rootPath),
            includingPropertiesForKeys: [.fileSizeKey, .isDirectoryKey],
            options: [.skipsHiddenFiles]
        ) { _, _ in true }

        let skipDirs: Set<String> = [".git", "node_modules", ".Trash", "Library", ".npm"]

        while let url = enumerator?.nextObject() as? URL {
            let normalized = url.standardizedFileURL.path
            guard rootSnapshot.exactlyMatches(path: rootPath),
                  isSameOrDescendant(normalized, of: rootPath) else { return [] }
            if skipDirs.contains(url.lastPathComponent) {
                enumerator?.skipDescendants()
                continue
            }
            guard let snapshot = FileIdentitySnapshot.capture(path: url.path),
                  snapshot.kind == .regularFile,
                  let values = try? url.resourceValues(forKeys: [.isDirectoryKey]),
                  values.isDirectory == false else { continue }

            if imageExtensions.contains(url.pathExtension.lowercased()) {
                imageURLs.append(url)
            }
        }

        guard imageURLs.count >= 2 else { return [] }

        progressCallback?("\(imageURLs.count)개 이미지 해시 계산 중...", 0.2)

        // 2. Compute perceptual hash (autoreleasepool로 메모리 즉시 회수)
        var hashes: [(url: URL, hash: UInt64, size: Int64, snapshot: FileIdentitySnapshot)] = []
        let total = Double(imageURLs.count)

        for (i, url) in imageURLs.enumerated() {
            autoreleasepool {
                if i % 50 == 0 {
                    progressCallback?("해시 계산 중... \(i)/\(imageURLs.count)", 0.2 + 0.5 * Double(i) / total)
                }

                guard let before = FileIdentitySnapshot.capture(path: url.path),
                      before.kind == .regularFile,
                      let hash = averageHash(for: url),
                      before.exactlyMatches(path: url.path) else { return }
                let fileSize = (try? url.resourceValues(forKeys: [.fileSizeKey]))?.fileSize ?? 0
                guard before.size == Int64(fileSize) else { return }
                hashes.append((url, hash, Int64(fileSize), before))
            }
        }

        progressCallback?("유사 이미지 비교 중...", 0.7)

        // 3. Compare hamming distances and group
        var visited = Set<Int>()
        var groups: [SimilarImageGroup] = []

        for i in 0..<hashes.count {
            guard !visited.contains(i) else { continue }
            var groupIndices = [i]

            for j in (i + 1)..<hashes.count {
                guard !visited.contains(j) else { continue }
                let distance = hammingDistance(hashes[i].hash, hashes[j].hash)
                if distance <= threshold {
                    groupIndices.append(j)
                }
            }

            guard groupIndices.count >= 2 else { continue }
            visited.formUnion(groupIndices)

            let images = groupIndices.map { idx -> SimilarImage in
                let item = hashes[idx]
                return SimilarImage(
                    path: item.url.path,
                    name: item.url.lastPathComponent,
                    size: item.size,
                    snapshot: item.snapshot
                )
            }.sorted { $0.path < $1.path }

            groups.append(SimilarImageGroup(images: images))
        }

        progressCallback?("완료!", 1.0)
        return groups.sorted { $0.wastedSize > $1.wastedSize }
    }

    // MARK: - Average Hash (aHash) — CIContext 재사용

    private func averageHash(for url: URL) -> UInt64? {
        guard let ciImage = CIImage(contentsOf: url) else { return nil }

        // Resize to 8x8
        let scaleX = 8.0 / ciImage.extent.width
        let scaleY = 8.0 / ciImage.extent.height
        let scaled = ciImage.transformed(by: CGAffineTransform(scaleX: scaleX, y: scaleY))

        // Grayscale
        let grayscale = scaled.applyingFilter("CIColorMonochrome", parameters: [
            "inputColor": CIColor(red: 0.7, green: 0.7, blue: 0.7),
            "inputIntensity": 1.0
        ])

        // Render (재사용된 CIContext)
        let extent = CGRect(x: 0, y: 0, width: 8, height: 8)
        guard let cgImage = ciContext.createCGImage(grayscale, from: extent) else { return nil }

        guard let data = cgImage.dataProvider?.data,
              let ptr = CFDataGetBytePtr(data) else { return nil }

        let bytesPerPixel = cgImage.bitsPerPixel / 8
        let bytesPerRow = cgImage.bytesPerRow
        var pixels: [Double] = []
        pixels.reserveCapacity(64)

        for y in 0..<8 {
            for x in 0..<8 {
                let offset = y * bytesPerRow + x * bytesPerPixel
                pixels.append(Double(ptr[offset]))
            }
        }

        guard pixels.count == 64 else { return nil }

        let avg = pixels.reduce(0, +) / 64.0
        var hash: UInt64 = 0
        for (i, pixel) in pixels.enumerated() {
            if pixel > avg { hash |= (1 << i) }
        }

        return hash
    }

    // MARK: - Hamming Distance

    private func hammingDistance(_ a: UInt64, _ b: UInt64) -> Int {
        (a ^ b).nonzeroBitCount
    }

    private func isSameOrDescendant(_ path: String, of root: String) -> Bool {
        path == root || path.hasPrefix(root.hasSuffix("/") ? root : root + "/")
    }

}
