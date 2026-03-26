import SwiftUI

struct MaintenanceView: View {
    @Bindable var viewModel: CleanerViewModel
    @State private var tasks: [MaintenanceTask] = MaintenanceManager.shared.getAvailableTasks()
    @State private var isRunningAll = false
    @State private var allCompleted = false

    private var completedCount: Int {
        tasks.filter(\.isCompleted).count
    }

    var body: some View {
        VStack(spacing: 0) {
            header
            taskList
        }
    }

    // MARK: - Header

    private var header: some View {
        HStack(spacing: 12) {
            VStack(alignment: .leading, spacing: 2) {
                Text("유지보수 스크립트")
                    .font(.title2.bold())
                Text("시스템 캐시 및 데이터베이스를 정리합니다")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            Spacer()
            if completedCount > 0 {
                Text("\(completedCount)/\(tasks.count) 완료")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .padding(.horizontal, 10)
                    .padding(.vertical, 4)
                    .background(.secondary.opacity(0.1), in: Capsule())
            }
            Button("초기화") { resetTasks() }
                .buttonStyle(.bordered)
                .disabled(isRunningAll)
            Button("전체 실행") {
                Task { await runAllTasks() }
            }
            .buttonStyle(.borderedProminent)
            .disabled(isRunningAll || allCompleted)
        }
        .padding(24)
    }

    // MARK: - Info Banner


    // MARK: - Task List

    private var taskList: some View {
        ScrollView {
            LazyVStack(spacing: 12) {
                ForEach($tasks) { $task in
                    MaintenanceTaskRow(task: $task) {
                        Task { await runSingleTask(index: tasks.firstIndex(where: { $0.id == task.id })!) }
                    }
                }
            }
            .padding(.horizontal, 24)
            .padding(.bottom, 24)
        }
    }

    // MARK: - Actions

    @MainActor
    private func runSingleTask(index: Int) async {
        guard !tasks[index].isRunning, !tasks[index].isCompleted else { return }
        tasks[index].isRunning = true
        let task = tasks[index]
        let result = await Task.detached {
            MaintenanceManager.shared.runTask(task)
        }.value
        tasks[index].isRunning = false
        tasks[index].isCompleted = true
        tasks[index].result = result
        viewModel.toastMessage = result
    }

    @MainActor
    private func runAllTasks() async {
        isRunningAll = true
        for index in tasks.indices {
            guard !tasks[index].isCompleted else { continue }
            await runSingleTask(index: index)
        }
        isRunningAll = false
        allCompleted = tasks.allSatisfy(\.isCompleted)
        viewModel.toastMessage = "모든 유지보수 작업이 완료되었습니다"
    }

    private func resetTasks() {
        tasks = MaintenanceManager.shared.getAvailableTasks()
        allCompleted = false
    }
}

// MARK: - Maintenance Task Row

private struct MaintenanceTaskRow: View {
    @Binding var task: MaintenanceTask
    let onRun: () -> Void

    var body: some View {
        HStack(spacing: 16) {
            // Icon
            ZStack {
                RoundedRectangle(cornerRadius: 10)
                    .fill(iconBackground)
                    .frame(width: 44, height: 44)
                if task.isRunning {
                    ProgressView()
                        .scaleEffect(0.75)
                        .progressViewStyle(.circular)
                } else {
                    Image(systemName: task.isCompleted ? "checkmark" : task.icon)
                        .font(.title3)
                        .foregroundStyle(iconForeground)
                }
            }

            // Content
            VStack(alignment: .leading, spacing: 4) {
                HStack(spacing: 6) {
                    Text(task.name)
                        .font(.callout.weight(.semibold))
                    if task.requiresAdmin {
                        Text("수동")
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
                    HStack(spacing: 4) {
                        Image(systemName: "checkmark.circle.fill")
                            .font(.caption2)
                            .foregroundStyle(.green)
                        Text(result)
                            .font(.caption2)
                            .foregroundStyle(.green)
                    }
                    .padding(.top, 2)
                }
            }

            Spacer()

            // Run button
            if task.isCompleted {
                Image(systemName: "checkmark.circle.fill")
                    .font(.title2)
                    .foregroundStyle(.green)
            } else {
                Button("실행") { onRun() }
                    .buttonStyle(.bordered)
                    .disabled(task.isRunning)
                    .overlay {
                        if task.isRunning {
                            ProgressView()
                                .scaleEffect(0.6)
                        }
                    }
            }
        }
        .padding(16)
        .background(rowBackground)
        .clipShape(RoundedRectangle(cornerRadius: 12))
        .overlay(
            RoundedRectangle(cornerRadius: 12)
                .stroke(borderColor, lineWidth: 1)
        )
    }

    private var iconBackground: Color {
        if task.isCompleted { return .green.opacity(0.12) }
        if task.isRunning { return .blue.opacity(0.12) }
        return .secondary.opacity(0.1)
    }

    private var iconForeground: Color {
        if task.isCompleted { return .green }
        if task.isRunning { return .blue }
        return .secondary
    }

    private var rowBackground: Color {
        if task.isCompleted { return .green.opacity(0.04) }
        if task.isRunning { return .blue.opacity(0.04) }
        return .clear
    }

    private var borderColor: Color {
        if task.isCompleted { return .green.opacity(0.2) }
        if task.isRunning { return .blue.opacity(0.2) }
        return .secondary.opacity(0.15)
    }
}
