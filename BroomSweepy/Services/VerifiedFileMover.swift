import Darwin
import Foundation

/// The only composition point that may move a reviewed user file.
///
/// A source is first renamed into a private, same-directory staging folder.
/// That rename is atomic and exclusive. The staged entry is then checked
/// against the scan-time no-follow identity before it can move any farther.
/// A durable journal lets the next launch restore an interrupted stage without
/// overwriting either the original path or another user file.
final class VerifiedFileMover: @unchecked Sendable {
    static let shared = VerifiedFileMover()

    struct MoveResult: Sendable {
        let succeeded: Bool
        let resultingPath: String?
        let error: String?
    }

    struct RecoveryReport: Sendable {
        var recovered: [String] = []
        var needsReview: [String] = []

        var isEmpty: Bool { recovered.isEmpty && needsReview.isEmpty }
    }

    private enum OperationKind: String, Codable, Equatable {
        case trash
        case move
    }

    private enum JournalState: String, Codable, Equatable {
        case prepared
        case staged
        case readyForTrash
        case completed
    }

    private struct JournalRecord: Codable {
        let id: UUID
        let kind: OperationKind
        let sourcePath: String
        let stagedPath: String
        let stagingDirectoryPath: String
        let destinationPath: String?
        let expectedSnapshot: FileIdentitySnapshot
        let trashSearchRoots: [String]
        let createdAt: Date
        var state: JournalState
        var resultingPath: String?
    }

    private enum MoveSafetyError: LocalizedError {
        case cancelled
        case directoryRequiresManifest
        case sourceChanged
        case unsafeStage
        case destinationOccupied
        case stagedIdentityChanged
        case resultingIdentityChanged
        case recoveryRequired(String)
        case startupRecoveryPending
        case system(String)

        var errorDescription: String? {
            switch self {
            case .cancelled:
                return "현재 항목을 휴지통으로 이동하기 전에 중단했습니다"
            case .directoryRequiresManifest:
                return "폴더 내부 전체를 다시 검증할 수 없어 자동 이동하지 않았습니다. Finder에서 검토해 주세요"
            case .sourceChanged:
                return "검토 뒤 파일이 변경되어 이동하지 않았습니다"
            case .unsafeStage:
                return "같은 디스크의 안전한 임시 보관 위치를 만들지 못했습니다"
            case .destinationOccupied:
                return "대상 위치에 다른 항목이 생겨 이동하지 않았습니다"
            case .stagedIdentityChanged:
                return "원자 이동 뒤 파일 동일성을 확인하지 못해 복구를 시도했습니다"
            case .resultingIdentityChanged:
                return "휴지통 또는 대상 위치에서 파일 동일성을 확인하지 못했습니다. 복구 기록을 보존했습니다"
            case .recoveryRequired(let path):
                return "자동 복구가 안전하지 않아 파일을 보존했습니다: \(path)"
            case .startupRecoveryPending:
                return "앱 시작 시 파일 복구 확인이 끝난 뒤 다시 시도해 주세요"
            case .system(let message):
                return message
            }
        }
    }

    private let fileManager = FileManager.default
    private let lock = NSLock()
    private let stagingPrefix = ".BroomSweepy-Recovery-"
    private let maximumJournalCount = 128
    private let maximumCompletedHistoryCount = 256
    private let maximumJournalBytes = 256 * 1024
    private let maximumTrashEntriesPerRoot = 20_000
    private var initialRecoveryCompleted = false

    private init() {}

    func moveToTrash(
        path: String,
        expectedSnapshot: FileIdentitySnapshot,
        shouldCancel: () -> Bool = { false }
    ) -> MoveResult {
        perform(
            kind: .trash,
            sourcePath: path,
            destinationPath: nil,
            expectedSnapshot: expectedSnapshot,
            shouldCancel: shouldCancel
        )
    }

    /// Used by the file organizer for a reviewed regular-file move and undo.
    /// The destination must already have a verified, non-symlink parent.
    func moveAtomically(
        sourcePath: String,
        destinationPath: String,
        expectedSnapshot: FileIdentitySnapshot,
        shouldCancel: () -> Bool = { false }
    ) -> MoveResult {
        perform(
            kind: .move,
            sourcePath: sourcePath,
            destinationPath: destinationPath,
            expectedSnapshot: expectedSnapshot,
            shouldCancel: shouldCancel
        )
    }

