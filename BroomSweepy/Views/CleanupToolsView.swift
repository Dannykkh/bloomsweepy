import SwiftUI
import AppKit

struct CleanupToolsView: View {
    @State private var selectedTab: CleanupTab = .similarImages

    enum CleanupTab: String, CaseIterable, Identifiable {
        case similarImages = "유사 이미지"
        case mailAttachments = "메일 첨부파일"
        case brokenDownloads = "깨진 다운로드"
        case languageFiles = "언어 파일"
        case brokenPlists = "깨진 설정"
        case appVersions = "앱 버전"
        var id: String { rawValue }
    }

    var body: some View {
        VStack(spacing: 0) {
            // Tab Bar
            HStack(spacing: 0) {
                ForEach(CleanupTab.allCases) { tab in
                    Button {
                        withAnimation(.easeInOut(duration: 0.2)) {
                            selectedTab = tab
                        }
                    } label: {
                        VStack(spacing: 6) {
                            Image(systemName: tabIcon(tab))
                                .font(.title3)
                            Text(tab.rawValue)
                                .font(.caption.bold())
                        }
                        .foregroundColor(selectedTab == tab ? .accentColor : .secondary)
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 12)
                        .background(
                            selectedTab == tab
                                ? Color.accentColor.opacity(0.1)
                                : Color.clear,
                            in: RoundedRectangle(cornerRadius: 8)
                        )
                    }
                    .buttonStyle(.plain)
                }
            }
            .padding(.horizontal, 24)
            .padding(.top, 16)
            .padding(.bottom, 8)

            Divider()

            // Content
            switch selectedTab {
            case .similarImages:
                SimilarImagesView()
            case .mailAttachments:
                MailAttachmentsSection()
            case .brokenDownloads:
                BrokenDownloadsSection()
            case .languageFiles:
                LanguageCleanerSection()
            case .brokenPlists:
                BrokenPlistSection()
            case .appVersions:
                AppVersionsSection()
            }
        }
    }

    private func tabIcon(_ tab: CleanupTab) -> String {
        switch tab {
        case .similarImages: return "photo.on.rectangle.angled"
        case .mailAttachments: return "paperclip"
        case .brokenDownloads: return "arrow.down.circle.dotted"
        case .languageFiles: return "globe"
        case .brokenPlists: return "doc.badge.ellipsis"
        case .appVersions: return "app.badge.checkmark"
        }
    }
}

// MARK: - Mail Attachments Section

struct MailAttachmentsSection: View {
    @State private var attachments: [MailAttachment] = []
    @State private var selectedIDs: Set<UUID> = []
    @State private var isScanning = false
    @State private var showConfirm = false
    @State private var toastMessage: String?

    private var totalSize: Int64 {
        attachments.reduce(Int64(0)) { $0 + $1.size }
    }

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                VStack(alignment: .leading, spacing: 2) {
                    Text("메일 첨부파일")
                        .font(.title2.bold())
                    if !attachments.isEmpty {
                        Text("\(attachments.count)개 파일 · 총 \(formatSize(totalSize))")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                }
                Spacer()
                Button("스캔") {
                    Task { await scan() }
                }
                .buttonStyle(.bordered)
                .disabled(isScanning)

                Button("선택 항목 삭제") { showConfirm = true }
                    .buttonStyle(.borderedProminent)
                    .tint(.red)
                    .disabled(selectedIDs.isEmpty)
            }
            .padding(24)

