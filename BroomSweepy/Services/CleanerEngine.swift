import Foundation
import CommonCrypto

final class CleanerEngine {
    static let shared = CleanerEngine()
    private let fileManager = FileManager.default

    /// 병렬 해싱용 concurrent queue
    private let hashQueue = DispatchQueue(label: "com.broomsweepy.hash", attributes: .concurrent)

    /// 사용자 설정 예외 경로
    var userExcludedPaths: [String] {
        let raw = UserDefaults.standard.string(forKey: "excludedPaths") ?? ""
        return raw.split(separator: "\n").map(String.init).filter { !$0.isEmpty }
    }

    /// 경로가 예외 목록에 포함되는지 확인
    private func isExcluded(_ path: String) -> Bool {
        userExcludedPaths.contains { path.hasPrefix($0) }
    }

    /// 실제 사용자 홈 디렉토리 (샌드박스 컨테이너가 아닌 진짜 경로)
    private var realHome: String {
        if let url = FileAccessManager.shared.loadBookmark() {
            return url.path
        }
        return ProcessInfo.processInfo.environment["HOME"]
            ?? ("/Users/" + NSUserName())
    }

    // MARK: - Cache Scan (병렬)

    func scanCache(homeURL: URL? = nil, progressCallback: ((String, Double) -> Void)? = nil) -> [CacheItem] {
        let home = homeURL?.path ?? realHome
        let targets: [(name: String, path: String, icon: String, desc: String, type: CacheItem.CacheType)] = [
            ("시스템 캐시", "\(home)/Library/Caches", "internaldrive", "앱별 캐시 데이터", .cache),
            ("시스템 로그", "\(home)/Library/Logs", "doc.text", "시스템 및 앱 로그", .log),
            ("Chrome 캐시", "\(home)/Library/Caches/Google/Chrome", "globe", "Chrome 브라우저 캐시", .cache),
            ("Safari 캐시", "\(home)/Library/Caches/com.apple.Safari", "safari", "Safari 브라우저 캐시", .cache),
            ("Xcode 빌드", "\(home)/Library/Developer/Xcode/DerivedData", "hammer", "Xcode 빌드 캐시", .cache),
            ("npm 캐시", "\(home)/.npm/_cacache", "shippingbox", "npm 패키지 캐시", .cache),
            ("pip 캐시", "\(home)/Library/Caches/pip", "shippingbox", "Python pip 캐시", .cache),
            ("Homebrew 캐시", "\(home)/Library/Caches/Homebrew", "shippingbox", "Homebrew 다운로드", .cache),
            ("휴지통", "\(home)/.Trash", "trash", "삭제된 파일", .cache),
            ("임시 파일", NSTemporaryDirectory(), "clock", "시스템 임시 파일", .temp),
        ]

        // 병렬 스캔: 각 타겟 디렉토리를 동시에 측정
        let lock = NSLock()
        var results: [CacheItem] = []
        var completed = 0

        DispatchQueue.concurrentPerform(iterations: targets.count) { i in
            let target = targets[i]
            guard FileManager.default.fileExists(atPath: target.path) else {
                lock.lock()
                completed += 1
                lock.unlock()
                return
            }

            let (size, count) = fastDirSize(URL(fileURLWithPath: target.path))

            lock.lock()
            completed += 1
            progressCallback?("스캔 중: \(target.name)", Double(completed) / Double(targets.count))
            if size > 0 {
                results.append(CacheItem(
                    name: target.name,
                    path: target.path,
                    icon: target.icon,
                    description: target.desc,
                    size: size,
                    fileCount: count,
                    type: target.type
                ))
            }
            lock.unlock()
        }

        return results.sorted { $0.size > $1.size }
    }

    // MARK: - Large File Scan