    /// Restores any entry that was staged but not conclusively delivered.
    /// Ambiguous entries are never overwritten or deleted; their private path
    /// and journal are kept for Finder review.
    func recoverPendingOperations(
        shouldCancel: () -> Bool = { false }
    ) -> RecoveryReport {
        lock.lock()
        defer {
            initialRecoveryCompleted = true
            lock.unlock()
        }

        var report = RecoveryReport()
        recoverCompletedHistories(into: &report, shouldCancel: shouldCancel)
        guard let directory = try? journalDirectory(create: false),
              let directorySnapshot = FileIdentitySnapshot.capture(path: directory.path),
              directorySnapshot.kind == .directory,
              let enumerator = fileManager.enumerator(
                at: directory,
                includingPropertiesForKeys: [.fileSizeKey],
                options: [.skipsSubdirectoryDescendants],
                errorHandler: nil
              ) else {
            return report
        }

        var journalURLs: [URL] = []
        var inspectedEntries = 0
        while let journalURL = enumerator.nextObject() as? URL {
            guard !shouldCancel() else {
                report.needsReview.append("파일 이동 복구 확인을 중단했습니다")
                return report
            }
            guard inspectedEntries < maximumJournalCount else {
                report.needsReview.append(
                    "복구 폴더 항목이 \(maximumJournalCount)개를 넘어 나머지는 자동 확인하지 않았습니다: \(directory.path)"
                )
                break
            }
            inspectedEntries += 1
            guard journalURL.pathExtension == "plist" else { continue }
            journalURLs.append(journalURL)
        }

        for journalURL in journalURLs.sorted(by: { $0.path < $1.path }) {
            guard !shouldCancel() else {
                report.needsReview.append("파일 이동 복구 확인을 중단했습니다")
                break
            }
            guard let snapshot = FileIdentitySnapshot.capture(path: journalURL.path),
                  snapshot.kind == .regularFile,
                  snapshot.size >= 0,
                  snapshot.size <= Int64(maximumJournalBytes),
                  let data = readBoundedFile(at: journalURL, maximumBytes: maximumJournalBytes),
                  snapshot.exactlyMatches(path: journalURL.path),
                  let record = try? PropertyListDecoder().decode(JournalRecord.self, from: data),
                  journalRecordIsStructurallyValid(record, at: journalURL) else {
                report.needsReview.append("손상되었거나 제한을 넘은 복구 기록: \(journalURL.path)")
                continue
            }

            if record.expectedSnapshot.exactlyMatches(path: record.sourcePath) {
                if !clearJournal(at: journalURL, stagingDirectoryPath: record.stagingDirectoryPath) {
                    report.needsReview.append("비어 있지 않은 복구 폴더를 보존했습니다: \(record.stagingDirectoryPath)")
                }
                continue
            }

            if record.expectedSnapshot.exactlyMatches(path: record.stagedPath) {
                do {
                    guard !entryExistsNoFollow(record.sourcePath) else {
                        report.needsReview.append(
                            "원래 위치가 사용 중이라 보존된 파일: \(record.stagedPath)"
                        )
                        continue
                    }
                    try atomicRename(from: record.stagedPath, to: record.sourcePath)
                    guard record.expectedSnapshot.exactlyMatches(path: record.sourcePath) else {
                        report.needsReview.append(
                            "복구 뒤 동일성 확인이 필요한 파일: \(record.sourcePath)"
                        )
                        continue
                    }
                    guard clearJournal(
                        at: journalURL,
                        stagingDirectoryPath: record.stagingDirectoryPath
                    ) else {
                        report.needsReview.append("비어 있지 않은 복구 폴더를 보존했습니다: \(record.stagingDirectoryPath)")
                        continue
                    }
                    report.recovered.append(record.sourcePath)
                } catch {
                    report.needsReview.append(
                        "자동 복구하지 못해 보존된 파일: \(record.stagedPath) — \(error.localizedDescription)"
                    )
                }
                continue
            }

            if record.kind == .move,
               let destinationPath = record.destinationPath,
               record.expectedSnapshot.exactlyMatches(path: destinationPath) {
                if !clearJournal(at: journalURL, stagingDirectoryPath: record.stagingDirectoryPath) {
                    report.needsReview.append("비어 있지 않은 복구 폴더를 보존했습니다: \(record.stagingDirectoryPath)")
                }
                continue
            }

            if record.kind == .trash {
                switch locateTrashedEntry(for: record, shouldCancel: shouldCancel) {
                case .found(let resultPath):
                    var completed = record
                    completed.state = .completed
                    completed.resultingPath = resultPath
                    do {
                        try persist(completed, at: journalURL)
                        try archiveCompletedTrash(completed, pendingJournalURL: journalURL)
                    } catch {
                        report.needsReview.append(
                            "휴지통 이동 기록을 보존하지 못해 복구 기록을 유지했습니다: \(journalURL.path)"
                        )
                    }
                    continue
                case .limitExceeded(let message):
                    report.needsReview.append(message)
                    continue
                case .notFound:
                    break
                }
            }

            report.needsReview.append(
                "원본 위치와 복구 위치를 자동 확인하지 못했습니다: \(record.sourcePath) " +
                "(복구 기록: \(journalURL.path))"
            )
        }

        return report
    }

