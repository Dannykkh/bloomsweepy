import SwiftUI

struct RuleBuilderView: View {
    @Bindable var viewModel: CleanerViewModel
    @State private var showAddSheet = false
    @State private var editingRule: OrganizeRule?

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Text("규칙 빌더")
                    .font(.title2.bold())
                Spacer()
                Button("기본 규칙 추가") {
                    for rule in OrganizeRule.defaults {
                        if !viewModel.rules.contains(where: { $0.name == rule.name }) {
                            viewModel.rules.append(rule)
                        }
                    }
                }
                .buttonStyle(.bordered)
                Button {
                    editingRule = nil
                    showAddSheet = true
                } label: {
                    Label("새 규칙", systemImage: "plus")
                }
                .buttonStyle(.borderedProminent)
            }
            .padding(24)

            if viewModel.rules.isEmpty {
                VStack(spacing: 12) {
                    Image(systemName: "slider.horizontal.3")
                        .font(.system(size: 40))
                        .foregroundStyle(.secondary)
                    Text("규칙이 없습니다")
                        .font(.headline)
                        .foregroundStyle(.secondary)
                    Text("'새 규칙' 또는 '기본 규칙 추가'를 눌러 시작하세요")
                        .font(.caption)
                        .foregroundStyle(.tertiary)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                List {
                    ForEach($viewModel.rules) { $rule in
                        RuleRow(rule: $rule) {
                            editingRule = rule
                            showAddSheet = true
                        } onDelete: {
                            viewModel.rules.removeAll { $0.id == rule.id }
                        }
                    }
                    .onMove { from, to in
                        viewModel.rules.move(fromOffsets: from, toOffset: to)
                    }
                }
                .listStyle(.inset(alternatesRowBackgrounds: true))
            }
        }
        .sheet(isPresented: $showAddSheet) {
            RuleEditorSheet(
                rule: editingRule,
                onSave: { rule in
                    if let index = viewModel.rules.firstIndex(where: { $0.id == rule.id }) {
                        viewModel.rules[index] = rule
                    } else {
                        viewModel.rules.append(rule)
                    }
                    showAddSheet = false
                },
                onCancel: { showAddSheet = false }
            )
            .frame(minWidth: 500, minHeight: 400)
        }
    }
}

// MARK: - Rule Row

struct RuleRow: View {
    @Binding var rule: OrganizeRule
    let onEdit: () -> Void
    let onDelete: () -> Void

    var body: some View {
        HStack(spacing: 14) {
            Toggle("", isOn: $rule.isEnabled)
                .toggleStyle(.switch)
                .labelsHidden()

            VStack(alignment: .leading, spacing: 4) {
                Text(rule.name)
                    .font(.headline)
                    .foregroundStyle(rule.isEnabled ? .primary : .secondary)

                HStack(spacing: 4) {
                    ForEach(rule.conditions, id: \.self) { cond in
                        Text(cond.description)
                            .font(.system(size: 10))
                            .padding(.horizontal, 6)
                            .padding(.vertical, 2)
                            .background(.blue.opacity(0.1), in: Capsule())
                            .foregroundStyle(.blue)
                    }
                    Image(systemName: "arrow.right")
                        .font(.system(size: 8))
                        .foregroundStyle(.secondary)
                    ForEach(rule.actions, id: \.self) { action in
                        Text(action.description)
                            .font(.system(size: 10))
                            .padding(.horizontal, 6)
                            .padding(.vertical, 2)
                            .background(.green.opacity(0.1), in: Capsule())
                            .foregroundStyle(.green)
                    }
                }
            }

            Spacer()

            Button { onEdit() } label: {
                Image(systemName: "pencil")
            }
            .buttonStyle(.borderless)

            Button { onDelete() } label: {
                Image(systemName: "trash")
                    .foregroundStyle(.red)
            }
            .buttonStyle(.borderless)
        }
        .padding(.vertical, 4)
    }
}

// MARK: - Rule Editor Sheet

struct RuleEditorSheet: View {
    let rule: OrganizeRule?
    let onSave: (OrganizeRule) -> Void
    let onCancel: () -> Void