    func scanLargeFiles(
        scanURL: URL,
        minSizeMB: Int = 100,
        progressCallback: ((String, Double) -> Void)? = nil
    ) -> [LargeFile] {
        let minSize = Int64(minSizeMB) * 1024 * 1024
        let skipDirs: Set<String> = [
            ".git", "node_modules", ".Trash", "Library", ".npm", ".cargo",
            ".venv", "venv", "__pycache__", "site-packages",
            ".framework", "Frameworks", ".xctoolchain", "DerivedData",
            ".build", "Pods", ".app"
        ]
        let skipExtensions: Set<String> = [
            "dylib", "so", "a", "o", "pyc", "pyo", "class"
        ]
        var largeFiles: [LargeFile] = []
        var scanned = 0

        let keys: Set<URLResourceKey> = [.fileSizeKey, .contentModificationDateKey, .isDirectoryKey]
        let enumerator = fileManager.enumerator(
            at: scanURL,
            includingPropertiesForKeys: Array(keys),
            options: [.skipsHiddenFiles]
        ) { _, _ in true }

        while let url = enumerator?.nextObject() as? URL {
            let name = url.lastPathComponent
            if skipDirs.contains(name) {
                enumerator?.skipDescendants()
                continue
            }

            guard let values = try? url.resourceValues(forKeys: keys),
                  values.isDirectory == false,
                  let fileSize = values.fileSize,
                  Int64(fileSize) >= minSize else { continue }

            // 시스템/라이브러리 파일 제외
            if skipExtensions.contains(url.pathExtension.lowercased()) { continue }
            let p = url.path
            if p.contains("/Library/") || p.contains("/.Trash/") ||
               p.contains("/site-packages/") || p.contains("/.venv/") { continue }

            // 사용자 예외 경로
            if isExcluded(p) { continue }

            scanned += 1
            if scanned % 500 == 0 {
                progressCallback?("스캔 중: \(scanned)개 파일 확인", -1)
            }

            let ext = "." + url.pathExtension.lowercased()
            largeFiles.append(LargeFile(
                name: name,
                path: url.path,
                size: Int64(fileSize),
                modified: values.contentModificationDate ?? Date(),
                ext: ext,
                category: LargeFile.FileCategory.from(ext: ext)
            ))
        }

        return Array(largeFiles.sorted { $0.size > $1.size }.prefix(100))
    }

    // MARK: - Duplicate Scan (병렬 해싱)

    func scanDuplicates(
        scanURL: URL,
        minSizeKB: Int = 100,
        progressCallback: ((String, Double) -> Void)? = nil
    ) -> [DuplicateGroup] {
        let minSize = Int64(minSizeKB) * 1024
        let skipDirs: Set<String> = [
            ".git", "node_modules", ".Trash", "Library", ".npm", ".cargo",
            ".venv", "venv", "__pycache__", "site-packages",
            ".framework", "Frameworks", ".app", ".xctoolchain",
            "DerivedData", ".build", "Pods"
        ]

        // 라이브러리/시스템 파일 확장자 제외
        let skipExtensions: Set<String> = [
            "dylib", "so", "a", "o", "pyc", "pyo",
            "class", "jar", "whl", "egg"
        ]

        progressCallback?("1단계: 파일 크기 분석 중...", 0.1)

        // Step 1: Group by size
        var sizeGroups: [Int64: [URL]] = [:]

        let enumerator = fileManager.enumerator(
            at: scanURL,
            includingPropertiesForKeys: [.fileSizeKey, .isDirectoryKey],
            options: [.skipsHiddenFiles]
        ) { _, _ in true }

        while let url = enumerator?.nextObject() as? URL {
            if skipDirs.contains(url.lastPathComponent) {
                enumerator?.skipDescendants()
                continue
            }
            guard let values = try? url.resourceValues(forKeys: [.fileSizeKey, .isDirectoryKey]),
                  values.isDirectory == false,
                  let size = values.fileSize,
                  Int64(size) >= minSize else { continue }

            // 라이브러리/바이너리 파일 제외
            let ext = url.pathExtension.lowercased()
            if skipExtensions.contains(ext) { continue }

            // 사용자 예외 경로
            if isExcluded(url.path) { continue }

            // 개발 경로 안의 파일 제외
            let pathStr = url.path
            if pathStr.contains("/site-packages/") ||
               pathStr.contains("/.venv/") ||
               pathStr.contains("/venv/") ||
               pathStr.contains("/__pycache__/") ||
               pathStr.contains("/.framework/") ||
               pathStr.contains("/Frameworks/") ||
               pathStr.contains("/DerivedData/") ||
               pathStr.contains("/node_modules/") { continue }

            sizeGroups[Int64(size), default: []].append(url)
        }

        // Step 2: 병렬 해싱 — 같은 크기 파일들만 해시
        progressCallback?("2단계: 파일 해시 비교 중...", 0.5)

        let candidates = sizeGroups.filter { $0.value.count > 1 }
        let allPairs: [(url: URL, size: Int64)] = candidates.flatMap { size, urls in
            urls.map { (url: $0, size: size) }
        }

        // 병렬 해싱
        let lock = NSLock()
        var hashGroups: [String: [(url: URL, size: Int64)]] = [:]
        let batchSize = max(1, allPairs.count / max(1, ProcessInfo.processInfo.activeProcessorCount))

        DispatchQueue.concurrentPerform(iterations: allPairs.count) { i in
            let pair = allPairs[i]
            guard let hash = quickHash(pair.url) else { return }

            lock.lock()
            hashGroups[hash, default: []].append(pair)
            lock.unlock()

            if i % 100 == 0 {
                let progress = 0.5 + Double(i) / Double(allPairs.count) * 0.4
                progressCallback?("해시 비교 중: \(i)/\(allPairs.count)", progress)
            }
        }

        // Step 3: Build duplicate groups
        progressCallback?("3단계: 결과 정리 중...", 0.95)

        var duplicates: [DuplicateGroup] = []
        for (hash, items) in hashGroups where items.count > 1 {
            let files = items.map { item in
                DuplicateFile(
                    name: item.url.lastPathComponent,
                    path: item.url.path,
                    size: item.size,
                    modified: (try? item.url.resourceValues(forKeys: [.contentModificationDateKey]))?.contentModificationDate ?? Date()
                )
            }
            duplicates.append(DuplicateGroup(
                hash: String(hash.prefix(16)),
                files: files,
                eachSize: items[0].size
            ))
        }

        return Array(duplicates.sorted { $0.wastedSize > $1.wastedSize }.prefix(50))
    }