            // Content
            if isScanning {
                ScanningPlaceholder()
            } else if attachments.isEmpty {
                EmptyPlaceholder(icon: "paperclip", message: "'스캔' 버튼을 눌러 메일 첨부파일을 탐색하세요")
            } else {
                List(attachments) { attachment in
                    MailAttachmentRow(
                        attachment: attachment,
                        isSelected: selectedIDs.contains(attachment.id)
                    ) {
                        if selectedIDs.contains(attachment.id) {
                            selectedIDs.remove(attachment.id)
                        } else {
                            selectedIDs.insert(attachment.id)
                        }
                    }
                }
                .listStyle(.inset(alternatesRowBackgrounds: true))
            }
        }
        .overlay(alignment: .bottom) {
            if let msg = toastMessage {
                ToastBanner(message: msg)
                    .onAppear {
                        DispatchQueue.main.asyncAfter(deadline: .now() + 3) {
                            withAnimation { toastMessage = nil }
                        }
                    }
            }
        }
        .alert("메일 첨부파일 삭제", isPresented: $showConfirm) {
            Button("취소", role: .cancel) {}
            Button("휴지통으로 이동", role: .destructive) { deleteSelected() }
        } message: {
            Text("\(selectedIDs.count)개 파일을 휴지통으로 이동하시겠습니까?")
        }
    }

    @MainActor
    private func scan() async {
        isScanning = true
        selectedIDs.removeAll()
        let homeURL = FileAccessManager.shared.loadBookmark()
        attachments = await Task.detached {
            MailAttachmentCleaner.shared.scan(homeURL: homeURL)
        }.value
        isScanning = false
        toastMessage = attachments.isEmpty
            ? "메일 첨부파일을 찾지 못했습니다"
            : "\(attachments.count)개 메일 첨부파일 발견"
    }

    private func deleteSelected() {
        let paths = attachments.filter { selectedIDs.contains($0.id) }.map(\.path)
        Task {
            let result = await Task.detached {
                MailAttachmentCleaner.shared.clean(paths: paths)
            }.value
            selectedIDs.removeAll()
            attachments.removeAll { att in paths.contains(att.path) }
            toastMessage = "삭제 완료! \(formatSize(result.freed)) 확보"
        }
    }
}

// MARK: - Mail Attachment Row

struct MailAttachmentRow: View {
    let attachment: MailAttachment
    let isSelected: Bool
    let onToggle: () -> Void

    var body: some View {
        HStack(spacing: 12) {
            Toggle("", isOn: Binding(get: { isSelected }, set: { _ in onToggle() }))
                .toggleStyle(.checkbox)
                .labelsHidden()

            Image(systemName: "paperclip")
                .font(.title2)
                .foregroundColor(.blue)
                .frame(width: 32)

            VStack(alignment: .leading, spacing: 3) {
                Text(attachment.name)
                    .font(.headline)
                    .lineLimit(1)
                Text("\(attachment.ageDays)일 전 · \(shortenPath(attachment.path))")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                    .truncationMode(.middle)
            }

            Spacer()

            Text(attachment.sizeFormatted)
                .font(.headline)
                .foregroundColor(.blue)
                .frame(minWidth: 80, alignment: .trailing)
        }
        .padding(.vertical, 4)
        .contentShape(Rectangle())
        .onTapGesture { onToggle() }
    }

    private func shortenPath(_ path: String) -> String {
        let home = NSHomeDirectory()
        return path.hasPrefix(home) ? "~" + path.dropFirst(home.count) : path
    }
}

// MARK: - Broken Downloads Section

struct BrokenDownloadsSection: View {
    @State private var downloads: [BrokenDownload] = []
    @State private var selectedIDs: Set<UUID> = []
    @State private var isScanning = false
    @State private var showConfirm = false
    @State private var toastMessage: String?

    private var totalSize: Int64 {
        downloads.reduce(Int64(0)) { $0 + $1.size }
    }

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                VStack(alignment: .leading, spacing: 2) {
                    Text("깨진 다운로드")
                        .font(.title2.bold())
                    if !downloads.isEmpty {
                        Text("\(downloads.count)개 파일 · 총 \(formatSize(totalSize))")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                }
                Spacer()
                Button("스캔") {
                    Task { await scan() }
                }
                .buttonStyle(.bordered)
                .disabled(isScanning)

                Button("선택 항목 삭제") { showConfirm = true }
                    .buttonStyle(.borderedProminent)
                    .tint(.red)
                    .disabled(selectedIDs.isEmpty)
            }
            .padding(24)

