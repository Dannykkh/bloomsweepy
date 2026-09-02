import SwiftUI

struct SettingsView: View {
    // 스캔 설정
    @AppStorage("minLargeFileMB") private var minLargeFileMB = 50
    @AppStorage("excludedPaths") private var excludedPathsRaw = ""
    @State private var newExcludePath = ""
    @AppStorage("autoCleanEnabled") private var autoCleanEnabled = false
    @AppStorage("autoCleanInterval") private var autoCleanInterval = 7 // days
    @AppStorage("showMenuBarPercent") private var showMenuBarPercent = true
    @AppStorage("notificationsEnabled") private var notificationsEnabled = true

    // 정리 이력
    @State private var history = CleanHistory.shared

    var body: some View {
        ScrollView {
            VStack(spacing: 24) {
                // 스캔 설정
                settingsSection("스캔 설정", icon: "magnifyingglass") {
                    HStack {
                        Text("대용량 파일 기준")
                        Spacer()
                        Picker("", selection: $minLargeFileMB) {
                            Text("50MB 이상").tag(50)
                            Text("100MB 이상").tag(100)
                            Text("500MB 이상").tag(500)
                        }
                        .frame(width: 150)
                    }
                }

                // 예외 폴더
                settingsSection("스캔 제외 폴더", icon: "folder.badge.minus") {
                    Text("아래 경로는 스캔에서 제외됩니다")
                        .font(.caption)
                        .foregroundStyle(.secondary)

                    let paths = excludedPathsRaw.split(separator: "\n").map(String.init)
                    ForEach(paths, id: \.self) { path in
                        HStack {
                            Image(systemName: "folder")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                            Text(path)
                                .font(.system(size: 12))
                                .lineLimit(1)
                                .truncationMode(.middle)
                            Spacer()
                            Button {
                                excludedPathsRaw = excludedPathsRaw
                                    .split(separator: "\n")
                                    .filter { String($0) != path }
                                    .joined(separator: "\n")
                            } label: {
                                Image(systemName: "xmark.circle.fill")
                                    .font(.caption)
                                    .foregroundStyle(.red.opacity(0.7))
                            }
                            .buttonStyle(.plain)
                        }
                    }

                    HStack {
                        Button("폴더 추가") {
                            let panel = NSOpenPanel()
                            panel.canChooseFiles = false
                            panel.canChooseDirectories = true
                            panel.allowsMultipleSelection = false
                            panel.message = "스캔에서 제외할 폴더를 선택하세요"
                            if panel.runModal() == .OK, let url = panel.url {
                                if excludedPathsRaw.isEmpty {
                                    excludedPathsRaw = url.path
                                } else {
                                    excludedPathsRaw += "\n" + url.path
                                }
                            }
                        }
                        .buttonStyle(.bordered)
                        Spacer()
                    }
                }

                // 정리 알림
                settingsSection("정리 알림", icon: "clock.badge.checkmark") {
                    Toggle("주기적으로 정리 시점 알림", isOn: $autoCleanEnabled)
                        .onChange(of: autoCleanEnabled) { _, _ in
                            HealthMonitor.shared.startScheduleIfEnabled()
                        }
                    if autoCleanEnabled {
                        HStack {
                            Text("알림 주기")
                            Spacer()
                            Picker("", selection: $autoCleanInterval) {
                                Text("3일마다").tag(3)
                                Text("7일마다").tag(7)
                                Text("14일마다").tag(14)
                                Text("30일마다").tag(30)
                            }
                            .frame(width: 130)
                            .onChange(of: autoCleanInterval) { _, _ in
                                HealthMonitor.shared.startScheduleIfEnabled()
                            }
                        }
                    }
                    Text("파일을 자동으로 삭제하지 않고, 정리할 시점만 알려드립니다.")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    Toggle("macOS 알림 사용", isOn: $notificationsEnabled)
                        .onChange(of: notificationsEnabled) { _, newVal in
                            HealthMonitor.shared.startScheduleIfEnabled(
                                requestPermissionIfNeeded: newVal
                            )
                        }
                }

                // 메뉴바
                settingsSection("메뉴바", icon: "menubar.rectangle") {
                    Toggle("메모리 사용량 표시", isOn: $showMenuBarPercent)
                }

                // 정리 이력
                settingsSection("정리 이력", icon: "clock.arrow.circlepath") {
                    HStack {
                        VStack(alignment: .leading, spacing: 2) {
                            Text("총 휴지통 이동 논리 용량")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                            Text(history.totalMovedFormatted)
                                .font(.title3.bold())
                                .foregroundStyle(.green)
                        }
                        Spacer()
                        VStack(alignment: .trailing, spacing: 2) {
                            Text("이번 달")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                            Text(formatSize(history.monthlyFreed))
                                .font(.title3.bold())
                                .foregroundStyle(.blue)
                        }
                    }

                    if !history.records.isEmpty {
                        Divider()
                        ForEach(history.records.prefix(10)) { record in
                            HStack {
                                Image(systemName: "checkmark.circle.fill")
                                    .font(.caption)
                                    .foregroundStyle(.green)
                                Text(record.dateFormatted)
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                                Spacer()
                                Text(record.movedFormatted)
                                    .font(.caption.bold())
                                    .foregroundStyle(.green)
                            }
                        }
                    }
                }

                // 앱 정보
                settingsSection("앱 정보", icon: "info.circle") {
                    HStack {
                        Text("버전")
                        Spacer()
                        Text("1.3.0")
                            .foregroundStyle(.secondary)
                    }
                    HStack {
                        Text("빌드")
                        Spacer()
                        Text("3")
                            .foregroundStyle(.secondary)
                    }
                    HStack {
                        Text("macOS 요구사항")
                        Spacer()
                        Text("15.0+")
                            .foregroundStyle(.secondary)
                    }
                }
            }
            .padding(32)
        }
    }

    private func settingsSection<Content: View>(_ title: String, icon: String, @ViewBuilder content: () -> Content) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            Label(title, systemImage: icon)
                .font(.headline)

            VStack(spacing: 10) {
                content()
            }
            .padding(16)
            .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: 12))
        }
    }
}
