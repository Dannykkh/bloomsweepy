import { Power, LoaderCircle } from "lucide-react";
import { useEffect, useState } from "react";
import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
import { useLanguage, type Translate } from "../i18n";

export function StartupSettingsPanel() {
  const { t } = useLanguage();
  const [enabled, setEnabled] = useState<boolean | null>(null);
  const [loading, setLoading] = useState(true);
  const [changing, setChanging] = useState(false);
  const [showWorking, setShowWorking] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let disposed = false;
    setLoading(true);
    setEnabled(null);
    setError(null);

    void isEnabled()
      .then((next) => {
        if (!disposed) setEnabled(next);
      })
      .catch((reason: unknown) => {
        if (!disposed) setError(startupError(reason, t));
      })
      .finally(() => {
        if (!disposed) setLoading(false);
      });

    return () => {
      disposed = true;
    };
  }, [t]);

  useEffect(() => {
    if (!loading && !changing) {
      setShowWorking(false);
      return;
    }

    const timer = window.setTimeout(() => setShowWorking(true), 300);
    return () => window.clearTimeout(timer);
  }, [changing, loading]);

  async function changeStartup(nextEnabled: boolean) {
    if (loading || changing) return;

    setChanging(true);
    setError(null);
    try {
      if (nextEnabled) {
        await enable();
      } else {
        await disable();
      }

      const actualEnabled = await isEnabled();
      setEnabled(actualEnabled);
      if (actualEnabled !== nextEnabled) {
        throw new Error(t("요청한 자동 시작 상태가 운영체제에 반영되지 않았습니다."));
      }
    } catch (reason) {
      try {
        setEnabled(await isEnabled());
      } catch {
        setEnabled(null);
      }
      setError(startupError(reason, t));
    } finally {
      setChanging(false);
    }
  }

  return (
    <section className="settings-panel startup-settings-panel">
      <div className="settings-panel__heading">
        <Power size={20} aria-hidden="true" />
        <div>
          <h2>{t("로그인 시 자동 시작")}</h2>
          <p>{t("운영체제에 등록된 실제 시작 상태를 표시합니다.")}</p>
        </div>
      </div>

      <label className="setting-row developer-tool-toggle">
        <span>
          <strong>{t("BroomSweepy 자동 시작")}</strong>
          <small>
            {t("기본값은 사용 안 함입니다. 켜면 로그인할 때 창을 띄우지 않고 백그라운드에서 시작합니다.")}
          </small>
          <small>{t("Windows 시작 앱 또는 macOS 로그인 항목 설정에서도 끌 수 있습니다.")}</small>
        </span>
        <span className="developer-tool-toggle__control">
          <input
            type="checkbox"
            name="autostartEnabled"
            role="switch"
            aria-label={t("자동 시작 사용")}
            checked={enabled ?? false}
            disabled={loading || changing}
            onChange={(event) => void changeStartup(event.currentTarget.checked)}
          />
          <strong>
            {enabled === null ? t("확인 불가") : enabled ? t("켜짐") : t("꺼짐")}
          </strong>
        </span>
      </label>

      {showWorking ? (
        <div className="settings-inline-state" role="status">
          <LoaderCircle className="is-spinning" size={18} aria-hidden="true" />
          <span>{changing ? t("시작 설정 적용 중…") : t("시작 설정 확인 중…")}</span>
        </div>
      ) : null}

      {error ? <p className="settings-inline-error" role="alert">{error}</p> : null}
    </section>
  );
}

function startupError(reason: unknown, t: Translate): string {
  const detail =
    reason instanceof Error
      ? reason.message
      : typeof reason === "string"
        ? reason
        : t("알 수 없는 오류가 발생했습니다");
  return t("자동 시작 설정을 처리하지 못했습니다. {{detail}}", { detail });
}
