import Foundation
import IOKit.ps

final class SystemMonitor {
    static let shared = SystemMonitor()

    // MARK: - CPU Info

    struct CPUInfo {
        let usage: Double       // 0-100%
        let userUsage: Double
        let systemUsage: Double
    }

    private var previousCPUInfo: host_cpu_load_info?

    func getCPUInfo() -> CPUInfo {
        var cpuLoadInfo = host_cpu_load_info()
        var count = mach_msg_type_number_t(
            MemoryLayout<host_cpu_load_info>.size / MemoryLayout<integer_t>.size
        )

        let result = withUnsafeMutablePointer(to: &cpuLoadInfo) {
            $0.withMemoryRebound(to: integer_t.self, capacity: Int(count)) {
                host_statistics(mach_host_self(), HOST_CPU_LOAD_INFO, $0, &count)
            }
        }

        guard result == KERN_SUCCESS else {
            return CPUInfo(usage: 0, userUsage: 0, systemUsage: 0)
        }

        let user = Double(cpuLoadInfo.cpu_ticks.0)
        let system = Double(cpuLoadInfo.cpu_ticks.1)
        let idle = Double(cpuLoadInfo.cpu_ticks.2)
        let nice = Double(cpuLoadInfo.cpu_ticks.3)

        if let prev = previousCPUInfo {
            let dUser = user - Double(prev.cpu_ticks.0)
            let dSystem = system - Double(prev.cpu_ticks.1)
            let dIdle = idle - Double(prev.cpu_ticks.2)
            let dNice = nice - Double(prev.cpu_ticks.3)
            let total = dUser + dSystem + dIdle + dNice

            previousCPUInfo = cpuLoadInfo
            if total > 0 {
                return CPUInfo(
                    usage: ((dUser + dSystem + dNice) / total) * 100,
                    userUsage: (dUser / total) * 100,
                    systemUsage: (dSystem / total) * 100
                )
            }
        }

        previousCPUInfo = cpuLoadInfo
        let total = user + system + idle + nice
        guard total > 0 else { return CPUInfo(usage: 0, userUsage: 0, systemUsage: 0) }
        return CPUInfo(
            usage: ((user + system + nice) / total) * 100,
            userUsage: (user / total) * 100,
            systemUsage: (system / total) * 100
        )
    }

    // MARK: - Battery Info

    struct BatteryInfo {
        let isPresent: Bool
        let percentage: Int
        let isCharging: Bool
        let cycleCount: Int
        let health: String       // "Good", "Fair", "Poor"
        let timeRemaining: Int?  // minutes
    }

    func getBatteryInfo() -> BatteryInfo {
        guard let snapshot = IOPSCopyPowerSourcesInfo()?.takeRetainedValue(),
              let sources = IOPSCopyPowerSourcesList(snapshot)?.takeRetainedValue() as? [CFTypeRef],
              let first = sources.first,
              let desc = IOPSGetPowerSourceDescription(snapshot, first)?.takeUnretainedValue() as? [String: Any]
        else {
            return BatteryInfo(isPresent: false, percentage: 0, isCharging: false,
                               cycleCount: 0, health: "N/A", timeRemaining: nil)
        }

        let isCharging = (desc[kIOPSPowerSourceStateKey] as? String) == kIOPSACPowerValue
        let percentage = desc[kIOPSCurrentCapacityKey] as? Int ?? 0
        let maxCapacity = desc[kIOPSMaxCapacityKey] as? Int ?? 100
        let designCapacity = desc[kIOPSDesignCapacityKey] as? Int ?? maxCapacity

        let healthRatio = designCapacity > 0 ? Double(maxCapacity) / Double(designCapacity) : 1.0
        let healthStr: String
        if healthRatio >= 0.8 {
            healthStr = "양호"
        } else if healthRatio >= 0.5 {
            healthStr = "보통"
        } else {
            healthStr = "교체 권장"
        }

        let timeRemaining: Int?
        let rawTime = IOPSGetTimeRemainingEstimate()
        if rawTime == kIOPSTimeRemainingUnlimited {
            timeRemaining = nil
        } else if rawTime == kIOPSTimeRemainingUnknown {
            timeRemaining = nil
        } else {
            timeRemaining = Int(rawTime / 60)
        }

        // Cycle count from IOKit service
        let cycleCount = Self.getBatteryCycleCount()

        return BatteryInfo(
            isPresent: true,
            percentage: percentage,
            isCharging: isCharging,
            cycleCount: cycleCount,
            health: healthStr,
            timeRemaining: timeRemaining
        )
    }

    private static func getBatteryCycleCount() -> Int {
        let service = IOServiceGetMatchingService(
            kIOMasterPortDefault,
            IOServiceMatching("AppleSmartBattery")
        )
        guard service != IO_OBJECT_NULL else { return 0 }
        defer { IOObjectRelease(service) }

        if let prop = IORegistryEntryCreateCFProperty(service, "CycleCount" as CFString, nil, 0) {
            return prop.takeRetainedValue() as? Int ?? 0
        }
        return 0
    }

    // MARK: - Disk Info

    struct DiskInfo {
        let totalSpace: Int64
        let freeSpace: Int64
        let usedSpace: Int64

        var usagePercent: Double {
            guard totalSpace > 0 else { return 0 }
            return Double(usedSpace) / Double(totalSpace) * 100
        }

        var totalFormatted: String { formatSize(totalSpace) }
        var freeFormatted: String { formatSize(freeSpace) }
        var usedFormatted: String { formatSize(usedSpace) }
    }

    func getDiskInfo() -> DiskInfo {
        do {
            let attrs = try FileManager.default.attributesOfFileSystem(forPath: "/")
            let total = (attrs[.systemSize] as? Int64) ?? 0
            let free = (attrs[.systemFreeSize] as? Int64) ?? 0
            return DiskInfo(totalSpace: total, freeSpace: free, usedSpace: total - free)
        } catch {
            return DiskInfo(totalSpace: 0, freeSpace: 0, usedSpace: 0)
        }
    }
}