    private func perform(
        kind: OperationKind,
        sourcePath: String,
        destinationPath: String?,
        expectedSnapshot: FileIdentitySnapshot,
        shouldCancel: () -> Bool
    ) -> MoveResult {
        lock.lock()
        defer { lock.unlock() }

        var record: JournalRecord?
        var journalURL: URL?

        do {
            guard initialRecoveryCompleted else {
                throw MoveSafetyError.startupRecoveryPending
            }
            guard expectedSnapshot.kind == .regularFile else {
                throw MoveSafetyError.directoryRequiresManifest
            }
            guard !shouldCancel() else { throw MoveSafetyError.cancelled }
            try ensureNoPendingJournals()
            if kind == .trash {
                try ensureCompletedHistoryCapacity()
            }
            guard expectedSnapshot.exactlyMatches(path: sourcePath) else {
                throw MoveSafetyError.sourceChanged
            }
            if let destinationPath, entryExistsNoFollow(destinationPath) {
                throw MoveSafetyError.destinationOccupied
            }

            let prepared = try makeJournalRecord(
                kind: kind,
                sourcePath: sourcePath,
                destinationPath: destinationPath,
                expectedSnapshot: expectedSnapshot
            )
            record = prepared.record
            journalURL = prepared.url
            try persist(prepared.record, at: prepared.url)
            try createStagingDirectory(for: prepared.record)

            try atomicRename(from: sourcePath, to: prepared.record.stagedPath)
            var stagedRecord = prepared.record
            stagedRecord.state = .staged
            record = stagedRecord
            try persist(stagedRecord, at: prepared.url)

            guard expectedSnapshot.exactlyMatches(path: stagedRecord.stagedPath) else {
                throw MoveSafetyError.stagedIdentityChanged
            }
            guard !shouldCancel() else { throw MoveSafetyError.cancelled }

            switch kind {
            case .trash:
                var resultingURL: NSURL?
                try fileManager.trashItem(
                    at: URL(fileURLWithPath: stagedRecord.stagedPath),
                    resultingItemURL: &resultingURL
                )
                guard let resultPath = resultingURL?.path,
                      expectedSnapshot.exactlyMatches(path: resultPath) else {
                    var unresolved = stagedRecord
                    unresolved.resultingPath = resultingURL?.path
                    try? persist(unresolved, at: prepared.url)
                    throw MoveSafetyError.resultingIdentityChanged
                }
                var completed = stagedRecord
                completed.state = .completed
                completed.resultingPath = resultPath
                record = completed
                try persist(completed, at: prepared.url)
                try archiveCompletedTrash(completed, pendingJournalURL: prepared.url)
                return MoveResult(succeeded: true, resultingPath: resultPath, error: nil)

            case .move:
                guard let destinationPath else {
                    throw MoveSafetyError.system("대상 경로가 없습니다")
                }
                guard !entryExistsNoFollow(destinationPath) else {
                    throw MoveSafetyError.destinationOccupied
                }
                try atomicRename(from: stagedRecord.stagedPath, to: destinationPath)
                guard expectedSnapshot.exactlyMatches(path: destinationPath) else {
                    var unresolved = stagedRecord
                    unresolved.resultingPath = destinationPath
                    record = unresolved
                    try? persist(unresolved, at: prepared.url)
                    throw MoveSafetyError.resultingIdentityChanged
                }
                var completed = stagedRecord
                completed.state = .completed
                completed.resultingPath = destinationPath
                record = completed
                try persist(completed, at: prepared.url)
                clearJournal(at: prepared.url, stagingDirectoryPath: completed.stagingDirectoryPath)
                return MoveResult(succeeded: true, resultingPath: destinationPath, error: nil)
            }
        } catch {
            let recoveryError = restoreStagedEntryIfPossible(record)
            let baseMessage = (error as? LocalizedError)?.errorDescription ?? error.localizedDescription
            let message = recoveryError.map { "\(baseMessage). \($0)" } ?? baseMessage
            if recoveryError == nil,
               let journalURL,
               let record,
               record.expectedSnapshot.exactlyMatches(path: record.sourcePath) {
                clearJournal(at: journalURL, stagingDirectoryPath: record.stagingDirectoryPath)
            }
            if let record,
               record.kind == .move,
               let resultingPath = record.resultingPath,
               record.expectedSnapshot.exactlyMatches(path: resultingPath) {
                return MoveResult(
                    succeeded: true,
                    resultingPath: resultingPath,
                    error: "파일 이동은 완료됐지만 복구 기록을 마무리하지 못했습니다. 되돌리기 목록을 유지합니다"
                )
            }
            return MoveResult(succeeded: false, resultingPath: nil, error: message)
        }
    }

