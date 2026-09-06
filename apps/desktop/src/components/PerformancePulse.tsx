import { Activity, ArrowRight, MemoryStick } from "lucide-react";
import { usePerformanceMonitor } from "../hooks/usePerformanceMonitor";
import { memoryUsagePercent } from "../lib/performancePresentation";
import { useLanguage } from "../i18n";

export function PerformancePulse({ onOpen }: { onOpen: () => void }) {
  const { t } = useLanguage();
  const monitor = usePerformanceMonitor(5_000);
  const cpu = monitor.snapshot ? Math.round(monitor.snapshot.cpuUsagePercent) : null;
  const memory = monitor.snapshot ? Math.round(memoryUsagePercent(monitor.snapshot)) : null;

  return (
    <button
      className={`dashboard-performance-pulse ${monitor.stale ? "is-stale" : ""}`}
      type="button"
      aria-label={t("CPU와 메모리 상세 보기")}
      onClick={onOpen}
    >
      <span className="dashboard-performance-pulse__icon" aria-hidden="true">
        <Activity size={17} />
      </span>
      <span className="dashboard-performance-pulse__copy">
        <small>{t("시스템 성능")}</small>
        <strong>
          {cpu === null || memory === null
            ? monitor.error
              ? t("측정 불가")
              : t("측정 준비 중…")
            : t("CPU {{cpu}}% · 메모리 {{memory}}%", { cpu, memory })}
        </strong>
      </span>
      <span className="dashboard-performance-pulse__metric" aria-hidden="true">
        <MemoryStick size={15} />
        {memory === null ? "—" : `${memory}%`}
      </span>
      <ArrowRight size={16} aria-hidden="true" />
    </button>
  );
}