    // MARK: - Clean

    func cleanCache(items: [CacheItem]) -> (freed: Int64, errors: [String]) {
        var totalFreed: Int64 = 0
        var errors: [String] = []

        for item in items {
            do {
                let size = item.size
                let contents = try fileManager.contentsOfDirectory(atPath: item.path)
                for name in contents {
                    let fullPath = (item.path as NSString).appendingPathComponent(name)
                    try? fileManager.removeItem(atPath: fullPath)
                }
                totalFreed += size
            } catch {
                errors.append("\(item.name): \(error.localizedDescription)")
            }
        }

        return (totalFreed, errors)
    }

    func deleteFiles(paths: [String]) -> (freed: Int64, errors: [String]) {
        var totalFreed: Int64 = 0
        var errors: [String] = []

        for path in paths {
            do {
                let attrs = try fileManager.attributesOfItem(atPath: path)
                let size = (attrs[.size] as? Int64) ?? 0
                try fileManager.trashItem(at: URL(fileURLWithPath: path), resultingItemURL: nil)
                totalFreed += size
            } catch {
                let fileName = (path as NSString).lastPathComponent
                errors.append("\(fileName): 삭제 권한이 없습니다")
            }
        }

        return (totalFreed, errors)
    }

    // MARK: - Fast Directory Size (URL enumerator, pre-fetched keys)

    private func fastDirSize(_ url: URL) -> (Int64, Int) {
        var totalSize: Int64 = 0
        var count = 0

        let keys: Set<URLResourceKey> = [.fileSizeKey, .isDirectoryKey]
        guard let enumerator = FileManager.default.enumerator(
            at: url,
            includingPropertiesForKeys: Array(keys),
            options: [.skipsHiddenFiles]
        ) else { return (0, 0) }

        for case let fileURL as URL in enumerator {
            guard let values = try? fileURL.resourceValues(forKeys: keys),
                  values.isDirectory == false,
                  let size = values.fileSize else { continue }
            totalSize += Int64(size)
            count += 1
        }

        return (totalSize, count)
    }

    // MARK: - Quick Hash (head + tail + size)

    private func quickHash(_ url: URL, chunkSize: Int = 8192) -> String? {
        guard let handle = try? FileHandle(forReadingFrom: url) else { return nil }
        defer { handle.closeFile() }

        var context = CC_MD5_CTX()
        CC_MD5_Init(&context)

        let headData = handle.readData(ofLength: chunkSize)
        headData.withUnsafeBytes { ptr in
            _ = CC_MD5_Update(&context, ptr.baseAddress, CC_LONG(headData.count))
        }

        let fileSize = handle.seekToEndOfFile()
        if fileSize > UInt64(chunkSize * 2) {
            handle.seek(toFileOffset: fileSize - UInt64(chunkSize))
            let tailData = handle.readData(ofLength: chunkSize)
            tailData.withUnsafeBytes { ptr in
                _ = CC_MD5_Update(&context, ptr.baseAddress, CC_LONG(tailData.count))
            }
        }

        let sizeStr = "\(fileSize)"
        sizeStr.withCString { ptr in
            _ = CC_MD5_Update(&context, ptr, CC_LONG(sizeStr.count))
        }

        var digest = [UInt8](repeating: 0, count: Int(CC_MD5_DIGEST_LENGTH))
        CC_MD5_Final(&digest, &context)

        return digest.map { String(format: "%02x", $0) }.joined()
    }
}