    private func makeJournalRecord(
        kind: OperationKind,
        sourcePath: String,
        destinationPath: String?,
        expectedSnapshot: FileIdentitySnapshot
    ) throws -> (record: JournalRecord, url: URL) {
        let id = UUID()
        let journalDirectory = try journalDirectory(create: true)
        let sourceURL = URL(fileURLWithPath: sourcePath).standardizedFileURL
        let parentURL = sourceURL.deletingLastPathComponent()
        let stagingDirectory = parentURL.appendingPathComponent(stagingPrefix + id.uuidString, isDirectory: true)

        let stagedURL = stagingDirectory.appendingPathComponent(sourceURL.lastPathComponent)

        let record = JournalRecord(
            id: id,
            kind: kind,
            sourcePath: sourceURL.path,
            stagedPath: stagedURL.path,
            stagingDirectoryPath: stagingDirectory.path,
            destinationPath: destinationPath.map { URL(fileURLWithPath: $0).standardizedFileURL.path },
            expectedSnapshot: expectedSnapshot,
            trashSearchRoots: trashSearchRoots(for: sourceURL),
            createdAt: Date(),
            state: .prepared,
            resultingPath: nil
        )
        return (record, journalDirectory.appendingPathComponent(id.uuidString + ".plist"))
    }

    private func ensureNoPendingJournals() throws {
        let directory = try journalDirectory(create: true)
        guard let snapshot = FileIdentitySnapshot.capture(path: directory.path),
              snapshot.kind == .directory,
              let enumerator = fileManager.enumerator(
                at: directory,
                includingPropertiesForKeys: nil,
                options: [.skipsSubdirectoryDescendants],
                errorHandler: nil
              ) else {
            throw MoveSafetyError.system("복구 기록 폴더를 안전하게 확인하지 못했습니다")
        }

        var journalCount = 0
        var inspectedEntries = 0
        while let entry = enumerator.nextObject() as? URL {
            inspectedEntries += 1
            guard inspectedEntries <= maximumJournalCount else {
                throw MoveSafetyError.recoveryRequired(directory.path)
            }
            guard entry.pathExtension == "plist" else { continue }
            journalCount += 1
        }
        guard journalCount == 0 else {
            throw MoveSafetyError.system(
                "이전 파일 이동의 복구 확인이 남아 있어 새 이동을 시작하지 않았습니다. 앱을 다시 열어 복구 안내를 확인해 주세요"
            )
        }
    }

    private func ensureCompletedHistoryCapacity() throws {
        let directory = try completedHistoryDirectory(create: true)
        guard let snapshot = FileIdentitySnapshot.capture(path: directory.path),
              snapshot.kind == .directory,
              let enumerator = fileManager.enumerator(
                at: directory,
                includingPropertiesForKeys: nil,
                options: [.skipsSubdirectoryDescendants],
                errorHandler: nil
              ) else {
            throw MoveSafetyError.system("휴지통 복원 기록 폴더를 안전하게 확인하지 못했습니다")
        }

        var count = 0
        var inspectedEntries = 0
        while let entry = enumerator.nextObject() as? URL {
            inspectedEntries += 1
            guard inspectedEntries <= maximumCompletedHistoryCount else {
                throw MoveSafetyError.system(
                    "휴지통 복원 폴더 항목이 \(maximumCompletedHistoryCount)개를 넘어 새 이동을 시작하지 않았습니다"
                )
            }
            guard entry.pathExtension == "plist" else { continue }
            count += 1
            guard count < maximumCompletedHistoryCount else {
                throw MoveSafetyError.system(
                    "휴지통 복원 기록이 \(maximumCompletedHistoryCount)개입니다. 휴지통을 검토한 뒤 앱을 다시 열어 주세요"
                )
            }
        }
    }