            // Content
            if isScanning {
                ScanningPlaceholder()
            } else if downloads.isEmpty {
                EmptyPlaceholder(icon: "arrow.down.circle.dotted", message: "'스캔' 버튼을 눌러 깨진 다운로드를 탐색하세요")
            } else {
                List(downloads) { download in
                    BrokenDownloadRow(
                        download: download,
                        isSelected: selectedIDs.contains(download.id)
                    ) {
                        if selectedIDs.contains(download.id) {
                            selectedIDs.remove(download.id)
                        } else {
                            selectedIDs.insert(download.id)
                        }
                    }
                }
                .listStyle(.inset(alternatesRowBackgrounds: true))
            }
        }
        .overlay(alignment: .bottom) {
            if let msg = toastMessage {
                ToastBanner(message: msg)
                    .onAppear {
                        DispatchQueue.main.asyncAfter(deadline: .now() + 3) {
                            withAnimation { toastMessage = nil }
                        }
                    }
            }
        }
        .alert("깨진 다운로드 삭제", isPresented: $showConfirm) {
            Button("취소", role: .cancel) {}
            Button("휴지통으로 이동", role: .destructive) { deleteSelected() }
        } message: {
            Text("\(selectedIDs.count)개 파일을 휴지통으로 이동하시겠습니까?")
        }
    }

    @MainActor
    private func scan() async {
        isScanning = true
        selectedIDs.removeAll()
        let homeURL = FileAccessManager.shared.loadBookmark()
        downloads = await Task.detached {
            BrokenDownloadCleaner.shared.scan(homeURL: homeURL)
        }.value
        isScanning = false
        toastMessage = downloads.isEmpty
            ? "깨진 다운로드를 찾지 못했습니다"
            : "\(downloads.count)개 깨진 다운로드 발견"
    }

    private func deleteSelected() {
        let paths = downloads.filter { selectedIDs.contains($0.id) }.map(\.path)
        Task {
            let result = await Task.detached {
                BrokenDownloadCleaner.shared.clean(paths: paths)
            }.value
            selectedIDs.removeAll()
            downloads.removeAll { dl in paths.contains(dl.path) }
            toastMessage = "삭제 완료! \(formatSize(result.freed)) 확보"
        }
    }
}

// MARK: - Broken Download Row

struct BrokenDownloadRow: View {
    let download: BrokenDownload
    let isSelected: Bool
    let onToggle: () -> Void

    var body: some View {
        HStack(spacing: 12) {
            Toggle("", isOn: Binding(get: { isSelected }, set: { _ in onToggle() }))
                .toggleStyle(.checkbox)
                .labelsHidden()

            Image(systemName: reasonIcon)
                .font(.title2)
                .foregroundColor(.orange)
                .frame(width: 32)

            VStack(alignment: .leading, spacing: 3) {
                Text(download.name)
                    .font(.headline)
                    .lineLimit(1)
                Text(download.reason.rawValue)
                    .font(.caption)
                    .padding(.horizontal, 6)
                    .padding(.vertical, 2)
                    .background(.orange.opacity(0.15), in: Capsule())
                    .foregroundColor(.orange)
            }

            Spacer()

            Text(download.sizeFormatted)
                .font(.headline)
                .foregroundColor(.orange)
                .frame(minWidth: 80, alignment: .trailing)
        }
        .padding(.vertical, 4)
        .contentShape(Rectangle())
        .onTapGesture { onToggle() }
    }

    private var reasonIcon: String {
        switch download.reason {
        case .incompleteDownload: return "arrow.down.circle.dotted"
        case .resourceFork: return "doc.badge.gearshape"
        case .zeroBytes: return "doc.text.magnifyingglass"
        }
    }
}

// MARK: - App Versions Section

struct AppVersionsSection: View {
    @State private var apps: [AppVersionInfo] = []
    @State private var isScanning = false
    @State private var searchText = ""

    private var filteredApps: [AppVersionInfo] {
        if searchText.isEmpty { return apps }
        return apps.filter {
            $0.name.localizedCaseInsensitiveContains(searchText) ||
            $0.bundleIdentifier.localizedCaseInsensitiveContains(searchText)
        }
    }

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Text("앱 버전 정보")
                    .font(.title2.bold())
                Spacer()

                if !apps.isEmpty {
                    TextField("검색", text: $searchText)
                        .textFieldStyle(.roundedBorder)
                        .frame(maxWidth: 200)
                }

                Button("스캔") {
                    Task { await scan() }
                }
                .buttonStyle(.bordered)
                .disabled(isScanning)
            }
            .padding(24)

            // Content
            if isScanning {
                ScanningPlaceholder()
            } else if apps.isEmpty {
                EmptyPlaceholder(icon: "app.badge.checkmark", message: "'스캔' 버튼을 눌러 설치된 앱 버전을 확인하세요")
            } else {
                List(filteredApps) { app in
                    AppVersionRow(app: app)
                }
                .listStyle(.inset(alternatesRowBackgrounds: true))
            }
        }
    }

    @MainActor
    private func scan() async {
        isScanning = true
        apps = await Task.detached {
            AppVersionChecker.shared.scanInstalledApps()
        }.value
        isScanning = false
    }
}

