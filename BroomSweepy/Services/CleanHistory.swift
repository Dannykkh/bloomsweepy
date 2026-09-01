import Foundation

/// 휴지통으로 이동한 논리 용량 이력을 기록하는 서비스.
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
        var movedFormatted: String { formatSize(freedBytes) }
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
            // 휴지통을 비우기 전에는 실제 디스크 여유가 늘지 않는다.
            // 기존 저장 형식 호환을 위해 두 필드는 유지하되 값을 꾸미지 않는다.
            diskBefore: disk.usedSpace,
            diskAfter: disk.usedSpace,
            type: type
        )
        records.insert(rec, at: 0)
        if records.count > 50 { records = Array(records.prefix(50)) }
        save()

        // 마지막 정리 날짜 업데이트
        UserDefaults.standard.set(Date(), forKey: "lastCleanDate")
    }

    /// 총 휴지통 이동 논리 용량
    var totalFreed: Int64 { records.reduce(0) { $0 + $1.freedBytes } }
    var totalFreedFormatted: String { formatSize(totalFreed) }
    var totalMovedFormatted: String { formatSize(totalFreed) }

    /// 이번 달 휴지통 이동 논리 용량
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
