import Foundation

/// 정리 이력을 기록하고 Before/After를 추적하는 서비스
@Observable
final class CleanHistory {
    static let shared = CleanHistory()

    struct CleanRecord: Codable, Identifiable {
        let id: UUID
        let date: Date
        let freedBytes: Int64
        let diskBefore: Int64
        let diskAfter: Int64
        let type: String   // "smart", "cache", "large", "duplicate", "manual"

        var freedFormatted: String { formatSize(freedBytes) }
        var dateFormatted: String {
            let f = DateFormatter()
            f.dateFormat = "M월 d일 HH:mm"
            return f.string(from: date)
        }
    }

    var records: [CleanRecord] = []

    private let key = "com.broomsweepy.cleanHistory"

    init() { load() }

    /// 정리 기록 추가
    func record(freed: Int64, type: String) {
        let disk = SystemMonitor.shared.getDiskInfo()
        let rec = CleanRecord(
            id: UUID(),
            date: Date(),
            freedBytes: freed,
            diskBefore: disk.usedSpace + freed,
            diskAfter: disk.usedSpace,
            type: type
        )
        records.insert(rec, at: 0)
        if records.count > 50 { records = Array(records.prefix(50)) }
        save()

        // 마지막 정리 날짜 업데이트
        UserDefaults.standard.set(Date(), forKey: "lastCleanDate")
    }

    /// 총 확보 용량
    var totalFreed: Int64 { records.reduce(0) { $0 + $1.freedBytes } }
    var totalFreedFormatted: String { formatSize(totalFreed) }

    /// 이번 달 확보 용량
    var monthlyFreed: Int64 {
        let cal = Calendar.current
        let startOfMonth = cal.date(from: cal.dateComponents([.year, .month], from: Date()))!
        return records.filter { $0.date >= startOfMonth }.reduce(0) { $0 + $1.freedBytes }
    }

    private func save() {
        if let data = try? JSONEncoder().encode(records) {
            UserDefaults.standard.set(data, forKey: key)
        }
    }

    private func load() {
        guard let data = UserDefaults.standard.data(forKey: key),
              let saved = try? JSONDecoder().decode([CleanRecord].self, from: data) else { return }
        records = saved
    }
}