    private func archiveCompletedTrash(
        _ record: JournalRecord,
        pendingJournalURL: URL
    ) throws {
        try ensureCompletedHistoryCapacity()
        let directory = try completedHistoryDirectory(create: true)
        let historyURL = directory.appendingPathComponent(record.id.uuidString + ".plist")
        guard !entryExistsNoFollow(historyURL.path) else {
            throw MoveSafetyError.system("같은 휴지통 복원 기록이 이미 있습니다")
        }
        try atomicRename(from: pendingJournalURL.path, to: historyURL.path)
        try synchronize(path: pendingJournalURL.deletingLastPathComponent().path)
        try synchronize(path: historyURL.deletingLastPathComponent().path)
    }

    private func recoverCompletedHistories(
        into report: inout RecoveryReport,
        shouldCancel: () -> Bool
    ) {
        guard let directory = try? completedHistoryDirectory(create: false),
              let snapshot = FileIdentitySnapshot.capture(path: directory.path),
              snapshot.kind == .directory,
              let enumerator = fileManager.enumerator(
                at: directory,
                includingPropertiesForKeys: [.fileSizeKey],
                options: [.skipsSubdirectoryDescendants],
                errorHandler: nil
              ) else { return }

        var historyURLs: [URL] = []
        var inspectedEntries = 0
        while let historyURL = enumerator.nextObject() as? URL {
            guard !shouldCancel() else {
                report.needsReview.append("휴지통 복원 기록 확인을 중단했습니다")
                return
            }
            guard inspectedEntries < maximumCompletedHistoryCount else {
                report.needsReview.append(
                    "휴지통 복원 폴더 항목이 \(maximumCompletedHistoryCount)개를 넘어 나머지는 자동 확인하지 않았습니다"
                )
                break
            }
            inspectedEntries += 1
            guard historyURL.pathExtension == "plist" else { continue }
            historyURLs.append(historyURL)
        }

        for historyURL in historyURLs.sorted(by: { $0.path < $1.path }) {
            guard !shouldCancel() else {
                report.needsReview.append("휴지통 복원 기록 확인을 중단했습니다")
                return
            }
            guard let fileSnapshot = FileIdentitySnapshot.capture(path: historyURL.path),
                  fileSnapshot.kind == .regularFile,
                  fileSnapshot.size >= 0,
                  fileSnapshot.size <= Int64(maximumJournalBytes),
                  let data = readBoundedFile(at: historyURL, maximumBytes: maximumJournalBytes),
                  fileSnapshot.exactlyMatches(path: historyURL.path),
                  let record = try? PropertyListDecoder().decode(JournalRecord.self, from: data),
                  journalRecordIsStructurallyValid(record, at: historyURL),
                  record.kind == .trash,
                  record.state == .completed else {
                report.needsReview.append("손상되었거나 제한을 넘은 휴지통 복원 기록: \(historyURL.path)")
                continue
            }

            if record.expectedSnapshot.exactlyMatches(path: record.sourcePath) {
                if !clearJournal(at: historyURL, stagingDirectoryPath: record.stagingDirectoryPath) {
                    report.needsReview.append("복원 폴더를 자동 정리하지 못했습니다: \(record.stagingDirectoryPath)")
                }
                continue
            }

            if record.expectedSnapshot.exactlyMatches(path: record.stagedPath) {
                guard !entryExistsNoFollow(record.sourcePath) else {
                    report.needsReview.append("원래 위치가 사용 중이라 복원 파일을 보존했습니다: \(record.stagedPath)")
                    continue
                }
                do {
                    try atomicRename(from: record.stagedPath, to: record.sourcePath)
                    guard record.expectedSnapshot.exactlyMatches(path: record.sourcePath),
                          clearJournal(at: historyURL, stagingDirectoryPath: record.stagingDirectoryPath) else {
                        report.needsReview.append("복원 뒤 확인이 필요한 파일: \(record.sourcePath)")
                        continue
                    }
                    report.recovered.append(record.sourcePath)
                } catch {
                    report.needsReview.append("원래 위치로 복원하지 못한 파일: \(record.stagedPath)")
                }
                continue
            }

            if let resultingPath = record.resultingPath,
               record.expectedSnapshot.exactlyMatches(path: resultingPath) {
                if !restoreDirectoryIsReady(for: record) {
                    report.needsReview.append(
                        "Finder 복원 위치를 안전하게 유지하지 못했습니다: \(record.stagingDirectoryPath)"
                    )
                }
                continue
            }

            // The item is no longer at the exact Trash URL (for example, the
            // user emptied Trash or moved it elsewhere). Only app-owned empty
            // metadata is removed; no user entry is deleted here.
            if !clearJournal(at: historyURL, stagingDirectoryPath: record.stagingDirectoryPath) {
                report.needsReview.append("휴지통 복원 경로를 확인해 주세요: \(record.stagingDirectoryPath)")
            }
        }
    }

