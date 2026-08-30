import { Database, Search } from "lucide-react";
import { formatBytes } from "../lib/format";
import type { ScanReport, VolumeInfo } from "../types";

interface StorageRingProps {
  volume: VolumeInfo | null;
  report: ScanReport | null;
  scanning: boolean;
}

export function StorageRing({ volume, report, scanning }: StorageRingProps) {
  const usedBytes = volume ? volume.totalBytes - volume.availableBytes : 0;
  const usedPercent = volume?.totalBytes
    ? Math.min(100, Math.max(0, (usedBytes / volume.totalBytes) * 100))
    : 0;
  const status = scanning
    ? "파일을 확인하는 중"
    : report
      ? `${report.totalFiles.toLocaleString("ko-KR")}개 파일 확인`
      : "분석 준비 완료";

  return (
    <div
      className="storage-ring"
      role="img"
      aria-label={
        volume
          ? `디스크 사용률 ${Math.round(usedPercent)}퍼센트, ${formatBytes(volume.availableBytes)} 남음`
          : "디스크를 선택하지 않음"
      }
    >
      <svg viewBox="0 0 220 220" aria-hidden="true">
        <defs>
          <linearGradient id="storage-sweep" x1="0" y1="0" x2="1" y2="1">
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
          strokeDasharray={`${usedPercent} ${100 - usedPercent}`}
        />
      </svg>
      <div className="storage-ring__content">
        <span className="storage-ring__icon" aria-hidden="true">
          {scanning ? <Search size={22} /> : <Database size={22} />}
        </span>
        <strong>{volume ? `${Math.round(usedPercent)}% 사용` : "BroomSweepy"}</strong>
        <span>{status}</span>
      </div>
    </div>
  );
}
