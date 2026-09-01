import SwiftUI

private struct PendingMaintenance: Identifiable {
    let id = UUID()
    let taskID: UUID
    let taskName: String
    let preview: MaintenancePreview
}

struct MaintenanceView: View {
    @Bindable var viewModel: CleanerViewModel
    @State private var tasks: [MaintenanceTask] = MaintenanceManager.shared.getAvailableTasks()
    @State private var pendingMaintenance: PendingMaintenance?

    private var completedCount: Int { tasks.filter(\.isCompleted).count }
    private var hasRunningTask: Bool { tasks.contains(where: \.isRunning) }

    var body: some View {
        VStack(spacing: 0) {
            header
            taskList
        }
        .alert(item: $pendingMaintenance) { pending in
            Alert(
                title: Text("휴지통으로 이동하기 전 최종 확인"),
                message: Text(
                    "\(pending.taskName)\n" +
                    "\(pending.preview.candidates.count)개 항목 · \(formatSize(pending.preview.logicalSize))\n\n" +
                    "검토한 항목과 상위 폴더가 같은지 실행 직전에 다시 확인합니다. 휴지통에서 복원할 수 있으며, 비워야 디스크 여유가 늘어납니다."
                ),
                primaryButton: .destructive(Text("휴지통으로 이동")) {
                    Task { await runConfirmed(pending) }
                },
                secondaryButton: .cancel(Text("취소"))
            )
        }
    }

    private var header: some View {
        HStack(spacing: 12) {
            VStack(alignment: .leading, spacing: 2) {
                Text("유지보수")
                    .font(.title2.bold())
                Text("파일 이동 작업은 대상 검토와 최종 확인 후 실행하며, 관리자 명령은 안내만 합니다")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            Spacer()
            if completedCount > 0 {
                Text("\(completedCount)개 이동 작업 완료")
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }
            Button("결과 초기화") { tasks = MaintenanceManager.shared.getAvailableTasks() }
                .buttonStyle(.bordered)
                .disabled(hasRunningTask || pendingMaintenance != nil)
        }
        .padding(24)
    }

    private var taskList: some View {
        ScrollView {
            LazyVStack(spacing: 12) {
                ForEach($tasks) { $task in
                    MaintenanceTaskRow(task: $task) {
                        guard let index = tasks.firstIndex(where: { $0.id == task.id }) else { return }
                        Task { await prepareTask(index: index) }
                    }
                }
            }
            .padding(.horizontal, 24)
            .padding(.bottom, 24)
        }
    }

    @MainActor
    private func prepareTask(index: Int) async {
        guard tasks.indices.contains(index), !tasks[index].isRunning else { return }
        let task = tasks[index]

        if let instruction = MaintenanceManager.shared.instruction(for: task) {
            apply(instruction, toTaskID: task.id)
            viewModel.toastMessage = instruction.message
            return
        }

        guard let homeURL = FileAccessManager.shared.loadBookmark()
                ?? FileAccessManager.shared.requestHomeAccess() else {
            apply(.failure(message: "홈 폴더 접근 권한이 필요합니다", errors: []), toTaskID: task.id)
            return
        }

        tasks[index].isRunning = true
        let preview = await Task.detached {
            MaintenanceManager.shared.preview(task: task, homeURL: homeURL)
        }.value
        guard let current = tasks.firstIndex(where: { $0.id == task.id }) else { return }
        tasks[current].isRunning = false

        if let firstError = preview.errors.first {
            apply(.failure(message: "대상 검토 중 오류가 있어 실행하지 않았습니다: \(firstError)", errors: preview.errors), toTaskID: task.id)
        } else if preview.candidates.isEmpty {
            apply(.noChange(message: "휴지통으로 이동할 대상이 없습니다"), toTaskID: task.id)
        } else {
            pendingMaintenance = PendingMaintenance(
                taskID: task.id,
                taskName: task.name,
                preview: preview
            )
        }
    }

    @MainActor
    private func runConfirmed(_ pending: PendingMaintenance) async {
        guard let index = tasks.firstIndex(where: { $0.id == pending.taskID }),
              !tasks[index].isRunning else { return }
        tasks[index].isRunning = true
        let task = tasks[index]
        let outcome = await Task.detached {
            MaintenanceManager.shared.runTask(task, preview: pending.preview)
        }.value
        apply(outcome, toTaskID: task.id)
        if outcome.movedSize > 0 {
            HealthMonitor.shared.recordClean()
            CleanHistory.shared.record(freed: outcome.movedSize, type: "manual")
        }
        viewModel.toastMessage = outcome.message
    }

    private func apply(_ outcome: MaintenanceOutcome, toTaskID id: UUID) {
        guard let index = tasks.firstIndex(where: { $0.id == id }) else { return }
        tasks[index].isRunning = false
        tasks[index].result = outcome.message
        tasks[index].resultKind = outcome.kind
        tasks[index].isCompleted = outcome.kind == .success || outcome.kind == .noChange
    }
}

private struct MaintenanceTaskRow: View {
    @Binding var task: MaintenanceTask
    let onRun: () -> Void

    var body: some View {
        HStack(spacing: 16) {
            ZStack {
                RoundedRectangle(cornerRadius: 10)
                    .fill(statusColor.opacity(0.12))
                    .frame(width: 44, height: 44)
                if task.isRunning {
                    ProgressView().scaleEffect(0.75)
                } else {
                    Image(systemName: task.isCompleted ? "checkmark" : task.icon)
                        .font(.title3)
                        .foregroundStyle(statusColor)
                }
            }

            VStack(alignment: .leading, spacing: 4) {
                HStack(spacing: 6) {
                    Text(task.name).font(.callout.weight(.semibold))
                    if task.type.isInstructionOnly {
                        Text("안내 전용")
                            .font(.system(size: 9, weight: .bold))
                            .padding(.horizontal, 5)
                            .padding(.vertical, 2)
                            .background(.orange.opacity(0.15), in: Capsule())
                            .foregroundStyle(.orange)
                    }
                }
                Text(task.description)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                if let result = task.result {
                    Text(result)
                        .font(.caption2)
                        .foregroundStyle(statusColor)
                        .textSelection(.enabled)
                        .padding(.top, 2)
                }
            }

            Spacer()

            Button(task.type.isInstructionOnly ? "안내 보기" : (task.isCompleted ? "다시 검토" : "검토")) {
                onRun()
            }
            .buttonStyle(.bordered)
            .disabled(task.isRunning)
        }
        .padding(16)
        .background(statusColor.opacity(task.result == nil ? 0 : 0.04))
        .clipShape(RoundedRectangle(cornerRadius: 12))
        .overlay(RoundedRectangle(cornerRadius: 12).stroke(statusColor.opacity(0.2)))
    }

    private var statusColor: Color {
        if task.isRunning { return .blue }
        switch task.resultKind {
        case .success, .noChange: return .green
        case .partial, .instruction: return .orange
        case .failure: return .red
        case nil: return .secondary
        }
    }
}