    private func restoreDirectoryIsReady(for record: JournalRecord) -> Bool {
        if !entryExistsNoFollow(record.stagingDirectoryPath) {
            guard mkdir(record.stagingDirectoryPath, mode_t(S_IRWXU)) == 0 else { return false }
        }
        guard let snapshot = FileIdentitySnapshot.capture(path: record.stagingDirectoryPath),
              snapshot.kind == .directory,
              snapshot.device == record.expectedSnapshot.device,
              let enumerator = fileManager.enumerator(
                at: URL(fileURLWithPath: record.stagingDirectoryPath),
                includingPropertiesForKeys: nil,
                options: [.skipsSubdirectoryDescendants],
                errorHandler: nil
              ),
              enumerator.nextObject() == nil else {
            return false
        }
        return true
    }

    private func createStagingDirectory(for record: JournalRecord) throws {
        guard mkdir(record.stagingDirectoryPath, mode_t(S_IRWXU)) == 0 else {
            throw MoveSafetyError.system(posixMessage(prefix: "임시 보관 폴더를 만들지 못했습니다"))
        }
        guard let stageSnapshot = FileIdentitySnapshot.capture(path: record.stagingDirectoryPath),
              stageSnapshot.kind == .directory,
              stageSnapshot.device == record.expectedSnapshot.device,
              !entryExistsNoFollow(record.stagedPath) else {
            _ = rmdir(record.stagingDirectoryPath)
            throw MoveSafetyError.unsafeStage
        }
    }

    private func journalRecordIsStructurallyValid(
        _ record: JournalRecord,
        at journalURL: URL
    ) -> Bool {
        guard journalURL.lastPathComponent == record.id.uuidString + ".plist",
              record.expectedSnapshot.kind == .regularFile,
              record.trashSearchRoots.count <= 4 else { return false }

        let sourceURL = URL(fileURLWithPath: record.sourcePath).standardizedFileURL
        guard sourceURL.path == record.sourcePath,
              !sourceURL.lastPathComponent.isEmpty else { return false }
        let expectedDirectory = sourceURL.deletingLastPathComponent()
            .appendingPathComponent(stagingPrefix + record.id.uuidString, isDirectory: true)
            .standardizedFileURL
        let expectedStagedPath = expectedDirectory
            .appendingPathComponent(sourceURL.lastPathComponent)
            .standardizedFileURL.path
        guard expectedDirectory.path == record.stagingDirectoryPath,
              expectedStagedPath == record.stagedPath else { return false }

        if let destinationPath = record.destinationPath {
            let destinationURL = URL(fileURLWithPath: destinationPath).standardizedFileURL
            guard record.kind == .move,
                  destinationURL.path == destinationPath,
                  destinationPath != record.sourcePath else { return false }
        } else if record.kind == .move {
            return false
        }
        return true
    }

