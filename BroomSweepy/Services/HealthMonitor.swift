import Foundation
import SwiftUI
import UserNotifications

/// Mac 건강 상태를 모니터링하고 브리핑을 생성하는 서비스
@Observable
final class HealthMonitor {
    static let shared = HealthMonitor()

    var briefing: DailyBriefing?
    var lastCheckDate: Date?

    struct DailyBriefing {
        let date: Date
        let healthScore: Int          // 0-100
        let diskUsagePercent: Double
        let diskFree: String
        let memoryUsagePercent: Double
        let estimatedCacheSize: String
        let recommendations: [Recommendation]
        let greeting: String

        struct Recommendation {
            let icon: String
            let text: String
            let priority: Priority
            let category: String

            enum Priority { case high, medium, low }

            var color: Color {
                switch priority {
                case .high: return .red
                case .medium: return .orange
                case .low: return .green
                }
            }
        }

        var scoreColor: Color {
            if healthScore >= 80 { return .green }
            if healthScore >= 60 { return .orange }
            return .red
        }

        var scoreLabel: String {
            if healthScore >= 90 { return "최고" }
            if healthScore >= 80 { return "양호" }
            if healthScore >= 60 { return "보통" }
            if healthScore >= 40 { return "주의" }
            return "위험"
        }
    }

    /// 현재 상태를 분석하여 브리핑 생성
    func generateBriefing() {
        let disk = SystemMonitor.shared.getDiskInfo()
        let mem = MemoryManager.shared.getMemoryInfo()

        // 건강 점수 계산 (100점 만점)
        var score = 100

        // 디스크: 90%+ 사용 → -30, 80%+ → -15, 70%+ → -5
        if disk.usagePercent > 90 { score -= 30 }
        else if disk.usagePercent > 80 { score -= 15 }
        else if disk.usagePercent > 70 { score -= 5 }

        // 메모리: 85%+ → -20, 70%+ → -10
        let memPct = mem.usagePercent
        if memPct > 85 { score -= 20 }
        else if memPct > 70 { score -= 10 }

        // 추천사항 생성
        var recs: [DailyBriefing.Recommendation] = []

        if disk.usagePercent > 80 {
            recs.append(.init(
                icon: "internaldrive.fill", text: "디스크 여유 공간이 부족합니다. 캐시 정리를 추천합니다.",
                priority: disk.usagePercent > 90 ? .high : .medium, category: "space"
            ))
        }

        if memPct > 75 {
            recs.append(.init(
                icon: "memorychip.fill", text: "메모리 사용량이 높습니다. 메모리 정리를 추천합니다.",
                priority: memPct > 85 ? .high : .medium, category: "speed"
            ))
        }

        // 마지막 정리 후 7일 이상 경과
        let lastClean = UserDefaults.standard.object(forKey: "lastCleanDate") as? Date
        if lastClean == nil || Date().timeIntervalSince(lastClean!) > 7 * 24 * 3600 {
            recs.append(.init(
                icon: "sparkles", text: "최근 정리한 지 7일이 넘었습니다. 원클릭 최적화를 실행해보세요.",
                priority: .low, category: "space"
            ))
        }

        if disk.usagePercent < 70 && memPct < 60 && recs.isEmpty {
            recs.append(.init(
                icon: "checkmark.seal.fill", text: "Mac이 최적 상태입니다. 잘 관리되고 있어요!",
                priority: .low, category: "space"
            ))
        }

        // 인사말
        let hour = Calendar.current.component(.hour, from: Date())
        let greeting: String
        if hour < 12 { greeting = "좋은 아침이에요" }
        else if hour < 18 { greeting = "좋은 오후에요" }
        else { greeting = "좋은 저녁이에요" }

        briefing = DailyBriefing(
            date: Date(),
            healthScore: max(0, min(100, score)),
            diskUsagePercent: disk.usagePercent,
            diskFree: disk.freeFormatted,
            memoryUsagePercent: memPct,
            estimatedCacheSize: "—",
            recommendations: recs,
            greeting: greeting
        )

        lastCheckDate = Date()
    }

    /// 마지막 정리 날짜 기록
    func recordClean() {
        UserDefaults.standard.set(Date(), forKey: "lastCleanDate")
    }

    /// macOS 알림 발송
    func sendNotificationIfNeeded() {
        guard let b = briefing, b.healthScore < 70 else { return }

        let content = UNMutableNotificationContent()
        content.title = "BroomSweepy"
        content.body = b.recommendations.first?.text ?? "Mac 상태를 확인해보세요."
        content.sound = .default

        let trigger = UNTimeIntervalNotificationTrigger(timeInterval: 1, repeats: false)
        let request = UNNotificationRequest(identifier: "health-\(Date())", content: content, trigger: trigger)

        UNUserNotificationCenter.current().add(request)
    }

    /// 알림 권한 요청
    func requestNotificationPermission() {
        UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .sound]) { _, _ in }
    }
}
