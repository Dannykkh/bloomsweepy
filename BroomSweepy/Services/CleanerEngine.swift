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
        actualUserHomeURL().path
    }

    // MARK: - Cache Scan (병렬)

    func scanCache(
        homeURL: URL? = nil,
        progressCallback: ((String, Double) -> Void)? = nil,
        shouldCancel: @escaping () -> Bool = { false }
    ) -> [CacheItem] {
        let home = homeURL?.path ?? realHome
        let rawTargets: [(name: String, path: String, icon: String, desc: String, type: CacheItem.CacheType)] = [
            ("시스템 캐시", "\(home)/Library/Caches", "internaldrive", "앱별 캐시 데이터", .cache),
            ("시스템 로그", "\(home)/Library/Logs", "doc.text", "시스템 및 앱 로그", .log),
            ("Chrome 캐시", "\(home)/Library/Caches/Google/Chrome", "globe", "Chrome 브라우저 캐시", .cache),
            ("Safari 캐시", "\(home)/Library/Caches/com.apple.Safari", "safari", "Safari 브라우저 캐시", .cache),
            ("Xcode 빌드", "\(home)/Library/Developer/Xcode/DerivedData", "hammer", "Xcode 빌드 캐시", .cache),
            ("npm 캐시", "\(home)/.npm/_cacache", "shippingbox", "npm 패키지 캐시", .cache),
            ("pip 캐시", "\(home)/Library/Caches/pip", "shippingbox", "Python pip 캐시", .cache),
            ("Homebrew 캐시", "\(home)/Library/Caches/Homebrew", "shippingbox", "Homebrew 다운로드", .cache),
            ("임시 파일", NSTemporaryDirectory(), "clock", "시스템 임시 파일", .temp),
        ]

        // 상위 캐시 경로와 그 하위 경로를 동시에 집계하면 같은 파일이 여러 번
        // 표시된다. 더 얕은 경로를 우선해 서로 겹치지 않는 후보만 스캔한다.
        let targets = nonOverlappingTargets(rawTargets)

        // 병렬 스캔: 각 타겟 디렉토리를 동시에 측정
        let lock = NSLock()
        var results: [CacheItem] = []
        var completed = 0

        DispatchQueue.concurrentPerform(iterations: targets.count) { i in
            guard !shouldCancel() else { return }
            let target = targets[i]
            guard let snapshot = FileIdentitySnapshot.capture(path: target.path),
                  snapshot.kind == .directory else {
                lock.lock()
                completed += 1
                lock.unlock()
                return
            }

            let (size, count) = fastDirSize(
                URL(fileURLWithPath: target.path),
                shouldCancel: shouldCancel
            )
            guard !shouldCancel() else { return }

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
                    type: target.type,
                    snapshot: snapshot
                ))
            }
            lock.unlock()
        }

        guard !shouldCancel() else { return [] }
        return results.sorted { $0.size > $1.size }
    }

    // MARK: - Large File Scan

    func scanLargeFiles(
        scanURL: URL,
        minSizeMB: Int = 100,
        progressCallback: ((String, Double) -> Void)? = nil,
        shouldCancel: @escaping () -> Bool = { false }
    ) -> [LargeFile] {
        let rootPath = normalizedPath(scanURL.path)
        guard let rootSnapshot = FileIdentitySnapshot.capture(path: rootPath),
              rootSnapshot.kind == .directory else { return [] }
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
            guard !shouldCancel() else { return [] }
            guard rootSnapshot.exactlyMatches(path: rootPath),
                  isSameOrDescendant(normalizedPath(url.path), of: rootPath) else { return [] }
            let name = url.lastPathComponent
            if skipDirs.contains(name) {
                enumerator?.skipDescendants()
                continue
            }

            guard let snapshot = FileIdentitySnapshot.capture(path: url.path),
                  snapshot.kind == .regularFile,
                  let values = try? url.resourceValues(forKeys: keys),
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
                category: LargeFile.FileCategory.from(ext: ext),
                snapshot: snapshot
            ))
        }

        guard !shouldCancel() else { return [] }
        return Array(largeFiles.sorted { $0.size > $1.size }.prefix(100))
    }

    // MARK: - Duplicate Scan (병렬 해싱)

    func scanDuplicates(
        scanURL: URL,
        minSizeKB: Int = 100,
        progressCallback: ((String, Double) -> Void)? = nil,
        shouldCancel: @escaping () -> Bool = { false }
    ) -> [DuplicateGroup] {
        let rootPath = normalizedPath(scanURL.path)
        guard let rootSnapshot = FileIdentitySnapshot.capture(path: rootPath),
              rootSnapshot.kind == .directory else { return [] }
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
            guard !shouldCancel() else { return [] }
            guard rootSnapshot.exactlyMatches(path: rootPath),
                  isSameOrDescendant(normalizedPath(url.path), of: rootPath) else { return [] }
            if skipDirs.contains(url.lastPathComponent) {
                enumerator?.skipDescendants()
                continue
            }
            guard let snapshot = FileIdentitySnapshot.capture(path: url.path),
                  snapshot.kind == .regularFile,
                  let values = try? url.resourceValues(forKeys: [.fileSizeKey, .isDirectoryKey]),
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

            guard snapshot.size == Int64(size) else { continue }
            sizeGroups[Int64(size), default: []].append(url)
        }

        // Step 2: 병렬 해싱 — 같은 크기 파일들만 해시
        progressCallback?("2단계: 파일 해시 비교 중...", 0.5)

        let candidates = sizeGroups.filter { $0.value.count > 1 }
        var allPairs: [(url: URL, size: Int64, modified: Date?)] = []
        for (size, urls) in candidates {
            guard !shouldCancel() else { return [] }
            for url in urls {
                guard !shouldCancel() else { return [] }
                let modified = (try? url.resourceValues(
                    forKeys: [.contentModificationDateKey]
                ))?.contentModificationDate
                allPairs.append((url: url, size: size, modified: modified))
            }
        }

        // 병렬 해싱
        let lock = NSLock()
        var hashGroups: [String: [(url: URL, size: Int64, modified: Date?)]] = [:]

        DispatchQueue.concurrentPerform(iterations: allPairs.count) { i in
            guard !shouldCancel() else { return }
            let pair = allPairs[i]
            guard let hash = quickHash(pair.url, shouldCancel: shouldCancel) else { return }

            lock.lock()
            hashGroups[hash, default: []].append(pair)
            lock.unlock()

            if i % 100 == 0 {
                let progress = 0.5 + Double(i) / Double(allPairs.count) * 0.2
                progressCallback?("빠른 해시 비교 중: \(i)/\(allPairs.count)", progress)
            }
        }

        guard !shouldCancel() else { return [] }

        // 부분 해시는 후보 필터일 뿐이다. 파일 전체를 스트리밍 SHA-256으로
        // 다시 검증해 실제 내용이 같은 파일만 확정 그룹으로 만든다.
        var verificationPairs: [(url: URL, size: Int64, modified: Date?)] = []
        for items in hashGroups.values where items.count > 1 {
            guard !shouldCancel() else { return [] }
            for item in items {
                guard !shouldCancel() else { return [] }
                verificationPairs.append(item)
            }
        }
        var fullHashGroups: [String: [(url: URL, size: Int64, modified: Date?)]] = [:]

        for (index, pair) in verificationPairs.enumerated() {
            guard !shouldCancel() else { return [] }
            let hash = fullHash(
                pair.url,
                expectedSize: pair.size,
                expectedModificationDate: pair.modified,
                shouldCancel: shouldCancel
            )
            guard !shouldCancel() else { return [] }
            if let hash {
                fullHashGroups[hash, default: []].append(pair)
            }

            if index % 10 == 0 || index == verificationPairs.count - 1 {
                let ratio = verificationPairs.isEmpty
                    ? 1.0
                    : Double(index + 1) / Double(verificationPairs.count)
                progressCallback?(
                    "전체 내용 검증 중: \(index + 1)/\(verificationPairs.count)",
                    0.7 + ratio * 0.25
                )
            }
        }

        guard !shouldCancel() else { return [] }

        // Step 3: Build duplicate groups
        progressCallback?("3단계: 결과 정리 중...", 0.98)

        var duplicates: [DuplicateGroup] = []
        for (hash, items) in fullHashGroups where items.count > 1 {
            guard !shouldCancel() else { return [] }
            let files = items.sorted { $0.url.path < $1.url.path }.compactMap { item -> DuplicateFile? in
                guard let snapshot = FileIdentitySnapshot.capture(path: item.url.path),
                      snapshot.kind == .regularFile,
                      snapshot.size == item.size else { return nil }
                return DuplicateFile(
                    name: item.url.lastPathComponent,
                    path: item.url.path,
                    size: item.size,
                    modified: item.modified ?? Date(),
                    snapshot: snapshot
                )
            }
            guard files.count > 1 else { continue }
            duplicates.append(DuplicateGroup(
                hash: hash,
                files: files,
                eachSize: items[0].size
            ))
        }

        guard !shouldCancel() else { return [] }
        return Array(duplicates.sorted { $0.wastedSize > $1.wastedSize }.prefix(50))
    }

    // MARK: - Clean

    func cleanCache(
        items: [CacheItem],
        shouldCancel: () -> Bool = { false }
    ) -> (freed: Int64, errors: [String]) {
        var totalFreed: Int64 = 0
        var errors: [String] = []

        let approvedHomePath = normalizedPath(realHome)
        let approvedTemporaryPath = normalizedPath(NSTemporaryDirectory())
        let approvedTemporaryResolvedPath = URL(fileURLWithPath: approvedTemporaryPath)
            .resolvingSymlinksInPath().standardizedFileURL.path
        let trashPath = normalizedPath((realHome as NSString).appendingPathComponent(".Trash"))
        var acceptedRoots: [String] = []
        let candidates = items.sorted {
            pathDepth($0.path) < pathDepth($1.path)
        }

        for item in candidates {
            guard !shouldCancel() else {
                errors.append("현재 항목 처리 후 중단했습니다")
                break
            }
            let itemPath = normalizedPath(item.path)
            let resolvedItemPath = URL(fileURLWithPath: itemPath)
                .resolvingSymlinksInPath().standardizedFileURL.path
            guard (isSameOrDescendant(itemPath, of: approvedHomePath)
                    && isSameOrDescendant(resolvedItemPath, of: approvedHomePath))
                    || (isSameOrDescendant(itemPath, of: approvedTemporaryPath)
                        && isSameOrDescendant(resolvedItemPath, of: approvedTemporaryResolvedPath)) else {
                errors.append("\(item.name): 승인된 홈 또는 임시 폴더 밖의 항목은 이동하지 않았습니다")
                continue
            }
            if isSameOrDescendant(itemPath, of: trashPath) {
                errors.append("\(item.name): 휴지통은 정리 후보에서 제외됩니다")
                continue
            }
            if acceptedRoots.contains(where: { isSameOrDescendant(itemPath, of: $0) }) {
                continue
            }
            acceptedRoots.append(itemPath)

            guard item.snapshot.exactlyMatches(path: itemPath) else {
                errors.append("\(item.name): 스캔 뒤 항목이 변경되어 이동하지 않았습니다")
                continue
            }

            if item.snapshot.kind == .directory {
                errors.append(
                    "\(item.name): 폴더 내부 전체를 검토 당시와 동일하다고 증명할 수 없어 " +
                    "자동 이동하지 않았습니다. Finder에서 검토해 주세요"
                )
            } else {
                guard item.snapshot.exactlyMatches(path: itemPath) else {
                    errors.append("\(item.name): 최종 확인 중 변경되어 이동하지 않았습니다")
                    continue
                }
                let result = VerifiedFileMover.shared.moveToTrash(
                    path: itemPath,
                    expectedSnapshot: item.snapshot,
                    shouldCancel: shouldCancel
                )
                if result.succeeded {
                    totalFreed += item.snapshot.size
                } else {
                    errors.append("\(item.name): \(result.error ?? "휴지통으로 이동하지 못했습니다")")
                }
            }
        }

        return (totalFreed, errors)
    }

    @available(*, unavailable, message: "스캔 스냅샷이 있는 typed trash API를 사용하세요")
    func deleteFiles(paths: [String]) -> (freed: Int64, errors: [String]) {
        let errors = paths.map {
            "\(($0 as NSString).lastPathComponent): 검토 당시 파일 정보가 없어 자동으로 이동하지 않았습니다"
        }
        return (0, errors)
    }

    func trashVerifiedLargeFiles(
        files: [LargeFile],
        shouldCancel: () -> Bool = { false }
    ) -> (freed: Int64, errors: [String]) {
        var totalFreed: Int64 = 0
        var errors: [String] = []
        let trashPath = normalizedPath((realHome as NSString).appendingPathComponent(".Trash"))

        for file in files.sorted(by: { $0.path < $1.path }) {
            guard !shouldCancel() else {
                errors.append("현재 항목 처리 후 중단했습니다")
                break
            }
            let path = normalizedPath(file.path)
            guard !isSameOrDescendant(path, of: trashPath) else {
                errors.append("\(file.name): 이미 휴지통 안에 있습니다")
                continue
            }
            guard file.snapshot.kind == .regularFile,
                  file.snapshot.exactlyMatches(path: path),
                  file.snapshot.size == file.size else {
                errors.append("\(file.name): 스캔 뒤 파일이 변경되어 이동하지 않았습니다")
                continue
            }
            let result = VerifiedFileMover.shared.moveToTrash(
                path: path,
                expectedSnapshot: file.snapshot,
                shouldCancel: shouldCancel
            )
            if result.succeeded {
                totalFreed += file.size
            } else {
                errors.append("\(file.name): \(result.error ?? "휴지통으로 이동하지 못했습니다")")
            }
        }
        return (totalFreed, errors)
    }

    /// 스캔 당시 중복 그룹의 보관할 파일과 선택 복사본을 파일 전체 해시, 크기,
    /// 수정 시간으로 다시 확인한 뒤 복사본만 휴지통으로 이동한다.
    /// 보관할 파일이 사라졌거나 바뀐 그룹은 어떤 복사본도 이동하지 않는다.
    func trashVerifiedDuplicates(
        groups: [DuplicateGroup],
        selectedFileIDs: Set<UUID>,
        shouldCancel: () -> Bool = { false }
    ) -> (freed: Int64, errors: [String]) {
        guard !selectedFileIDs.isEmpty else { return (0, []) }

        var totalFreed: Int64 = 0
        var errors: [String] = []
        var matchedIDs: Set<UUID> = []
        let trashPath = normalizedPath((realHome as NSString).appendingPathComponent(".Trash"))

        for group in groups {
            guard !shouldCancel() else {
                errors.append("현재 항목 처리 후 중단했습니다")
                break
            }
            let orderedFiles = group.files.sorted { $0.path < $1.path }
            guard let keeper = orderedFiles.first else { continue }

            if selectedFileIDs.contains(keeper.id) {
                matchedIDs.insert(keeper.id)
                errors.append("\(keeper.name): 보관할 파일은 휴지통으로 이동하지 않았습니다")
            }

            let selectedCopies = orderedFiles.dropFirst().filter {
                selectedFileIDs.contains($0.id)
            }
            guard !selectedCopies.isEmpty else { continue }
            matchedIDs.formUnion(selectedCopies.map(\.id))

            let keeperPath = normalizedPath(keeper.path)
            guard !isSameOrDescendant(keeperPath, of: trashPath) else {
                errors.append(contentsOf: selectedCopies.map {
                    "\($0.name): 보관할 파일이 휴지통 안에 있어 이동하지 않았습니다"
                })
                continue
            }

            for copy in selectedCopies {
                guard !shouldCancel() else {
                    errors.append("현재 항목 처리 후 중단했습니다")
                    break
                }
                let copyPath = normalizedPath(copy.path)
                guard copyPath != keeperPath else {
                    errors.append("\(copy.name): 보관할 파일과 경로가 같아 이동하지 않았습니다")
                    continue
                }
                guard !isSameOrDescendant(copyPath, of: trashPath) else {
                    errors.append("\(copy.name): 이미 휴지통 안에 있습니다")
                    continue
                }

                guard let copyHash = fullHash(
                    URL(fileURLWithPath: copyPath),
                    expectedSize: copy.size,
                    expectedModificationDate: copy.modified,
                    shouldCancel: shouldCancel
                ),
                !group.hash.isEmpty,
                copyHash == group.hash else {
                    errors.append("\(copy.name): 파일 내용이 스캔 뒤 변경되어 이동하지 않았습니다")
                    continue
                }

                // 복사본을 확인한 다음 보관할 파일을 마지막으로 전체 검증해,
                // 이동 시점에 검증된 보관본이 실제 경로에 남도록 한다.
                guard let keeperHash = fullHash(
                    URL(fileURLWithPath: keeperPath),
                    expectedSize: keeper.size,
                    expectedModificationDate: keeper.modified,
                    shouldCancel: shouldCancel
                ),
                keeperHash == group.hash,
                keeperHash == copyHash else {
                    errors.append("\(copy.name): 보관할 파일이 스캔 뒤 변경되었거나 없어 이동하지 않았습니다")
                    continue
                }

                // 해시 계산 직후의 짧은 경합 구간에서도 크기·수정 시간이
                // 달라졌다면 이동을 중단한다. 보관본은 언제나 현재 위치에 남긴다.
                guard keeper.snapshot.exactlyMatches(path: keeperPath),
                      copy.snapshot.exactlyMatches(path: copyPath),
                      duplicateSnapshotStillMatches(keeper, atPath: keeperPath),
                      duplicateSnapshotStillMatches(copy, atPath: copyPath) else {
                    errors.append("\(copy.name): 최종 확인 중 파일이 변경되어 이동하지 않았습니다")
                    continue
                }

                let actualSize = logicalSize(at: URL(fileURLWithPath: copyPath))
                guard actualSize == copy.size else {
                    errors.append("\(copy.name): 최종 확인 중 파일 크기가 변경되어 이동하지 않았습니다")
                    continue
                }

                guard keeper.snapshot.exactlyMatches(path: keeperPath),
                      copy.snapshot.exactlyMatches(path: copyPath) else {
                    errors.append("\(copy.name): 이동 직전 파일이 변경되어 이동하지 않았습니다")
                    continue
                }
                let result = VerifiedFileMover.shared.moveToTrash(
                    path: copyPath,
                    expectedSnapshot: copy.snapshot,
                    shouldCancel: shouldCancel
                )
                if result.succeeded {
                    totalFreed += actualSize
                } else {
                    errors.append("\(copy.name): \(result.error ?? "휴지통으로 이동하지 못했습니다")")
                }
            }
        }

        let unmatchedCount = selectedFileIDs.subtracting(matchedIDs).count
        if unmatchedCount > 0 {
            errors.append("스캔 결과와 일치하지 않는 선택 항목 \(unmatchedCount)개는 이동하지 않았습니다")
        }

        return (totalFreed, errors)
    }

    // MARK: - Fast Directory Size (URL enumerator, pre-fetched keys)

    private func fastDirSize(
        _ url: URL,
        shouldCancel: () -> Bool = { false }
    ) -> (Int64, Int) {
        var totalSize: Int64 = 0
        var count = 0
        let rootPath = normalizedPath(url.path)
        guard let rootSnapshot = FileIdentitySnapshot.capture(path: rootPath),
              rootSnapshot.kind == .directory else { return (0, 0) }

        let keys: Set<URLResourceKey> = [.fileSizeKey, .isDirectoryKey]
        guard let enumerator = FileManager.default.enumerator(
            at: url,
            includingPropertiesForKeys: Array(keys),
            options: []
        ) else { return (0, 0) }

        for case let fileURL as URL in enumerator {
            guard !shouldCancel() else { return (0, 0) }
            guard rootSnapshot.exactlyMatches(path: rootPath),
                  isSameOrDescendant(normalizedPath(fileURL.path), of: rootPath),
                  let snapshot = FileIdentitySnapshot.capture(path: fileURL.path),
                  snapshot.kind == .regularFile,
                  let values = try? fileURL.resourceValues(forKeys: keys),
                  values.isDirectory == false,
                  let size = values.fileSize else { continue }
            totalSize += Int64(size)
            count += 1
        }

        return (totalSize, count)
    }

    private func logicalSize(at url: URL) -> Int64 {
        guard let values = try? url.resourceValues(
            forKeys: [.fileSizeKey, .isDirectoryKey, .isSymbolicLinkKey]
        ) else {
            return 0
        }
        if values.isSymbolicLink == true {
            return Int64(values.fileSize ?? 0)
        }
        if values.isDirectory == true {
            return fastDirSize(url).0
        }
        return Int64(values.fileSize ?? 0)
    }

    private func normalizedPath(_ path: String) -> String {
        URL(fileURLWithPath: path).standardizedFileURL.path
    }

    private func pathDepth(_ path: String) -> Int {
        (normalizedPath(path) as NSString).pathComponents.count
    }

    private func isSameOrDescendant(_ path: String, of root: String) -> Bool {
        path == root || path.hasPrefix(root.hasSuffix("/") ? root : root + "/")
    }

    private func nonOverlappingTargets(
        _ targets: [(name: String, path: String, icon: String, desc: String, type: CacheItem.CacheType)]
    ) -> [(name: String, path: String, icon: String, desc: String, type: CacheItem.CacheType)] {
        var acceptedPaths: [String] = []
        var accepted: [(name: String, path: String, icon: String, desc: String, type: CacheItem.CacheType)] = []

        for target in targets.sorted(by: {
            pathDepth($0.path) < pathDepth($1.path)
        }) {
            let path = normalizedPath(target.path)
            guard !acceptedPaths.contains(where: { isSameOrDescendant(path, of: $0) }) else {
                continue
            }
            acceptedPaths.append(path)
            accepted.append((target.name, path, target.icon, target.desc, target.type))
        }

        return accepted
    }

    // MARK: - Quick Hash (head + tail + size)

    private func quickHash(
        _ url: URL,
        chunkSize: Int = 8192,
        shouldCancel: () -> Bool = { false }
    ) -> String? {
        guard !shouldCancel() else { return nil }
        guard let handle = try? FileHandle(forReadingFrom: url) else { return nil }
        defer { handle.closeFile() }

        var context = CC_MD5_CTX()
        CC_MD5_Init(&context)

        let headData = handle.readData(ofLength: chunkSize)
        guard !shouldCancel() else { return nil }
        headData.withUnsafeBytes { ptr in
            _ = CC_MD5_Update(&context, ptr.baseAddress, CC_LONG(headData.count))
        }

        let fileSize = handle.seekToEndOfFile()
        if fileSize > UInt64(chunkSize * 2) {
            guard !shouldCancel() else { return nil }
            handle.seek(toFileOffset: fileSize - UInt64(chunkSize))
            let tailData = handle.readData(ofLength: chunkSize)
            guard !shouldCancel() else { return nil }
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

    /// 전체 파일을 작은 조각으로 읽어 SHA-256을 계산한다.
    /// 스캔 당시 크기와 다르거나 읽는 동안 크기/수정 시간이 바뀐 파일은
    /// 중복 확정 결과에서 제외한다.
    private func fullHash(
        _ url: URL,
        expectedSize: Int64,
        expectedModificationDate: Date?,
        chunkSize: Int = 1_048_576,
        shouldCancel: () -> Bool = { false }
    ) -> String? {
        guard !shouldCancel() else { return nil }

        let keys: Set<URLResourceKey> = [
            .fileSizeKey,
            .contentModificationDateKey,
            .isRegularFileKey
        ]
        guard let before = try? url.resourceValues(forKeys: keys),
              before.isRegularFile == true,
              let beforeSize = before.fileSize,
              Int64(beforeSize) == expectedSize,
              before.contentModificationDate == expectedModificationDate,
              let handle = try? FileHandle(forReadingFrom: url) else {
            return nil
        }
        defer { try? handle.close() }

        var context = CC_SHA256_CTX()
        CC_SHA256_Init(&context)
        var bytesRead: Int64 = 0

        do {
            while true {
                guard !shouldCancel() else { return nil }
                guard let data = try handle.read(upToCount: chunkSize), !data.isEmpty else {
                    break
                }
                guard !shouldCancel() else { return nil }

                data.withUnsafeBytes { ptr in
                    _ = CC_SHA256_Update(&context, ptr.baseAddress, CC_LONG(data.count))
                }
                bytesRead += Int64(data.count)
            }
        } catch {
            return nil
        }

        guard !shouldCancel(), bytesRead == expectedSize,
              let after = try? url.resourceValues(forKeys: keys),
              after.isRegularFile == true,
              after.fileSize == before.fileSize,
              after.contentModificationDate == before.contentModificationDate else {
            return nil
        }

        var digest = [UInt8](repeating: 0, count: Int(CC_SHA256_DIGEST_LENGTH))
        CC_SHA256_Final(&digest, &context)
        return digest.map { String(format: "%02x", $0) }.joined()
    }

    private func duplicateSnapshotStillMatches(
        _ file: DuplicateFile,
        atPath path: String
    ) -> Bool {
        let keys: Set<URLResourceKey> = [
            .fileSizeKey,
            .contentModificationDateKey,
            .isRegularFileKey
        ]
        guard let values = try? URL(fileURLWithPath: path).resourceValues(forKeys: keys),
              values.isRegularFile == true,
              let size = values.fileSize else {
            return false
        }
        return Int64(size) == file.size && values.contentModificationDate == file.modified
    }
}