    private func restoreStagedEntryIfPossible(_ record: JournalRecord?) -> String? {
        guard let record else { return nil }

        if record.expectedSnapshot.exactlyMatches(path: record.sourcePath) {
            return nil
        }
        if record.kind == .move,
           let resultingPath = record.resultingPath,
           record.expectedSnapshot.exactlyMatches(path: resultingPath) {
            guard !entryExistsNoFollow(record.sourcePath) else {
                return "원래 위치가 사용 중이라 이동된 파일을 보존했습니다: \(resultingPath)"
            }
            do {
                try atomicRename(from: resultingPath, to: record.sourcePath)
                guard record.expectedSnapshot.exactlyMatches(path: record.sourcePath) else {
                    return "원래 위치로 돌려놓았지만 파일 동일성을 다시 확인해야 합니다: \(record.sourcePath)"
                }
                return nil
            } catch {
                return "자동으로 되돌리지 못해 이동된 파일과 복구 기록을 보존했습니다: \(resultingPath)"
            }
        }
        guard record.expectedSnapshot.exactlyMatches(path: record.stagedPath) else {
            if let resultingPath = record.resultingPath,
               record.expectedSnapshot.exactlyMatches(path: resultingPath) {
                return "파일은 다음 위치에 보존되어 있습니다: \(resultingPath)"
            }
            return "복구 기록을 보존했습니다: \(record.stagedPath)"
        }
        guard !entryExistsNoFollow(record.sourcePath) else {
            return MoveSafetyError.recoveryRequired(record.stagedPath).localizedDescription
        }

        do {
            try atomicRename(from: record.stagedPath, to: record.sourcePath)
            guard record.expectedSnapshot.exactlyMatches(path: record.sourcePath) else {
                return "원래 위치로 돌려놓았지만 파일 동일성을 다시 확인해야 합니다: \(record.sourcePath)"
            }
            return nil
        } catch {
            return "자동 복구하지 못해 파일과 복구 기록을 보존했습니다: \(record.stagedPath)"
        }
    }

    private func atomicRename(from sourcePath: String, to destinationPath: String) throws {
        let status = sourcePath.withCString { source in
            destinationPath.withCString { destination in
                renamex_np(
                    source,
                    destination,
                    UInt32(RENAME_EXCL) | UInt32(RENAME_NOFOLLOW_ANY)
                )
            }
        }
        guard status == 0 else {
            throw MoveSafetyError.system(posixMessage(prefix: "원자 이동에 실패했습니다"))
        }
    }

    private func persist(_ record: JournalRecord, at url: URL) throws {
        let encoder = PropertyListEncoder()
        encoder.outputFormat = .binary
        let data = try encoder.encode(record)
        try data.write(to: url, options: [.atomic])
        try synchronize(path: url.path)
        try synchronize(path: url.deletingLastPathComponent().path)
    }

    private func readBoundedFile(at url: URL, maximumBytes: Int) -> Data? {
        do {
            let handle = try FileHandle(forReadingFrom: url)
            defer { try? handle.close() }
            var data = Data()
            let chunkSize = 64 * 1024
            while data.count <= maximumBytes {
                let remaining = maximumBytes + 1 - data.count
                guard let chunk = try handle.read(upToCount: min(chunkSize, remaining)),
                      !chunk.isEmpty else {
                    return data
                }
                data.append(chunk)
                guard data.count <= maximumBytes else { return nil }
            }
            return nil
        } catch {
            return nil
        }
    }

    private func synchronize(path: String) throws {
        let descriptor = open(path, O_RDONLY)
        guard descriptor >= 0 else {
            throw MoveSafetyError.system(posixMessage(prefix: "복구 기록을 열지 못했습니다"))
        }
        defer { close(descriptor) }
        guard fsync(descriptor) == 0 else {
            throw MoveSafetyError.system(posixMessage(prefix: "복구 기록을 디스크에 기록하지 못했습니다"))
        }
    }

    private func journalDirectory(create: Bool) throws -> URL {
        guard let applicationSupport = fileManager.urls(
            for: .applicationSupportDirectory,
            in: .userDomainMask
        ).first else {
            throw MoveSafetyError.system("복구 기록 폴더를 찾지 못했습니다")
        }
        let directory = applicationSupport
            .appendingPathComponent("BroomSweepy", isDirectory: true)
            .appendingPathComponent("MoveRecovery", isDirectory: true)
        if create {
            try fileManager.createDirectory(
                at: directory,
                withIntermediateDirectories: true,
                attributes: [.posixPermissions: 0o700]
            )
        }
        return directory
    }

