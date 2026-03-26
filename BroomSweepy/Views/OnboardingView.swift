import SwiftUI

struct OnboardingView: View {
    @Binding var isComplete: Bool
    @State private var currentStep = 0
    @State private var showContent = false
    @State private var bounceVal = 0

    var body: some View {
        VStack(spacing: 0) {
            // Progress dots
            HStack(spacing: 8) {
                ForEach(0..<3) { i in
                    Circle()
                        .fill(i <= currentStep ? Color.accentColor : Color.secondary.opacity(0.3))
                        .frame(width: 8, height: 8)
                        .scaleEffect(i == currentStep ? 1.2 : 1.0)
                        .animation(.spring(duration: 0.3, bounce: 0.4), value: currentStep)
                }
            }
            .padding(.top, 32)

            Spacer()

            // Step content
            Group {
                switch currentStep {
                case 0: welcomeStep
                case 1: permissionStep
                case 2: readyStep
                default: EmptyView()
                }
            }
            .transition(.asymmetric(
                insertion: .push(from: .trailing),
                removal: .push(from: .leading)
            ))

            Spacer()

            // Navigation
            HStack {
                if currentStep > 0 {
                    Button("이전") {
                        withAnimation(.spring(duration: 0.4, bounce: 0.2)) {
                            currentStep -= 1
                        }
                    }
                    .buttonStyle(.bordered)
                }

                Spacer()

                if currentStep < 2 {
                    Button("다음") {
                        withAnimation(.spring(duration: 0.4, bounce: 0.2)) {
                            currentStep += 1
                        }
                    }
                    .buttonStyle(.borderedProminent)
                } else {
                    Button("시작하기") {
                        UserDefaults.standard.set(true, forKey: "onboardingCompleted")
                        withAnimation(.spring(duration: 0.5, bounce: 0.3)) {
                            isComplete = true
                        }
                    }
                    .buttonStyle(.borderedProminent)
                    .controlSize(.large)
                }
            }
            .padding(.horizontal, 48)
            .padding(.bottom, 32)
        }
        .frame(width: 600, height: 480)
        .onAppear {
            withAnimation(.spring(duration: 0.8, bounce: 0.3).delay(0.1)) {
                showContent = true
            }
        }
    }

    // MARK: - Step 1: Welcome

    private var welcomeStep: some View {
        VStack(spacing: 24) {
            ZStack {
                Circle()
                    .fill(.linearGradient(
                        colors: [.blue.opacity(0.15), .purple.opacity(0.1)],
                        startPoint: .topLeading,
                        endPoint: .bottomTrailing
                    ))
                    .frame(width: 120, height: 120)

                Image(systemName: "sparkles")
                    .font(.system(size: 56))
                    .foregroundStyle(.linearGradient(
                        colors: [.blue, .purple],
                        startPoint: .topLeading,
                        endPoint: .bottomTrailing
                    ))
                    .symbolEffect(.breathe, isActive: showContent)
            }
            .scaleEffect(showContent ? 1 : 0.7)
            .opacity(showContent ? 1 : 0)

            VStack(spacing: 8) {
                Text("BroomSweepy에 오신 것을 환영합니다")
                    .font(.title2.bold())
                Text("Mac을 깨끗하고 빠르게 유지해드립니다")
                    .font(.body)
                    .foregroundStyle(.secondary)
            }

            // Feature highlights
            VStack(alignment: .leading, spacing: 12) {
                FeatureRow(icon: "internaldrive.fill", color: .red,
                           text: "캐시, 로그, 임시파일 정리")
                FeatureRow(icon: "doc.on.doc.fill", color: .orange,
                           text: "중복 파일 탐색 및 제거")
                FeatureRow(icon: "shield.checkered", color: .green,
                           text: "악성코드 검사 및 보안 관리")
                FeatureRow(icon: "gauge.with.dots.needle.33percent", color: .blue,
                           text: "실시간 시스템 모니터링")
            }
            .padding(.horizontal, 80)
        }
    }

    // MARK: - Step 2: Permission

    private var permissionStep: some View {
        VStack(spacing: 24) {
            Image(systemName: "folder.badge.gearshape")
                .font(.system(size: 64))
                .symbolRenderingMode(.hierarchical)
                .foregroundStyle(.blue)
                .symbolEffect(.bounce, value: bounceVal)

            VStack(spacing: 8) {
                Text("폴더 접근 권한이 필요합니다")
                    .font(.title2.bold())
                Text("캐시, 로그, 중복 파일을 검색하려면\n홈 폴더에 대한 접근 권한이 필요합니다")
                    .font(.body)
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
            }

            Button {
                bounceVal += 1
                requestFolderAccess()
            } label: {
                Label("폴더 접근 허용하기", systemImage: "folder.badge.plus")
                    .font(.headline)
                    .padding(.horizontal, 24)
                    .padding(.vertical, 12)
            }
            .buttonStyle(.borderedProminent)

            if FileAccessManager.shared.loadBookmark() != nil {
                HStack(spacing: 6) {
                    Image(systemName: "checkmark.circle.fill")
                        .foregroundStyle(.green)
                    Text("접근 권한이 설정되었습니다")
                        .font(.callout)
                        .foregroundStyle(.green)
                }
                .transition(.push(from: .bottom).combined(with: .opacity))
            }

            Text("나중에 설정할 수도 있습니다")
                .font(.caption)
                .foregroundStyle(.tertiary)
        }
    }

    // MARK: - Step 3: Ready

    private var readyStep: some View {
        VStack(spacing: 24) {
            Image(systemName: "checkmark.seal.fill")
                .font(.system(size: 64))
                .foregroundStyle(.green)
                .phaseAnimator([false, true], trigger: currentStep) { content, phase in
                    content
                        .scaleEffect(phase ? 1.0 : 0.7)
                        .rotationEffect(.degrees(phase ? 0 : -15))
                } animation: { _ in
                    .spring(duration: 0.6, bounce: 0.4)
                }

            VStack(spacing: 8) {
                Text("준비 완료!")
                    .font(.title2.bold())
                Text("BroomSweepy가 Mac을 깨끗하게 유지해드리겠습니다")
                    .font(.body)
                    .foregroundStyle(.secondary)
            }

            VStack(alignment: .leading, spacing: 10) {
                TipRow(icon: "menubar.arrow.up.rectangle", text: "메뉴바에서 실시간 시스템 상태를 확인하세요")
                TipRow(icon: "xmark.circle", text: "창을 닫아도 메뉴바에 상주합니다")
                TipRow(icon: "magnifyingglass", text: "대시보드에서 '전체 스캔'으로 시작하세요")
            }
            .padding(.horizontal, 80)
        }
    }

    private func requestFolderAccess() {
        Task { @MainActor in
            _ = await FileAccessManager.shared.requestHomeAccess()
        }
    }
}

// MARK: - Feature Row

private struct FeatureRow: View {
    let icon: String
    let color: Color
    let text: String

    var body: some View {
        HStack(spacing: 12) {
            Image(systemName: icon)
                .font(.title3)
                .foregroundStyle(color)
                .frame(width: 28)
            Text(text)
                .font(.callout)
        }
    }
}

// MARK: - Tip Row

private struct TipRow: View {
    let icon: String
    let text: String

    var body: some View {
        HStack(spacing: 12) {
            Image(systemName: icon)
                .font(.callout)
                .foregroundStyle(.blue)
                .frame(width: 24)
            Text(text)
                .font(.callout)
                .foregroundStyle(.secondary)
        }
    }
}
