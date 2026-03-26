import Foundation

final class MemoryManager {
    static let shared = MemoryManager()

    struct MemoryInfo {
        let total: UInt64        // 전체 RAM
        let used: UInt64         // 사용 중
        let free: UInt64         // 여유
        let wired: UInt64        // 고정 (커널)
        let compressed: UInt64   // 압축된 메모리
        let appMemory: UInt64    // 앱 사용

        var usagePercent: Double {
            Double(used) / Double(total) * 100
        }
        var totalFormatted: String { ByteCountFormatter.string(fromByteCount: Int64(total), countStyle: .memory) }
        var usedFormatted: String { ByteCountFormatter.string(fromByteCount: Int64(used), countStyle: .memory) }
        var freeFormatted: String { ByteCountFormatter.string(fromByteCount: Int64(free), countStyle: .memory) }
    }

    func getMemoryInfo() -> MemoryInfo {
        let total = ProcessInfo.processInfo.physicalMemory

        var stats = vm_statistics64()
        var count = mach_msg_type_number_t(MemoryLayout<vm_statistics64>.size / MemoryLayout<integer_t>.size)
        let pageSize = UInt64(vm_kernel_page_size)

        let result = withUnsafeMutablePointer(to: &stats) {
            $0.withMemoryRebound(to: integer_t.self, capacity: Int(count)) {
                host_statistics64(mach_host_self(), HOST_VM_INFO64, $0, &count)
            }
        }

        guard result == KERN_SUCCESS else {
            return MemoryInfo(total: total, used: 0, free: total, wired: 0, compressed: 0, appMemory: 0)
        }

        let wired = UInt64(stats.wire_count) * pageSize
        let compressed = UInt64(stats.compressor_page_count) * pageSize
        let active = UInt64(stats.active_count) * pageSize
        let used = active + wired + compressed

        return MemoryInfo(
            total: total,
            used: used,
            free: total - used,
            wired: wired,
            compressed: compressed,
            appMemory: active
        )
    }

    /// 메모리 정리 — 메모리 압박을 유도하여 비활성 페이지 해제
    func purgeMemory(progressCallback: ((String) -> Void)? = nil) -> (before: MemoryInfo, after: MemoryInfo) {
        let before = getMemoryInfo()

        // 1단계: 앱/시스템 캐시 정리
        progressCallback?("캐시 정리 중...")
        URLCache.shared.removeAllCachedResponses()
        URLCache.shared.diskCapacity = 0
        URLCache.shared.memoryCapacity = 0
        // 복원
        URLCache.shared.diskCapacity = 50_000_000
        URLCache.shared.memoryCapacity = 10_000_000

        // 2단계: 메모리 압박 유도 (점진적 할당→즉시 해제 방식)
        // 한번에 큰 블록을 잡지 않고 작은 청크를 반복하여 시스템 부담 최소화
        progressCallback?("비활성 메모리 회수 중...")
        let totalRAM = ProcessInfo.processInfo.physicalMemory
        let freeBytes = before.free
        // 최대 전체 RAM의 10% 또는 여유 메모리의 50% (더 작은 쪽)
        let allocSize = min(freeBytes / 2, totalRAM / 10)

        if allocSize > 50_000_000 { // 50MB 이상일 때만
            let chunkSize = 32 * 1024 * 1024 // 32MB 청크 (작게)
            let rounds = 3 // 3번 반복 (할당→해제→할당→해제...)

            for round in 0..<rounds {
                progressCallback?("메모리 회수 중... (\(round + 1)/\(rounds))")
                var chunks: [UnsafeMutableRawPointer] = []
                var allocated: UInt64 = 0

                while allocated < allocSize {
                    let size = min(chunkSize, Int(allocSize - allocated))
                    guard let ptr = malloc(size) else { break }
                    memset(ptr, 0, size)
                    chunks.append(ptr)
                    allocated += UInt64(size)
                }

                // 즉시 해제
                for ptr in chunks { free(ptr) }
                chunks.removeAll()

                // 시스템이 페이지 회수할 시간
                Thread.sleep(forTimeInterval: 0.3)
            }
        }

        // 3단계: malloc 트림 (프래그먼트 정리)
        progressCallback?("메모리 최적화 중...")
        malloc_zone_pressure_relief(nil, 0)

        // 시스템 회수 대기
        Thread.sleep(forTimeInterval: 2.0)

        let after = getMemoryInfo()
        return (before, after)
    }
}
