import SwiftUI

struct FileOrganizerView: View {
    @Bindable var viewModel: CleanerViewModel
    @State private var options = OrganizeOptions()
    @State private var preview: [OrganizePlan] = []
    @State private var showConfirm = false
    @State private var lastExecuted: [OrganizePlan] = []
    @State private var resultMessage = ""

    private let engine = FileOrganizerEngine.shared

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Text("파일 정리")
                    .font(.title2.bold())
                Spacer()
                if !lastExecuted.isEmpty {
                    Button("되돌리기") { undoOrganize() }
                        .buttonStyle(.bordered)
                }
                Button("정리 실행") { showConfirm = true }
                    .buttonStyle(.borderedProminent)
                    .disabled(preview.isEmpty)
            }
            .padding(24)


            // Folder Selection + Options
            VStack(spacing: 16) {
                // Folder picker
                HStack {
                    Image(systemName: "folder.badge.gearshape")
                        .font(.title2)
                        .foregroundColor(.accentColor)

                    if let url = viewModel.organizerTargetURL {
                        Text(url.path)
                            .font(.callout)
                            .lineLimit(1)
                            .truncationMode(.middle)
                    } else {
                        Text("정리할 폴더를 선택하세요")
                            .foregroundStyle(.secondary)
                    }

                    Spacer()

                    Button("폴더 선택") { selectFolder() }
                        .buttonStyle(.bordered)
                }
                .padding(16)
                .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: 12))

                // Options
                VStack(alignment: .leading, spacing: 12) {
                    Text("정리 규칙")
                        .font(.headline)

                    Toggle("날짜 접두어 추가 (YYYY-MM-DD_파일명)", isOn: $options.addDatePrefix)
                    Toggle("확장자별 폴더 분류", isOn: $options.sortByType)
                    Toggle("사진 날짜별 분류 (EXIF)", isOn: $options.sortPhotosByDate)
                    Toggle("스크린샷 분류", isOn: $options.sortScreenshots)
                }
                .padding(16)
                .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: 12))
                .onChange(of: options.addDatePrefix) { refreshPreview() }
                .onChange(of: options.sortByType) { refreshPreview() }
                .onChange(of: options.sortPhotosByDate) { refreshPreview() }
                .onChange(of: options.sortScreenshots) { refreshPreview() }
            }
            .padding(.horizontal, 24)

            // Preview
            if !preview.isEmpty {
                VStack(alignment: .leading, spacing: 8) {
                    Text("미리보기 (\(preview.count)개 변경)")
                        .font(.headline)
                        .padding(.horizontal, 24)
                        .padding(.top, 16)

                    List(preview) { plan in
                        VStack(alignment: .leading, spacing: 4) {
                            HStack {
                                Image(systemName: "doc")
                                    .foregroundStyle(.secondary)
                                Text(plan.originalURL.lastPathComponent)
                                    .font(.callout)
                            }
                            HStack {
                                Image(systemName: "arrow.right")
                                    .foregroundColor(.accentColor)
                                Text(shortenDest(plan.destinationURL))
                                    .font(.caption)
                                    .foregroundColor(.accentColor)
                            }
                        }
                        .padding(.vertical, 2)
                    }
                    .listStyle(.inset(alternatesRowBackgrounds: true))
                }
            } else if viewModel.organizerTargetURL != nil {
                VStack(spacing: 12) {
                    Image(systemName: "checkmark.circle")
                        .font(.system(size: 40))
                        .foregroundStyle(.green)
                    Text("변경할 파일이 없습니다")
                        .foregroundStyle(.secondary)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                Spacer()
            }

            if !resultMessage.isEmpty {
                Text(resultMessage)
                    .font(.callout.bold())
                    .foregroundStyle(.green)
                    .padding()
            }
        }
        .alert("파일 정리 실행", isPresented: $showConfirm) {
            Button("취소", role: .cancel) {}
            Button("실행") { executeOrganize() }
        } message: {
            Text("\(preview.count)개 파일을 정리하시겠습니까?")
        }
    }

    private func selectFolder() {
        if let url = FileAccessManager.shared.requestFolderAccess(message: "정리할 폴더를 선택하세요") {
            viewModel.organizerTargetURL = url
            refreshPreview()
        }
    }

    private func refreshPreview() {
        guard let url = viewModel.organizerTargetURL else { return }
        preview = engine.preview(folderURL: url, options: options)
    }

    private func executeOrganize() {
        var plans = preview
        let result = engine.execute(plans: &plans)
        lastExecuted = plans
        preview = []
        resultMessage = "\(result.moved)개 파일 정리 완료"
        if !result.errors.isEmpty {
            resultMessage += " (\(result.errors.count)개 실패)"
        }
    }

    private func undoOrganize() {
        let undone = engine.undo(plans: lastExecuted)
        lastExecuted = []
        resultMessage = "\(undone)개 파일 원래 위치로 복원"
        refreshPreview()
    }

    private func shortenDest(_ url: URL) -> String {
        guard let target = viewModel.organizerTargetURL else { return url.path }
        let base = target.path
        if url.path.hasPrefix(base) {
            return "./" + url.path.dropFirst(base.count + 1)
        }
        return url.path
    }
}
