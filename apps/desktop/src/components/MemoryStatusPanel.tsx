import { MemoryStick, RefreshCw } from "lucide-react";
import { useEffect, useState } from "react";
import { useLanguage, type Translate } from "../i18n";
import { getSystemMemoryStatus } from "../lib/bridge";
import { formatBytes, formatDate } from "../lib/format";
import type { SystemMemoryStatus } from "../types";

export function MemoryStatusPanel() {
  const { t } = useLanguage();
  const [status, setStatus] = useState<SystemMemoryStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [showLoading, setShowLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let disposed = false;
    setLoading(true);
    setError(null);

    void getSystemMemoryStatus()
      .then((next) => {
        if (!disposed) setStatus(next);
      })
      .catch((reason: unknown) => {
        if (!disposed) setError(memoryStatusError(reason, t));
      })
      .finally(() => {
        if (!disposed) setLoading(false);
      });

    return () => {
      disposed = true;
    };
  }, [t]);

  useEffect(() => {
    if (!loading) {
      setShowLoading(false);
      return;
    }

    const timer = window.setTimeout(() => setShowLoading(true), 300);
    return () => window.clearTimeout(timer);
  }, [loading]);

  async function refresh() {
    if (loading) return;

    setLoading(true);
    setError(null);
    try {
      setStatus(await getSystemMemoryStatus());
    } catch (reason) {
      setError(memoryStatusError(reason, t));
    } finally {
      setLoading(false);
    }
  }

  const usedPercent = memoryUsagePercent(status);

  return (
    <section className="settings-panel memory-status-panel">
      <div className="settings-panel__heading">
        <MemoryStick size={20} aria-hidden="true" />
        <div>
          <h2>{t("시스템 메모리 상태")}</h2>
          <p>{t("운영체제가 보고하는 현재 메모리 수치를 읽습니다.")}</p>
        </div>
        <button
          type="button"
          className="icon-button memory-status-panel__refresh"
          aria-label={t("메모리 상태 새로 고침")}
          disabled={loading}
          onClick={() => void refresh()}
        >
          <RefreshCw className={showLoading ? "is-spinning" : undefined} size={16} aria-hidden="true" />
        </button>
      </div>

      {status ? (
        <div className="memory-status-panel__body">
          <div className="memory-status-panel__usage">
            <span>
              <strong>{t("사용 중")}</strong>
              <b>{t("{{percent}}% 사용", { percent: usedPercent })}</b>
            </span>
            <div
              className="memory-status-panel__meter"
              role="progressbar"
              aria-label={t("메모리 사용률")}
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={usedPercent}
            >
              <span style={{ width: `${usedPercent}%` }} />
            </div>
          </div>

          <dl className="memory-status-panel__metrics">
            <div>
              <dt>{t("전체 메모리")}</dt>
              <dd>{formatBytes(status.totalBytes)}</dd>
            </div>
            <div>
              <dt>{t("사용 중")}</dt>
              <dd>{formatBytes(status.usedBytes)}</dd>
            </div>
            <div>
              <dt>{t("사용 가능")}</dt>
              <dd>{formatBytes(status.availableBytes)}</dd>
            </div>
            <div>
              <dt>
                {isWindowsPlatform(status.platform)
                  ? t("스왑/커밋 추정")
                  : t("스왑 사용량")}
              </dt>
              <dd>{formatBytes(status.usedSwapBytes)} / {formatBytes(status.totalSwapBytes)}</dd>
            </div>
          </dl>

          {isWindowsPlatform(status.platform) ? (
            <p className="memory-status-panel__estimate-note">
              {t("Windows에서는 커밋 사용량에서 전체 물리 메모리를 뺀 추정치이며 페이지 파일의 실제 사용량이 아닙니다.")}
            </p>
          ) : null}

          <p className="memory-status-panel__captured">
            {t("{{platform}} · {{date}} 확인", {
              platform: platformLabel(status.platform),
              date: formatDate(status.capturedAtUnixMs),
            })}
          </p>
        </div>
      ) : null}

      {showLoading ? (
        <p className="settings-inline-state" role="status">
          <RefreshCw className="is-spinning" size={18} aria-hidden="true" />
          <span>{t("메모리 상태 확인 중…")}</span>
        </p>
      ) : null}

      {error ? <p className="settings-inline-error" role="alert">{error}</p> : null}

      <p className="memory-status-panel__notice">
        {t("이 화면은 상태만 읽습니다. 다른 앱의 메모리, 누수 메모리 또는 대기(standby) 메모리를 정리하지 않습니다.")}
      </p>
    </section>
  );
}

function memoryUsagePercent(status: SystemMemoryStatus | null): number {
  if (!status || status.totalBytes <= 0) return 0;
  return Math.min(100, Math.max(0, Math.round((status.usedBytes / status.totalBytes) * 100)));
}

function platformLabel(platform: string): string {
  if (isWindowsPlatform(platform)) return "Windows";
  const normalized = platform.toLowerCase();
  if (normalized.includes("mac") || normalized.includes("darwin")) return "macOS";
  return platform;
}

function isWindowsPlatform(platform: string): boolean {
  return platform.toLowerCase().includes("windows");
}

function memoryStatusError(reason: unknown, t: Translate): string {
  const detail =
    reason instanceof Error
      ? reason.message
      : typeof reason === "string"
        ? reason
        : t("알 수 없는 오류가 발생했습니다");
  return t("메모리 상태를 확인하지 못했습니다. {{detail}}", { detail });
}
