import SwiftUI
import AppKit

// MARK: - Smart Clean Models

struct SmartCleanFile: Identifiable {
    let id = UUID()
    let name: String
    let path: String
    let size: Int64
    var isChecked: Bool = true

    var sizeFormatted: String { formatSize(size) }
}

struct SmartCleanGroup: Identifiable {
    let id = UUID()
    let icon: String
    let name: String
    let safety: Safety
    let detail: String
    var files: [SmartCleanFile]

    var totalSize: Int64 { files.reduce(0) { $0 + $1.size } }
    var checkedSize: Int64 { files.filter(\.isChecked).reduce(0) { $0 + $1.size } }
    var checkedCount: Int { files.filter(\.isChecked).count }
    var sizeFormatted: String { formatSize(totalSize) }
    var checkedSizeFormatted: String { formatSize(checkedSize) }

    enum Safety: String {
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
}

// MARK: - Smart Clean View

struct SmartCleanView: View {
    @Bindable var viewModel: CleanerViewModel
    @Binding var isPresented: Bool

    @State private var phase: Phase = .scanning
    @State private var groups: [SmartCleanGroup] = []
    @State private var scanMessage = ""
    @State private var scanProgress: Double = 0
    @State private var freedAmount: Int64 = 0
    @State private var cleanErrors: [String] = []
    @State private var celebrateBounce = 0
    @State private var expandedGroupID: UUID?
    @State private var showCleanConfirmation = false
    @State private var activeLease: CleanerOperationLease?

    enum Phase { case scanning, result, cleaning, done }

    private var safeCheckedTotal: Int64 {
        groups.filter { $0.safety == .safe }.reduce(0) { $0 + $1.checkedSize }
    }

    private var allCheckedTotal: Int64 {
        var seenPaths: Set<String> = []
        return groups.flatMap(\.files).reduce(Int64(0)) { total, file in
            guard file.isChecked, seenPaths.insert(file.path).inserted else { return total }
            return total + file.size
        }
    }

    private var allCheckedCount: Int {
        Set(groups.flatMap(\.files).filter(\.isChecked).map(\.path)).count
    }

    private var highestCheckedRisk: SmartCleanGroup.Safety {
        if groups.contains(where: { $0.safety == .caution && $0.checkedCount > 0 }) {
            return .caution
        }
        if groups.contains(where: { $0.safety == .review && $0.checkedCount > 0 }) {
            return .review
        }
        return .safe
    }

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Text("원클릭 스마트 정리")
                    .font(.title2.bold())
                Spacer()
                Button {
                    withAnimation(.spring(duration: 0.3)) { isPresented = false }
                } label: {
                    Label("대시보드로", systemImage: "chevron.left")
                }
                .buttonStyle(.bordered)
            }
            .padding(24)

            Divider()