    private func completedHistoryDirectory(create: Bool) throws -> URL {
        guard let applicationSupport = fileManager.urls(
            for: .applicationSupportDirectory,
            in: .userDomainMask
        ).first else {
            throw MoveSafetyError.system("휴지통 복원 기록 폴더를 찾지 못했습니다")
        }
        let directory = applicationSupport
            .appendingPathComponent("BroomSweepy", isDirectory: true)
            .appendingPathComponent("TrashRestoreHistory", isDirectory: true)
        if create {
            try fileManager.createDirectory(
                at: directory,
                withIntermediateDirectories: true,
                attributes: [.posixPermissions: 0o700]
            )
        }
        return directory
    }

    @discardableResult
    private func clearJournal(at journalURL: URL, stagingDirectoryPath: String) -> Bool {
        let stagingDirectoryURL = URL(fileURLWithPath: stagingDirectoryPath).standardizedFileURL
        let stagingName = stagingDirectoryURL.lastPathComponent
        guard stagingDirectoryURL.path == stagingDirectoryPath,
              stagingName.hasPrefix(stagingPrefix),
              UUID(uuidString: String(stagingName.dropFirst(stagingPrefix.count))) != nil else {
            return false
        }
        if entryExistsNoFollow(stagingDirectoryPath) {
            guard let snapshot = FileIdentitySnapshot.capture(path: stagingDirectoryPath),
                  snapshot.kind == .directory,
                  let enumerator = fileManager.enumerator(
                    at: URL(fileURLWithPath: stagingDirectoryPath),
                    includingPropertiesForKeys: nil,
                    options: [.skipsSubdirectoryDescendants],
                    errorHandler: nil
                  ),
                  enumerator.nextObject() == nil,
                  rmdir(stagingDirectoryPath) == 0 else {
                return false
            }
        }
        guard unlink(journalURL.path) == 0 || errno == ENOENT else { return false }
        _ = fsyncParent(of: journalURL.path)
        return true
    }

    private func fsyncParent(of path: String) -> Int32 {
        let parent = URL(fileURLWithPath: path).deletingLastPathComponent().path
        let descriptor = open(parent, O_RDONLY)
        guard descriptor >= 0 else { return -1 }
        defer { close(descriptor) }
        return fsync(descriptor)
    }

    private enum TrashLookup {
        case found(String)
        case notFound
        case limitExceeded(String)
    }

    private func locateTrashedEntry(
        for record: JournalRecord,
        shouldCancel: () -> Bool
    ) -> TrashLookup {
        if let resultingPath = record.resultingPath,
           record.expectedSnapshot.exactlyMatches(path: resultingPath) {
            return .found(resultingPath)
        }

        for root in record.trashSearchRoots {
            guard let rootSnapshot = FileIdentitySnapshot.capture(path: root),
                  rootSnapshot.kind == .directory,
                  let enumerator = fileManager.enumerator(
                    at: URL(fileURLWithPath: root),
                    includingPropertiesForKeys: nil,
                    options: [.skipsSubdirectoryDescendants],
                    errorHandler: nil
                  ) else { continue }
            var inspected = 0
            while let entry = enumerator.nextObject() as? URL {
                guard !shouldCancel() else {
                    return .limitExceeded("휴지통 복구 확인을 중단했습니다: \(root)")
                }
                guard inspected < maximumTrashEntriesPerRoot else {
                    return .limitExceeded(
                        "휴지통 항목이 \(maximumTrashEntriesPerRoot)개를 넘어 자동 복구 확인을 중단했습니다: \(root)"
                    )
                }
                inspected += 1
                if record.expectedSnapshot.exactlyMatches(path: entry.path) {
                    return .found(entry.path)
                }
            }
        }
        return .notFound
    }

    private func trashSearchRoots(for sourceURL: URL) -> [String] {
        var roots = [actualUserHomeURL().appendingPathComponent(".Trash", isDirectory: true).path]
        if let values = try? sourceURL.resourceValues(forKeys: [.volumeURLKey]),
           let volumeURL = values.volume {
            roots.append(
                volumeURL
                    .appendingPathComponent(".Trashes", isDirectory: true)
                    .appendingPathComponent(String(getuid()), isDirectory: true)
                    .path
            )
        }
        return Array(Set(roots))
    }

    private func entryExistsNoFollow(_ path: String) -> Bool {
        var value = stat()
        return lstat(path, &value) == 0
    }

    private func posixMessage(prefix: String) -> String {
        let detail = String(cString: strerror(errno))
        return "\(prefix): \(detail)"
    }
}
