import SwiftUI

/// 각 기능 페이지 상단에 표시되는 안내 배너
struct FeatureInfoBanner: View {
    let icon: String
    let description: String
    let safetyNote: String?

    @State private var isCollapsed: Bool

    init(icon: String, description: String, safetyNote: String? = nil, defaultCollapsed: Bool = false) {
        self.icon = icon
        self.description = description
        self.safetyNote = safetyNote
        self._isCollapsed = State(initialValue: defaultCollapsed)
    }

    var body: some View {
        if isCollapsed {
            // 접힌 상태: 작은 ℹ️ 버튼
            HStack {
                Spacer()
                Button {
                    withAnimation(.spring(duration: 0.3)) { isCollapsed = false }
                } label: {
                    Image(systemName: "info.circle")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                .buttonStyle(.plain)
                .help("도움말 보기")
            }
            .padding(.horizontal, 24)
        } else {
            HStack(alignment: .top, spacing: 12) {
                Image(systemName: icon)
                    .font(.title3)
                    .foregroundStyle(.blue)
                    .frame(width: 24, alignment: .center)
                    .padding(.top, 2)

                VStack(alignment: .leading, spacing: 6) {
                    Text(description)
                        .font(.callout)
                        .foregroundStyle(.secondary)
                        .fixedSize(horizontal: false, vertical: true)

                    if let note = safetyNote {
                        HStack(spacing: 6) {
                            Image(systemName: "checkmark.shield.fill")
                                .font(.caption)
                                .foregroundStyle(.green)
                            Text(note)
                                .font(.caption)
                                .foregroundStyle(.green)
                        }
                    }
                }

                Spacer(minLength: 0)

                Button {
                    withAnimation(.spring(duration: 0.3)) { isCollapsed = true }
                } label: {
                    Image(systemName: "xmark")
                        .font(.caption2)
                        .foregroundStyle(.tertiary)
                }
                .buttonStyle(.plain)
            }
            .padding(14)
            .background(.blue.opacity(0.04), in: RoundedRectangle(cornerRadius: 10))
            .overlay(
                RoundedRectangle(cornerRadius: 10)
                    .stroke(.blue.opacity(0.1), lineWidth: 1)
            )
            .padding(.horizontal, 24)
            .transition(.opacity.combined(with: .move(edge: .top)))
        }
    }
}
