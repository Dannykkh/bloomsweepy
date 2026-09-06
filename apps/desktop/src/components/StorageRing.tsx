import { Database, Search } from "lucide-react";
import { useId } from "react";
import { formatBytes } from "../lib/format";
import type { ScanReport, VolumeInfo } from "../types";
import { useLanguage } from "../i18n";

interface StorageRingProps {
  volume: VolumeInfo | null;
  report: ScanReport | null;
  scanning: boolean;
  loading?: boolean;
  status?: string | null;
}

export function StorageRing({
  volume,
  report,
  scanning,
  loading = false,
  status,
}: StorageRingProps) {
  const { t } = useLanguage();
  const gradientId = useId().replace(/:/g, "");
  const usedBytes = volume ? Math.max(0, volume.totalBytes - volume.availableBytes) : 0;
  const usedPercent = volume?.totalBytes
    ? Math.min(100, Math.max(0, (usedBytes / volume.totalBytes) * 100))
    : 0;
  const statusText = status
    ?? (loading && !volume
      ? `${t("디스크 확인 중")}…`
      : scanning
        ? t("파일을 확인하는 중")
        : report
          ? t("{{count}}개 파일 확인", { count: report.totalFiles.toLocaleString() })
          : t("분석 준비 완료"));
  const diskLabel = volume
    ? t("디스크 사용률 {{percent}}퍼센트, {{size}} 남음", {
        percent: Math.round(usedPercent),
        size: formatBytes(volume.availableBytes),
      })
    : loading
      ? `${t("디스크 확인 중")}…`
      : t("디스크를 선택하지 않음");

  return (
    <div
      className="storage-ring storage-ring--dashboard"
      role="img"
      aria-label={`${diskLabel}. ${statusText}`}
      aria-busy={loading || scanning}
      data-state={loading && !volume ? "loading" : scanning ? "scanning" : volume ? "ready" : "empty"}
    >
      <svg viewBox="0 0 220 220" aria-hidden="true">
        <defs>
          <linearGradient id={gradientId} x1="0" y1="0" x2="1" y2="1">
            <stop offset="0" stopColor="var(--accent-blue)" />
            <stop offset="1" stopColor="var(--accent-violet)" />
          </linearGradient>
        </defs>
        <circle className="storage-ring__track" cx="110" cy="110" r="88" />
        <circle
          className="storage-ring__value"
          cx="110"
          cy="110"
          r="88"
          pathLength="100"
          style={{ stroke: `url(#${gradientId})` }}
          strokeDasharray={`${usedPercent} ${100 - usedPercent}`}
        />
      </svg>
      <div className="storage-ring__content">
        <span className="storage-ring__icon" aria-hidden="true">
          {scanning ? <Search size={24} /> : <Database size={24} />}
        </span>
        <strong translate={volume || loading ? undefined : "no"}>
          {volume
            ? formatBytes(volume.availableBytes)
            : loading
              ? `${t("확인 중")}…`
              : "BroomSweepy"}
        </strong>
        <span className="storage-ring__label">
          {volume ? t("남음") : statusText}
        </span>
        {volume ? (
          <small className="storage-ring__meta">
            {t("{{percent}}% 사용", { percent: Math.round(usedPercent) })}
          </small>
        ) : null}
      </div>
      <span className="storage-ring__status" aria-hidden="true">{statusText}</span>
    </div>
  );
}
