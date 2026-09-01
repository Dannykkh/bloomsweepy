import Foundation
import SwiftUI
import UserNotifications

/// Mac 건강 상태를 모니터링하고 브리핑을 생성하는 서비스
@Observable
final class HealthMonitor {
    static let shared = HealthMonitor()

    private let defaults = UserDefaults.standard
    private let notificationCenter = UNUserNotificationCenter.current()
    private let cleanReminderIdentifier = "schedule-clean"
    private let scanCompleteIdentifier = "scan-complete"
    private let lastCleanReminderDateKey = "lastCleanReminderDate"
    private let cleanReminderThrottle: TimeInterval = 24 * 3600

    var briefing: DailyBriefing?
    var lastCheckDate: Date?

    private init() {
        defaults.register(defaults: [
            "notificationsEnabled": true,
            "autoCleanEnabled": false,
            "autoCleanInterval": 7,
            "showMenuBarPercent": true
        ])
    }

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
        defaults.set(Date(), forKey: "lastCleanDate")
        defaults.removeObject(forKey: lastCleanReminderDateKey)

        DispatchQueue.main.async { [weak self] in
            self?.startScheduleIfEnabled()
        }
    }

    // MARK: - 정리 알림 스케줄

    private var scheduleTimer: Timer?

    /// 설정에 맞춰 기존 예약을 취소하고 정리 알림 체크를 다시 시작한다.
    func startScheduleIfEnabled(requestPermissionIfNeeded: Bool = false) {
        scheduleTimer?.invalidate()
        scheduleTimer = nil
        notificationCenter.removePendingNotificationRequests(
            withIdentifiers: [cleanReminderIdentifier]
        )

        guard defaults.bool(forKey: "notificationsEnabled") else {
            notificationCenter.removePendingNotificationRequests(
                withIdentifiers: [scanCompleteIdentifier]
            )
            return
        }

        if requestPermissionIfNeeded {
            requestNotificationPermission { [weak self] isAuthorized in
                guard isAuthorized else { return }
                DispatchQueue.main.async {
                    self?.startScheduleIfEnabled()
                }
            }
            return
        }

        guard defaults.bool(forKey: "autoCleanEnabled") else { return }
        let intervalDays = defaults.integer(forKey: "autoCleanInterval")
        guard intervalDays > 0 else { return }

        // 마지막 정리 시점 확인
        let lastClean = defaults.object(forKey: "lastCleanDate") as? Date ?? .distantPast
        let daysSinceClean = Date().timeIntervalSince(lastClean) / (24 * 3600)

        if daysSinceClean >= Double(intervalDays) {
            // 바로 알림
            sendCleanReminder()
        } else if let lastReminder = defaults.object(forKey: lastCleanReminderDateKey) as? Date,
                  lastClean >= lastReminder {
            defaults.removeObject(forKey: lastCleanReminderDateKey)
        }

        // 6시간마다 체크 (앱이 실행 중인 동안)
        scheduleTimer = Timer.scheduledTimer(withTimeInterval: 6 * 3600, repeats: true) { [weak self] _ in
            self?.checkSchedule()
        }
    }

    private func checkSchedule() {
        guard defaults.bool(forKey: "notificationsEnabled"),
              defaults.bool(forKey: "autoCleanEnabled") else { return }
        let intervalDays = defaults.integer(forKey: "autoCleanInterval")
        guard intervalDays > 0 else { return }
        let lastClean = defaults.object(forKey: "lastCleanDate") as? Date ?? .distantPast
        let daysSinceClean = Date().timeIntervalSince(lastClean) / (24 * 3600)

        guard daysSinceClean >= Double(intervalDays) else {
            if let lastReminder = defaults.object(forKey: lastCleanReminderDateKey) as? Date,
               lastClean >= lastReminder {
                defaults.removeObject(forKey: lastCleanReminderDateKey)
            }
            return
        }
        sendCleanReminder()
    }

    private func sendCleanReminder() {
        if let lastReminder = defaults.object(forKey: lastCleanReminderDateKey) as? Date,
           Date() < lastReminder.addingTimeInterval(cleanReminderThrottle) {
            return
        }

        let content = UNMutableNotificationContent()
        content.title = "BroomSweepy"
        content.body = "정리할 시점입니다. 파일은 자동으로 삭제되지 않습니다."
        content.sound = .default
        content.userInfo = ["destination": "dashboard"]

        let trigger = UNTimeIntervalNotificationTrigger(timeInterval: 1, repeats: false)
        let request = UNNotificationRequest(
            identifier: cleanReminderIdentifier,
            content: content,
            trigger: trigger
        )
        addIfAuthorized(request) { [weak self] wasScheduled in
            guard let self, wasScheduled else { return }
            self.defaults.set(Date(), forKey: self.lastCleanReminderDateKey)
        }
    }

    /// macOS 알림 발송
    func sendNotificationIfNeeded() {
        guard let b = briefing, b.healthScore < 70 else { return }

        let content = UNMutableNotificationContent()
        content.title = "BroomSweepy"
        content.body = b.recommendations.first?.text ?? "Mac 상태를 확인해보세요."
        content.sound = .default
        content.userInfo = ["destination": "dashboard"]

        let trigger = UNTimeIntervalNotificationTrigger(timeInterval: 1, repeats: false)
        let request = UNNotificationRequest(identifier: "health-\(Date())", content: content, trigger: trigger)

        addIfAuthorized(request)
    }

    /// 전체 스캔이 성공적으로 끝난 경우 한 번 호출한다.
    func sendScanCompletedNotification() {
        let content = UNMutableNotificationContent()
        content.title = "BroomSweepy 스캔 완료"
        content.body = "스캔이 끝났습니다. 정리 후보를 확인하세요."
        content.sound = .default
        content.userInfo = ["destination": "dashboard"]

        let trigger = UNTimeIntervalNotificationTrigger(timeInterval: 1, repeats: false)
        let request = UNNotificationRequest(
            identifier: scanCompleteIdentifier,
            content: content,
            trigger: trigger
        )
        addIfAuthorized(request)
    }

    /// 알림 권한 요청
    func requestNotificationPermission(completion: ((Bool) -> Void)? = nil) {
        guard defaults.bool(forKey: "notificationsEnabled") else {
            completion?(false)
            return
        }

        notificationCenter.requestAuthorization(options: [.alert, .sound]) { granted, error in
            completion?(granted && error == nil)
        }
    }

    private func addIfAuthorized(
        _ request: UNNotificationRequest,
        completion: ((Bool) -> Void)? = nil
    ) {
        guard defaults.bool(forKey: "notificationsEnabled") else {
            completion?(false)
            return
        }

        notificationCenter.getNotificationSettings { [weak self] settings in
            guard let self,
                  self.defaults.bool(forKey: "notificationsEnabled") else {
                completion?(false)
                return
            }

            switch settings.authorizationStatus {
            case .authorized, .provisional:
                self.notificationCenter.add(request) { error in
                    completion?(error == nil)
                }
            default:
                completion?(false)
            }
        }
    }
}