            Group {
                switch phase {
                case .scanning: scanningPhase
                case .result: resultPhase
                case .cleaning: cleaningPhase
                case .done: donePhase
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .task { await runSmartScan() }
        .onDisappear {
            if let lease = activeLease {
                viewModel.cancelCoordinatedOperation(lease)
                activeLease = nil
            }
        }
        .alert("휴지통으로 이동하기 전 최종 확인", isPresented: $showCleanConfirmation) {
            Button("취소", role: .cancel) {}
            Button("휴지통으로 이동", role: .destructive) {
                Task { await performSmartClean() }
            }
        } message: {
            Text(
                "\(allCheckedCount)개 항목, \(formatSize(allCheckedTotal))\n" +
                "가장 높은 위험도: \(highestCheckedRisk.rawValue)\n\n" +
                "선택 항목을 휴지통으로 이동합니다. 휴지통을 비우기 전에는 복원할 수 있으며 디스크 여유는 늘어나지 않습니다."
            )
        }
    }

    // MARK: - Scanning

    private var scanningPhase: some View {
        VStack(spacing: 24) {
            ZStack {
                Circle()
                    .fill(RadialGradient(
                        colors: [.blue.opacity(0.1), .clear],
                        center: .center, startRadius: 30, endRadius: 100
                    ))
                    .frame(width: 200, height: 200)
                    .blur(radius: 15)

                Image(systemName: "magnifyingglass")
                    .font(.system(size: 48))
                    .foregroundStyle(.blue)
                    .symbolEffect(.bounce, value: scanProgress)
            }

            Text("Mac을 분석하고 있습니다...")
                .font(.title3.bold())

            Text(scanMessage)
                .font(.callout)
                .foregroundStyle(.secondary)

            ProgressView(value: scanProgress)
                .frame(maxWidth: 400)
                .tint(.blue)

            if let lease = activeLease {
                Button("검사 중단 요청") {
                    viewModel.cancelCoordinatedOperation(lease)
                }
                .buttonStyle(.bordered)
                .disabled(lease.isCancelled)
            }
        }
        .padding(32)
    }

    // MARK: - Result (그룹 목록 + 펼침 가능한 파일 리스트)

    private var resultPhase: some View {
        VStack(spacing: 0) {
            // 상단 요약
            HStack(spacing: 20) {
                VStack(spacing: 4) {
                    Text(formatSize(safeCheckedTotal))
                        .font(.system(size: 28, weight: .bold, design: .rounded))
                        .contentTransition(.numericText())
                        .foregroundStyle(.green)
                    Text("안전 항목 선택됨")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                .frame(maxWidth: .infinity)
                .padding(16)
                .background(.green.opacity(0.06), in: RoundedRectangle(cornerRadius: 12))

                VStack(spacing: 4) {
                    Text(formatSize(allCheckedTotal))
                        .font(.system(size: 28, weight: .bold, design: .rounded))
                        .contentTransition(.numericText())
                        .foregroundStyle(.blue)
                    Text("전체 선택됨")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                .frame(maxWidth: .infinity)
                .padding(16)
                .background(.blue.opacity(0.06), in: RoundedRectangle(cornerRadius: 12))
            }
            .padding(.horizontal, 24)
            .padding(.top, 16)
            .padding(.bottom, 12)

            // 그룹 + 파일 리스트
            ScrollView {
                VStack(spacing: 10) {
                    ForEach($groups) { $group in
                        SmartGroupCard(
                            group: $group,
                            isExpanded: expandedGroupID == group.id,
                            onToggleExpand: {
                                withAnimation(.spring(duration: 0.3, bounce: 0.2)) {
                                    expandedGroupID = expandedGroupID == group.id ? nil : group.id
                                }
                            }
                        )
                    }
                }
                .padding(.horizontal, 24)
                .padding(.bottom, 16)
            }

            // 하단 액션 버튼
            HStack(spacing: 12) {
                Button("안전 항목만 선택") {
                    withAnimation {
                        for i in groups.indices {
                            for j in groups[i].files.indices {
                                groups[i].files[j].isChecked = groups[i].safety == .safe
                            }
                        }
                    }
                }
                .buttonStyle(.bordered)

                Spacer()

                Button {
                    showCleanConfirmation = true
                } label: {
                    HStack(spacing: 8) {
                        Image(systemName: "trash")
                        Text("선택 항목 휴지통으로 이동 (\(formatSize(allCheckedTotal)))")
                            .font(.headline)
                    }
                    .padding(.horizontal, 24)
                    .padding(.vertical, 10)
                }
                .buttonStyle(.borderedProminent)
                .tint(.green)
                .disabled(allCheckedTotal == 0)
            }
            .padding(.horizontal, 24)
            .padding(.vertical, 16)
            .background(.bar)
        }
    }

    // MARK: - Cleaning / Done

    private var cleaningPhase: some View {
        VStack(spacing: 24) {
            Image(systemName: "sparkles")
                .font(.system(size: 56))
                .foregroundStyle(.green)
                .symbolEffect(.bounce, value: celebrateBounce)
            Text("휴지통으로 이동 중...")
                .font(.title3.bold())
            ProgressView()
                .scaleEffect(1.5)

            if let lease = activeLease {
                Button("현재 항목 후 중단") {
                    viewModel.cancelCoordinatedOperation(lease)
                }
                .buttonStyle(.bordered)
                .disabled(lease.isCancelled)
            }
        }
    }

    private var donePhase: some View {
        VStack(spacing: 24) {
            Image(systemName: "checkmark.seal.fill")
                .font(.system(size: 72))
                .foregroundStyle(.green)
                .symbolEffect(.bounce, value: celebrateBounce)

            Text(formatSize(freedAmount))
                .font(.system(size: 48, weight: .bold, design: .rounded))
                .contentTransition(.numericText())
                .foregroundStyle(.green)

            Text("휴지통으로 이동한 논리 용량")
                .font(.title3)
                .foregroundStyle(.secondary)

            Text("휴지통을 비워야 디스크 여유가 늘어납니다.")
                .font(.caption)
                .foregroundStyle(.secondary)

            if !cleanErrors.isEmpty {
                VStack(spacing: 4) {
                    Text("\(cleanErrors.count)개 항목은 이동하지 못했습니다")
                        .font(.callout.bold())
                        .foregroundStyle(.red)
                    Text(cleanErrors[0])
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .lineLimit(2)
                }
                .padding(12)
                .background(.red.opacity(0.06), in: RoundedRectangle(cornerRadius: 10))
            }

            Button("대시보드로 돌아가기") {
                withAnimation(.spring(duration: 0.3)) { isPresented = false }
            }
            .buttonStyle(.borderedProminent)
            .controlSize(.large)
        }
        .onAppear { celebrateBounce += 1 }
    }

    // MARK: - Smart Scan

    @MainActor
    private func runSmartScan() async {
        guard let lease = viewModel.beginCoordinatedOperation(
            message: "스마트 정리 검사 중..."
        ) else {
            isPresented = false
            return
        }
        activeLease = lease
        defer {
            viewModel.finishCoordinatedOperation(lease)
            if activeLease?.id == lease.id { activeLease = nil }
        }
        let scanURL: URL
        if let bookmark = FileAccessManager.shared.loadBookmark() {
            scanURL = bookmark
        } else if let granted = FileAccessManager.shared.requestHomeAccess() {
            scanURL = granted
        } else {
            isPresented = false
            viewModel.toastMessage = "폴더 접근 권한이 필요합니다"
            return
        }

        let engine = CleanerEngine.shared

        scanMessage = "캐시 분석 중..."
        scanProgress = 0.1

        let (cacheItems, largeFiles) = await withTaskCancellationHandler {
            async let cacheTask = Task.detached {
                engine.scanCache(
                    homeURL: scanURL,
                    progressCallback: nil,
                    shouldCancel: { lease.isCancelled }
                )
            }.value
            async let largeTask = Task.detached {
                engine.scanLargeFiles(
                    scanURL: scanURL,
                    minSizeMB: 50,
                    progressCallback: nil,
                    shouldCancel: { lease.isCancelled }
                )
            }.value
            return await (cacheTask, largeTask)
        } onCancel: {
            lease.token.cancel()
        }
        guard activeLease?.id == lease.id,
              viewModel.shouldContinueCoordinatedOperation(lease) else {
            if activeLease?.id == lease.id {
                viewModel.toastMessage = "스마트 검사를 중단했습니다"
                isPresented = false
            }
            return
        }
        scanProgress = 0.4
        scanMessage = "대용량 파일 분석 중..."
        scanProgress = 0.6
        scanMessage = "중복 파일 분석 중..."

        let duplicates = await withTaskCancellationHandler {
            await Task.detached {
                engine.scanDuplicates(
                    scanURL: scanURL,
                    minSizeKB: 100,
                    progressCallback: nil,
                    shouldCancel: { lease.isCancelled }
                )
            }.value
        } onCancel: {
            lease.token.cancel()
        }
        guard activeLease?.id == lease.id,
              viewModel.shouldContinueCoordinatedOperation(lease) else {
            if activeLease?.id == lease.id {
                viewModel.toastMessage = "스마트 검사를 중단했습니다"
                isPresented = false
            }
            return
        }

        scanProgress = 0.9
        scanMessage = "안전성 분석 중..."

        viewModel.cacheItems = cacheItems
        viewModel.largeFiles = largeFiles
        viewModel.duplicateGroups = duplicates

        // 그룹 생성 (파일 레벨 데이터 포함)
        var result: [SmartCleanGroup] = []

        // 폴더 캐시는 하위 항목 전체 명세가 없어 Smart Clean에서 자동
        // 이동 후보로 내보내지 않는다. 개별 일반 파일만 안전 후보가 된다.
        let movableCacheItems = cacheItems.filter { $0.snapshot.kind == .regularFile }
        if !movableCacheItems.isEmpty {
            result.append(SmartCleanGroup(
                icon: "internaldrive.fill", name: "캐시/임시파일",
                safety: .safe, detail: "검토 당시와 같은 개별 캐시 파일만 이동합니다",
                files: movableCacheItems.map {
                    SmartCleanFile(name: $0.name, path: $0.path, size: $0.size)
                }
            ))
        }

        // 중복 (보관할 파일 제외 복사본)
        let dupeFiles = duplicates.flatMap { group in
            group.files.sorted { $0.path < $1.path }.dropFirst().map {
                SmartCleanFile(name: $0.name, path: $0.path, size: $0.size, isChecked: false)
            }
        }
        if !dupeFiles.isEmpty {
            result.append(SmartCleanGroup(
                icon: "doc.on.doc.fill", name: "중복 파일 (복사본)",
                safety: .review, detail: "전체 내용이 같은 복사본을 확인한 뒤 휴지통으로 이동합니다",
                files: dupeFiles
            ))
        }

        // 오래된 설치파일 (90일+)
        let oldInstallers = largeFiles
            .filter { ($0.category == .installer || $0.category == .archive) && $0.ageDays > 90 }
            .map { SmartCleanFile(name: $0.name, path: $0.path, size: $0.size, isChecked: false) }
        if !oldInstallers.isEmpty {
            result.append(SmartCleanGroup(
                icon: "arrow.down.circle.fill", name: "오래된 설치/압축파일",
                safety: .review, detail: "90일+ 수정되지 않은 설치·압축파일입니다. 필요한 원본인지 확인하세요",
                files: oldInstallers
            ))
        }

        // 6개월+ 수정 없음 대용량
        let oldLarge = largeFiles
            .filter { $0.category != .installer && $0.category != .archive && $0.ageDays > 180 }
            .map { SmartCleanFile(name: $0.name, path: $0.path, size: $0.size, isChecked: false) }
        if !oldLarge.isEmpty {
            result.append(SmartCleanGroup(
                icon: "doc.richtext.fill", name: "6개월+ 수정 없음 대용량",
                safety: .review, detail: "수정 시각 기준 참고 항목이며 중요할 수 있습니다",
                files: oldLarge
            ))
        }

        // 최근 대용량
        let recentLarge = largeFiles
            .filter { $0.ageDays <= 180 && $0.category != .installer && $0.category != .archive }
            .map { SmartCleanFile(name: $0.name, path: $0.path, size: $0.size, isChecked: false) }
        if !recentLarge.isEmpty {
            result.append(SmartCleanGroup(
                icon: "doc.fill", name: "최근 대용량 파일",
                safety: .caution, detail: "최근 파일이므로 직접 확인 필요",
                files: recentLarge
            ))
        }

        scanProgress = 1.0
        groups = result

        withAnimation(.spring(duration: 0.5, bounce: 0.2)) {
            phase = .result
        }
    }

    // MARK: - Perform Clean

    @MainActor
    private func performSmartClean() async {
        guard let lease = viewModel.beginCoordinatedOperation(
            message: "선택 항목을 휴지통으로 이동 중..."
        ) else { return }
        activeLease = lease
        defer {
            viewModel.finishCoordinatedOperation(lease)
            if activeLease?.id == lease.id { activeLease = nil }
        }
        withAnimation(.spring(duration: 0.3)) { phase = .cleaning }
        celebrateBounce += 1
        cleanErrors = []
        var operationErrors: [String] = []

        // 체크된 파일 경로만 수집
        var cachePaths: [CacheItem] = []
        var filePaths: Set<String> = []
        var duplicatePaths: Set<String> = []

        for group in groups {
            guard !lease.isCancelled else { break }
            let checkedFiles = group.files.filter(\.isChecked)
            if group.name.contains("캐시") {
                // 캐시는 CacheItem으로 매핑
                cachePaths = viewModel.cacheItems.filter { item in
                    checkedFiles.contains(where: { $0.path == item.path })
                }
            } else if group.name.contains("중복 파일") {
                duplicatePaths.formUnion(checkedFiles.map(\.path))
            } else {
                filePaths.formUnion(checkedFiles.map(\.path))
            }
        }

        let duplicateGroups = viewModel.duplicateGroups
        let protectedKeeperPaths: Set<String> = Set(duplicateGroups.compactMap { group -> String? in
            let ordered = group.files.sorted { $0.path < $1.path }
            guard ordered.dropFirst().contains(where: { duplicatePaths.contains($0.path) }) else {
                return nil
            }
            return ordered.first?.path
        })

        // 같은 파일이 다른 추천 그룹에도 보이면 중복 전용 최종 검증을 우선하고,
        // 해당 그룹의 보관할 파일은 이번 작업에서 반드시 남긴다.
        filePaths.subtract(duplicatePaths)
        filePaths.subtract(protectedKeeperPaths)
        let duplicateIDs = Set(duplicateGroups
            .flatMap(\.files)
            .filter { duplicatePaths.contains($0.path) }
            .map(\.id))

        var totalFreed: Int64 = 0

        if !cachePaths.isEmpty, !lease.isCancelled {
            let r = await withTaskCancellationHandler {
                await Task.detached {
                    CleanerEngine.shared.cleanCache(
                        items: cachePaths,
                        shouldCancel: { lease.isCancelled }
                    )
                }.value
            } onCancel: {
                lease.token.cancel()
            }
            totalFreed += r.freed
            operationErrors.append(contentsOf: r.errors)
        }

        if !filePaths.isEmpty, !lease.isCancelled {
            let largeTargets = viewModel.largeFiles.filter { filePaths.contains($0.path) }
            let r = await withTaskCancellationHandler {
                await Task.detached {
                    CleanerEngine.shared.trashVerifiedLargeFiles(
                        files: largeTargets,
                        shouldCancel: { lease.isCancelled }
                    )
                }.value
            } onCancel: {
                lease.token.cancel()
            }
            totalFreed += r.freed
            operationErrors.append(contentsOf: r.errors)
        }

        if !duplicateIDs.isEmpty, !lease.isCancelled {
            let r = await withTaskCancellationHandler {
                await Task.detached {
                    CleanerEngine.shared.trashVerifiedDuplicates(
                        groups: duplicateGroups,
                        selectedFileIDs: duplicateIDs,
                        shouldCancel: { lease.isCancelled }
                    )
                }.value
            } onCancel: {
                lease.token.cancel()
            }
            totalFreed += r.freed
            operationErrors.append(contentsOf: r.errors)
        }

        if lease.isCancelled { operationErrors.append("현재 항목 처리 후 중단했습니다") }
        if totalFreed > 0 {
            HealthMonitor.shared.recordClean()
            CleanHistory.shared.record(freed: totalFreed, type: "smart")
        }
        guard activeLease?.id == lease.id,
              viewModel.isCoordinatedOperationActive(lease) else { return }

        freedAmount = totalFreed
        cleanErrors = operationErrors
        if cleanErrors.isEmpty {
            viewModel.toastMessage = "휴지통으로 이동한 논리 용량: \(formatSize(totalFreed))"
        } else {
            viewModel.toastMessage = "휴지통으로 이동한 논리 용량: \(formatSize(totalFreed)), \(cleanErrors.count)개 실패"
        }

        withAnimation(.spring(duration: 0.5, bounce: 0.3)) {
            phase = .done
        }
    }
}

// MARK: - Smart Group Card (펼침 가능, 파일 리스트 포함)

private struct SmartGroupCard: View {
    @Binding var group: SmartCleanGroup
    let isExpanded: Bool
    let onToggleExpand: () -> Void

    private var allChecked: Bool {
        group.files.allSatisfy(\.isChecked)
    }

    var body: some View {
        VStack(spacing: 0) {
            // 그룹 헤더 (클릭하면 펼침)
            HStack(spacing: 12) {
                Image(systemName: group.icon)
                    .font(.title3)
                    .foregroundStyle(group.safety.color)
                    .frame(width: 24)

                VStack(alignment: .leading, spacing: 2) {
                    Text(group.name)
                        .font(.system(size: 13, weight: .semibold))
                        .lineLimit(1)
                    Text("\(group.checkedCount)/\(group.files.count)개 선택")
                        .font(.system(size: 11))
                        .foregroundStyle(.secondary)
                }

                Spacer(minLength: 8)

                Text(group.safety.rawValue)
                    .font(.system(size: 10, weight: .bold))
                    .padding(.horizontal, 8)
                    .padding(.vertical, 3)
                    .background(group.safety.color.opacity(0.15), in: Capsule())
                    .foregroundStyle(group.safety.color)
                    .fixedSize()

                Text(group.checkedSizeFormatted)
                    .font(.system(size: 13, weight: .bold, design: .rounded))
                    .monospacedDigit()
                    .foregroundStyle(group.safety.color)
                    .frame(width: 80, alignment: .trailing)

                Image(systemName: isExpanded ? "chevron.up" : "chevron.down")
                    .font(.caption2)
                    .foregroundStyle(.tertiary)
                    .frame(width: 16)
            }
            .padding(.horizontal, 14)
            .padding(.vertical, 12)
            .background(isExpanded ? group.safety.color.opacity(0.04) : .clear)
            .contentShape(Rectangle())
            .onTapGesture { onToggleExpand() }

            // 펼쳐진 파일 리스트
            if isExpanded {
                Divider()

                // 전체 선택/해제
                HStack {
                    Toggle("전체 선택", isOn: Binding(
                        get: { allChecked },
                        set: { selectAll in
                            for i in group.files.indices {
                                group.files[i].isChecked = selectAll
                            }
                        }
                    ))
                    .toggleStyle(.checkbox)
                    .font(.caption)
                    Spacer()
                }
                .padding(.horizontal, 14)
                .padding(.vertical, 8)
                .background(.secondary.opacity(0.04))

                // 파일 목록 (스크롤 가능, 높이 제한)
                ScrollView {
                    VStack(spacing: 0) {
                        ForEach($group.files) { $file in
                            SmartFileRow(file: $file)
                            if file.id != group.files.last?.id {
                                Divider().padding(.leading, 48)
                            }
                        }
                    }
                }
                .frame(maxHeight: 250)
            }
        }
        .background {
            RoundedRectangle(cornerRadius: 12)
                .fill(group.safety.color.opacity(0.03))
                .overlay(
                    RoundedRectangle(cornerRadius: 12)
                        .stroke(group.safety.color.opacity(isExpanded ? 0.25 : 0.1), lineWidth: 1)
                )
        }
    }
}

// MARK: - Smart File Row (체크박스 + 클릭하면 Finder 열기)

private struct SmartFileRow: View {
    @Binding var file: SmartCleanFile
    @State private var isHovering = false

    var body: some View {
        HStack(spacing: 12) {
            Toggle("", isOn: $file.isChecked)
                .toggleStyle(.checkbox)
                .labelsHidden()

            // 파일명 (클릭하면 Finder에서 보기)
            Button {
                revealInFinder(path: file.path)
            } label: {
                HStack(spacing: 6) {
                    Image(systemName: "doc")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .frame(width: 14)
                    Text(file.name)
                        .font(.system(size: 12))
                        .lineLimit(1)
                        .truncationMode(.middle)
                        .underline(isHovering)
                        .foregroundStyle(isHovering ? .blue : .primary)
                }
            }
            .buttonStyle(.plain)
            .frame(maxWidth: .infinity, alignment: .leading)
            .onHover { hovering in
                isHovering = hovering
                if hovering { NSCursor.pointingHand.push() }
                else { NSCursor.pop() }
            }
            .help("Finder에서 보기: \(file.path)")

            Text(file.sizeFormatted)
                .font(.system(size: 11, weight: .medium, design: .rounded))
                .monospacedDigit()
                .foregroundStyle(.secondary)
                .frame(width: 70, alignment: .trailing)
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 7)
        .background(file.isChecked ? Color.clear : Color.secondary.opacity(0.04))
        .opacity(file.isChecked ? 1.0 : 0.6)
    }

    private func revealInFinder(path: String) {
        let url = URL(fileURLWithPath: path)
        NSWorkspace.shared.activateFileViewerSelecting([url])
    }
}
