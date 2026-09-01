import SwiftUI

/// 빈 상태에서 다음 기능을 추천하는 미니 버튼
struct MiniNextButton: View {
    let icon: String
    let title: String
    let color: Color

    var body: some View {
        VStack(spacing: 6) {
            ZStack {
                Circle()
                    .fill(color.opacity(0.1))
                    .frame(width: 36, height: 36)
                Image(systemName: icon)
                    .font(.system(size: 14))
                    .foregroundStyle(color)
            }
            Text(title)
                .font(.system(size: 10))
                .foregroundStyle(.secondary)
        }
        .frame(width: 80)
    }
}

/// 액션 완료 후 다음 단계를 안내하는 카드
struct NextStepCard: View {
    let icon: String
    let title: String
    let description: String
    let color: Color
    let action: () -> Void

    @State private var isHovering = false

    var body: some View {
        Button(action: action) {
            HStack(spacing: 14) {
                ZStack {
                    Circle()
                        .fill(color.opacity(0.12))
                        .frame(width: 36, height: 36)
                    Image(systemName: icon)
                        .font(.body)
                        .foregroundStyle(color)
                }

                VStack(alignment: .leading, spacing: 2) {
                    Text(title)
                        .font(.system(size: 13, weight: .semibold))
                        .foregroundStyle(.primary)
                    Text(description)
                        .font(.system(size: 11))
                        .foregroundStyle(.secondary)
                }

                Spacer()

                Image(systemName: "chevron.right")
                    .font(.caption)
                    .foregroundStyle(.tertiary)
            }
            .padding(12)
            .background(
                RoundedRectangle(cornerRadius: 10)
                    .fill(color.opacity(isHovering ? 0.08 : 0.03))
                    .overlay(
                        RoundedRectangle(cornerRadius: 10)
                            .stroke(color.opacity(isHovering ? 0.2 : 0.08), lineWidth: 1)
                    )
            )
            .scaleEffect(isHovering ? 1.01 : 1.0)
        }
        .buttonStyle(.plain)
        .onHover { h in
            withAnimation(.easeInOut(duration: 0.15)) { isHovering = h }
        }
    }
}

/// 정리 완료 후 표시되는 "다음 단계" 섹션
struct NextStepsSection: View {
    let currentFeature: String
    let onNavigate: (MainCategory) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("다음 추천")
                .font(.system(size: 12, weight: .semibold))
                .foregroundStyle(.secondary)
                .padding(.leading, 4)

            ForEach(recommendations, id: \.title) { rec in
                NextStepCard(
                    icon: rec.icon,
                    title: rec.title,
                    description: rec.description,
                    color: rec.color,
                    action: { onNavigate(rec.target) }
                )
            }
        }
        .padding(.horizontal, 24)
        .padding(.top, 12)
        .transition(.move(edge: .bottom).combined(with: .opacity))
    }

    private struct Recommendation {
        let icon: String
        let title: String
        let description: String
        let color: Color
        let target: MainCategory
    }

    private var recommendations: [Recommendation] {
        switch currentFeature {
        case "cache":
            return [
                Recommendation(icon: "doc.richtext.fill", title: "대용량 파일 확인",
                              description: "50MB 이상의 큰 파일을 찾아보세요", color: .blue, target: .space),
                Recommendation(icon: "gauge.with.dots.needle.67percent", title: "속도 최적화",
                              description: "메모리 정리와 시작프로그램 관리", color: .green, target: .speed),
            ]
        case "largeFiles":
            return [
                Recommendation(icon: "doc.on.doc.fill", title: "중복 파일 탐색",
                              description: "같은 파일이 여러 곳에 있는지 확인", color: .orange, target: .space),
                Recommendation(icon: "hand.raised.fill", title: "개인정보 정리",
                              description: "브라우저 기록과 쿠키 검토", color: .purple, target: .privacy),
            ]
        case "duplicates":
            return [
                Recommendation(icon: "shield.checkered", title: "의심 항목 검토",
                              description: "이름 패턴이 일치한 항목 확인", color: .orange, target: .security),
                Recommendation(icon: "folder.fill", title: "파일 자동 정리",
                              description: "파일을 규칙에 따라 자동 분류", color: .cyan, target: .files),
            ]
        case "memory":
            return [
                Recommendation(icon: "internaldrive.fill", title: "캐시 정리",
                              description: "불필요한 캐시 검토", color: .red, target: .space),
                Recommendation(icon: "power", title: "시작프로그램 관리",
                              description: "불필요한 자동 실행 프로그램 정리", color: .green, target: .speed),
            ]
        case "malware":
            return [
                Recommendation(icon: "lock.shield.fill", title: "앱 권한 확인",
                              description: "카메라/마이크 권한 현황 점검", color: .orange, target: .security),
                Recommendation(icon: "hand.raised.fill", title: "개인정보 정리",
                              description: "브라우저 추적 데이터 검토", color: .purple, target: .privacy),
            ]
        default:
            return [
                Recommendation(icon: "sparkles", title: "원클릭 최적화",
                              description: "모든 항목을 한번에 정리", color: .green, target: .dashboard),
            ]
        }
    }
}