// MARK: - App Version Row

struct AppVersionRow: View {
    let app: AppVersionInfo

    var body: some View {
        HStack(spacing: 12) {
            Image(nsImage: app.icon ?? NSImage())
                .resizable()
                .interpolation(.high)
                .frame(width: 32, height: 32)

            VStack(alignment: .leading, spacing: 3) {
                Text(app.name)
                    .font(.headline)
                Text(app.bundleIdentifier)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }

            Spacer()

            VStack(alignment: .trailing, spacing: 3) {
                HStack(spacing: 4) {
                    Text("v\(app.version)")
                        .font(.callout.bold())
                        .foregroundColor(.accentColor)
                    if !app.buildNumber.isEmpty {
                        Text("(\(app.buildNumber))")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                }
                Text(app.sizeFormatted)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            .frame(minWidth: 120, alignment: .trailing)
        }
        .padding(.vertical, 4)
    }
}

// MARK: - Language Cleaner Section

struct LanguageCleanerSection: View {
    @State private var resources: [LanguageCleaner.LanguageResource] = []
    @State private var selectedIDs: Set<UUID> = []
    @State private var isScanning = false
    @State private var showConfirm = false
    @State private var toastMessage: String?
    @State private var scanProgress: Double = 0
    @State private var scanMessage = ""

    private var totalSize: Int64 {
        resources.reduce(Int64(0)) { $0 + $1.size }
    }

    private var selectedSize: Int64 {
        resources.filter { selectedIDs.contains($0.id) }.reduce(Int64(0)) { $0 + $1.size }
    }

    private var groupedByApp: [(app: String, items: [LanguageCleaner.LanguageResource])] {
        let grouped = Dictionary(grouping: resources) { $0.appName }
        return grouped.map { (app: $0.key, items: $0.value.sorted { $0.size > $1.size }) }
            .sorted { $0.items.reduce(0) { $0 + $1.size } > $1.items.reduce(0) { $0 + $1.size } }
    }

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                VStack(alignment: .leading, spacing: 2) {
                    Text("불필요한 언어 파일")
                        .font(.title2.bold())
                    if !resources.isEmpty {
                        Text("\(resources.count)개 파일 · 총 \(formatSize(totalSize)) · \(groupedByApp.count)개 앱")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                }
                Spacer()
                if !resources.isEmpty {
                    Button("전체 선택") {
                        if selectedIDs.count == resources.count {
                            selectedIDs.removeAll()
                        } else {
                            selectedIDs = Set(resources.map(\.id))
                        }
                    }
                    .buttonStyle(.bordered)
                }
                Button("스캔") { Task { await scan() } }
                    .buttonStyle(.bordered)
                    .disabled(isScanning)
                Button("선택 항목 삭제 (\(formatSize(selectedSize)))") { showConfirm = true }
                    .buttonStyle(.borderedProminent)
                    .tint(.red)
                    .disabled(selectedIDs.isEmpty)
            }
            .padding(24)

            if isScanning {
                VStack(spacing: 16) {
                    ProgressView()
                        .scaleEffect(1.5)
                        .progressViewStyle(.circular)
                    Text(scanMessage.isEmpty ? "스캔 중..." : scanMessage)
                        .font(.headline)
                        .foregroundStyle(.secondary)
                    if scanProgress > 0 {
                        ProgressView(value: scanProgress)
                            .frame(maxWidth: 300)
                    }
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else if resources.isEmpty {
                VStack(spacing: 20) {
                    Image(systemName: "globe")
                        .font(.system(size: 56))
                        .symbolRenderingMode(.hierarchical)
                        .foregroundStyle(.blue)
                        .symbolEffect(.bounce, value: isScanning)
                    VStack(spacing: 6) {
                        Text("언어 파일 정리")
                            .font(.title3.bold())
                        Text("한국어/영어 외의 불필요한 언어 리소스를 제거합니다")
                            .font(.callout)
                            .foregroundStyle(.secondary)
                    }
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                List(groupedByApp, id: \.app) { group in
                    Section {
                        ForEach(group.items) { item in
                            HStack(spacing: 12) {
                                Toggle("", isOn: Binding(
                                    get: { selectedIDs.contains(item.id) },
                                    set: { sel in
                                        if sel { selectedIDs.insert(item.id) }
                                        else { selectedIDs.remove(item.id) }
                                    }
                                ))
                                .toggleStyle(.checkbox)
                                .labelsHidden()

                                Image(systemName: "globe")
                                    .foregroundStyle(.blue)
                                    .frame(width: 20)

                                Text(item.language)
                                    .font(.callout)

                                Spacer()

                                Text(item.sizeFormatted)
                                    .font(.callout.bold())
                                    .foregroundStyle(.blue)
                                    .frame(minWidth: 60, alignment: .trailing)
                            }
                            .padding(.vertical, 2)
                            .contentShape(Rectangle())
                            .onTapGesture {
                                if selectedIDs.contains(item.id) { selectedIDs.remove(item.id) }
                                else { selectedIDs.insert(item.id) }
                            }
                        }
                    } header: {
                        HStack {
                            Text(group.app)
                                .font(.headline)
                            Spacer()
                            Text(formatSize(group.items.reduce(0) { $0 + $1.size }))
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                    }
                }
                .listStyle(.inset(alternatesRowBackgrounds: true))
            }
        }
        .overlay(alignment: .bottom) {
            if let msg = toastMessage {
                ToastBanner(message: msg)
                    .onAppear {
                        DispatchQueue.main.asyncAfter(deadline: .now() + 3) {
                            withAnimation { toastMessage = nil }
                        }
                    }
            }
        }
        .alert("언어 파일 삭제", isPresented: $showConfirm) {
            Button("취소", role: .cancel) {}
            Button("삭제", role: .destructive) { deleteSelected() }
        } message: {
            Text("\(selectedIDs.count)개 언어 파일을 삭제하시겠습니까?\n(\(formatSize(selectedSize)))\n\n한국어·영어는 보존됩니다.")
        }
    }

    @MainActor
    private func scan() async {
        isScanning = true
        selectedIDs.removeAll()
        resources = await Task.detached {
            LanguageCleaner.shared.scan { msg, progress in
                Task { @MainActor in
                    scanMessage = msg
                    scanProgress = progress
                }
            }
        }.value
        isScanning = false
        toastMessage = resources.isEmpty
            ? "불필요한 언어 파일이 없습니다"
            : "\(resources.count)개 언어 파일 발견 (\(formatSize(totalSize)))"
    }

    private func deleteSelected() {
        let targets = resources.filter { selectedIDs.contains($0.id) }
        Task {
            let result = await Task.detached {
                LanguageCleaner.shared.clean(resources: targets)
            }.value
            selectedIDs.removeAll()
            resources.removeAll { r in targets.contains(where: { $0.id == r.id }) }
            toastMessage = "삭제 완료! \(formatSize(result.freed)) 확보"
        }
    }
}

// MARK: - Broken Plist Section

struct BrokenPlistSection: View {
    @State private var plists: [BrokenPlistCleaner.BrokenPlist] = []
    @State private var selectedIDs: Set<UUID> = []
    @State private var isScanning = false
    @State private var showConfirm = false
    @State private var toastMessage: String?

    private var totalSize: Int64 {
        plists.reduce(Int64(0)) { $0 + $1.size }
    }

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                VStack(alignment: .leading, spacing: 2) {
                    Text("깨진 설정 파일")
                        .font(.title2.bold())
                    if !plists.isEmpty {
                        Text("\(plists.count)개 파일 · 총 \(formatSize(totalSize))")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                }
                Spacer()
                Button("스캔") { Task { await scan() } }
                    .buttonStyle(.bordered)
                    .disabled(isScanning)
                Button("선택 항목 삭제") { showConfirm = true }
                    .buttonStyle(.borderedProminent)
                    .tint(.red)
                    .disabled(selectedIDs.isEmpty)
            }
            .padding(24)

            if isScanning {
                ScanningPlaceholder()
            } else if plists.isEmpty {
                VStack(spacing: 20) {
                    Image(systemName: "doc.badge.ellipsis")
                        .font(.system(size: 56))
                        .symbolRenderingMode(.hierarchical)
                        .foregroundStyle(.orange)
                        .symbolEffect(.bounce, value: isScanning)
                    VStack(spacing: 6) {
                        Text("설정 파일 검사")
                            .font(.title3.bold())
                        Text("파싱 불가하거나 삭제된 앱의 고아 plist를 찾습니다")
                            .font(.callout)
                            .foregroundStyle(.secondary)
                    }
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                List(plists) { plist in
                    HStack(spacing: 12) {
                        Toggle("", isOn: Binding(
                            get: { selectedIDs.contains(plist.id) },
                            set: { sel in
                                if sel { selectedIDs.insert(plist.id) }
                                else { selectedIDs.remove(plist.id) }
                            }
                        ))
                        .toggleStyle(.checkbox)
                        .labelsHidden()

                        Image(systemName: plist.reason == .parseError ? "exclamationmark.triangle.fill" : "questionmark.circle.fill")
                            .font(.title3)
                            .foregroundColor(plist.reason == .parseError ? .red : .orange)
                            .frame(width: 24)

                        VStack(alignment: .leading, spacing: 3) {
                            Text(plist.name)
                                .font(.callout.weight(.medium))
                                .lineLimit(1)
                            Text(plist.reason.rawValue)
                                .font(.system(size: 10, weight: .bold))
                                .padding(.horizontal, 6)
                                .padding(.vertical, 2)
                                .background(
                                    (plist.reason == .parseError ? Color.red : Color.orange).opacity(0.15),
                                    in: Capsule()
                                )
                                .foregroundStyle(plist.reason == .parseError ? .red : .orange)
                        }

                        Spacer()

                        Text(plist.sizeFormatted)
                            .font(.callout.bold())
                            .foregroundColor(.orange)
                            .frame(minWidth: 60, alignment: .trailing)
                    }
                    .padding(.vertical, 2)
                    .contentShape(Rectangle())
                    .onTapGesture {
                        if selectedIDs.contains(plist.id) { selectedIDs.remove(plist.id) }
                        else { selectedIDs.insert(plist.id) }
                    }
                }
                .listStyle(.inset(alternatesRowBackgrounds: true))
            }
        }
        .overlay(alignment: .bottom) {
            if let msg = toastMessage {
                ToastBanner(message: msg)
                    .onAppear {
                        DispatchQueue.main.asyncAfter(deadline: .now() + 3) {
                            withAnimation { toastMessage = nil }
                        }
                    }
            }
        }
        .alert("설정 파일 삭제", isPresented: $showConfirm) {
            Button("취소", role: .cancel) {}
            Button("삭제", role: .destructive) { deleteSelected() }
        } message: {
            Text("\(selectedIDs.count)개 파일을 삭제하시겠습니까?")
        }
    }

    @MainActor
    private func scan() async {
        isScanning = true
        selectedIDs.removeAll()
        let homeURL = FileAccessManager.shared.loadBookmark()
        plists = await Task.detached {
            BrokenPlistCleaner.shared.scan(homeURL: homeURL, progressCallback: nil)
        }.value
        isScanning = false
        toastMessage = plists.isEmpty
            ? "깨진 설정 파일이 없습니다"
            : "\(plists.count)개 깨진 설정 파일 발견"
    }

    private func deleteSelected() {
        let targets = plists.filter { selectedIDs.contains($0.id) }
        Task {
            let result = await Task.detached {
                BrokenPlistCleaner.shared.clean(plists: targets)
            }.value
            selectedIDs.removeAll()
            plists.removeAll { p in targets.contains(where: { $0.id == p.id }) }
            toastMessage = "삭제 완료! \(formatSize(result.freed)) 확보"
        }
    }
}

// MARK: - Shared Helpers

private struct ScanningPlaceholder: View {
    var body: some View {
        VStack(spacing: 16) {
            ProgressView()
                .scaleEffect(1.5)
                .progressViewStyle(.circular)
            Text("스캔 중...")
                .font(.headline)
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .transition(.opacity)
    }
}

private struct EmptyPlaceholder: View {
    let icon: String
    let message: String

    var body: some View {
        VStack(spacing: 12) {
            Image(systemName: icon)
                .font(.system(size: 40))
                .foregroundStyle(.secondary)
            Text(message)
                .multilineTextAlignment(.center)
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

private struct ToastBanner: View {
    let message: String

    var body: some View {
        Text(message)
            .font(.callout.bold())
            .padding(.horizontal, 24)
            .padding(.vertical, 12)
            .background(.green, in: Capsule())
            .foregroundStyle(.white)
            .padding(.bottom, 24)
            .transition(.move(edge: .bottom).combined(with: .opacity))
    }
}