    @State private var name = ""
    @State private var conditions: [RuleCondition] = []
    @State private var actions: [RuleAction] = []

    // Condition input
    @State private var condType = 0
    @State private var condValue = ""
    @State private var condIntValue = 30

    // Action input
    @State private var actionType = 0
    @State private var actionValue = ""

    var body: some View {
        VStack(spacing: 20) {
            Text(rule == nil ? "새 규칙" : "규칙 수정")
                .font(.title2.bold())

            TextField("규칙 이름", text: $name)
                .textFieldStyle(.roundedBorder)

            // Conditions
            GroupBox("조건") {
                VStack(alignment: .leading, spacing: 8) {
                    ForEach(conditions, id: \.self) { cond in
                        HStack {
                            Text(cond.description)
                                .font(.callout)
                            Spacer()
                            Button { conditions.removeAll { $0 == cond } } label: {
                                Image(systemName: "xmark.circle.fill").foregroundStyle(.red)
                            }.buttonStyle(.borderless)
                        }
                    }

                    HStack {
                        Picker("", selection: $condType) {
                            Text("확장자").tag(0)
                            Text("파일명 포함").tag(1)
                            Text("크기(MB) 초과").tag(2)
                            Text("경과 일수").tag(3)
                        }
                        .frame(width: 120)

                        if condType < 2 {
                            TextField("값", text: $condValue)
                                .textFieldStyle(.roundedBorder)
                        } else {
                            TextField("숫자", value: $condIntValue, format: .number)
                                .textFieldStyle(.roundedBorder)
                                .frame(width: 80)
                        }

                        Button("추가") { addCondition() }
                            .buttonStyle(.bordered)
                    }
                }
            }

            // Actions
            GroupBox("동작") {
                VStack(alignment: .leading, spacing: 8) {
                    ForEach(actions, id: \.self) { action in
                        HStack {
                            Text(action.description)
                                .font(.callout)
                            Spacer()
                            Button { actions.removeAll { $0 == action } } label: {
                                Image(systemName: "xmark.circle.fill").foregroundStyle(.red)
                            }.buttonStyle(.borderless)
                        }
                    }

                    HStack {
                        Picker("", selection: $actionType) {
                            Text("폴더로 이동").tag(0)
                            Text("날짜 접두어").tag(1)
                            Text("휴지통").tag(2)
                        }
                        .frame(width: 120)

                        if actionType == 0 {
                            TextField("폴더명", text: $actionValue)
                                .textFieldStyle(.roundedBorder)
                        }

                        Button("추가") { addAction() }
                            .buttonStyle(.bordered)
                    }
                }
            }

            Spacer()

            HStack {
                Button("취소") { onCancel() }
                    .buttonStyle(.bordered)
                Spacer()
                Button("저장") {
                    let r = OrganizeRule(
                        id: rule?.id ?? UUID(),
                        name: name,
                        conditions: conditions,
                        actions: actions
                    )
                    onSave(r)
                }
                .buttonStyle(.borderedProminent)
                .disabled(name.isEmpty || conditions.isEmpty || actions.isEmpty)
            }
        }
        .padding(24)
        .onAppear {
            if let r = rule {
                name = r.name
                conditions = r.conditions
                actions = r.actions
            }
        }
    }

    private func addCondition() {
        switch condType {
        case 0: if !condValue.isEmpty { conditions.append(.extensionIs(condValue)); condValue = "" }
        case 1: if !condValue.isEmpty { conditions.append(.nameContains(condValue)); condValue = "" }
        case 2: conditions.append(.sizeGreaterThan(Int64(condIntValue) * 1024 * 1024))
        case 3: conditions.append(.olderThanDays(condIntValue))
        default: break
        }
    }

    private func addAction() {
        switch actionType {
        case 0: if !actionValue.isEmpty { actions.append(.moveToFolder(actionValue)); actionValue = "" }
        case 1: actions.append(.addDatePrefix)
        case 2: actions.append(.moveToTrash)
        default: break
        }
    }
}
