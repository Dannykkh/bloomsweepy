import { PanelTop, RefreshCw } from "lucide-react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef, useState } from "react";
import { useLanguage } from "../i18n";

interface MenuBarSettings {
  supported: boolean;
  available: boolean;
  showMemoryPercent: boolean;
}

export function MenuBarSettingsPanel() {
  const { t } = useLanguage();
  const [settings, setSettings] = useState<MenuBarSettings | null>(null);
  const [loading, setLoading] = useState(true);
  const [changing, setChanging] = useState(false);
  const [failed, setFailed] = useState(false);
  const [retry, setRetry] = useState(0);
  const inFlight = useRef(false);

  useEffect(() => {
    if (!isTauri()) { setLoading(false); return; }
    let disposed = false;
    let unlisten: (() => void) | undefined;
    let revision = 0;
    setLoading(true);
    setFailed(false);
    void (async () => {
      try {
        const stop = await listen<MenuBarSettings>("menu-bar-settings-changed", (event) => {
          revision += 1;
          if (!disposed) { setSettings(event.payload); setFailed(false); }
        });
        if (disposed) { stop(); return; }
        unlisten = stop;
        const requestedRevision = revision;
        const next = await invoke<MenuBarSettings>("get_menu_bar_settings");
        if (!disposed && requestedRevision === revision) setSettings(next);
      } catch {
        if (!disposed) setFailed(true);
      } finally {
        if (!disposed) setLoading(false);
      }
    })();
    return () => { disposed = true; unlisten?.(); };
  }, [retry]);

  async function change(enabled: boolean) {
    if (inFlight.current || loading || !settings?.available) return;
    inFlight.current = true;
    setChanging(true);
    setFailed(false);
    try {
      setSettings(await invoke<MenuBarSettings>("set_menu_bar_memory_percent", { enabled }));
    } catch {
      setFailed(true);
      // Read actual state; a failed response does not prove that a change failed.
      try { setSettings(await invoke<MenuBarSettings>("get_menu_bar_settings")); } catch { /* retry remains visible */ }
    } finally {
      inFlight.current = false;
      setChanging(false);
    }
  }

  async function openPanel() {
    try { await invoke("open_menu_bar_panel"); }
    catch { setFailed(true); }
  }

  if (!isTauri() || settings?.supported === false) return null;
  return <section className="settings-panel">
    <div className="settings-panel__heading">
      <PanelTop size={20} aria-hidden="true" />
      <div><h2>{t("메뉴 막대")}</h2><p>{t("창을 닫아도 메뉴 막대에서 상태를 확인하고 다시 열 수 있습니다.")}</p></div>
    </div>
    <label className="setting-row developer-tool-toggle">
      <span><strong>{t("메모리 사용량 표시")}</strong>
        <small>{t("아이콘 옆의 메모리 사용률만 숨기거나 표시합니다. 아이콘은 계속 남습니다.")}</small>
        <small>{t("CPU·메모리·시스템 디스크만 10초마다 확인하며 파일을 검사하지 않습니다.")}</small>
      </span>
      <span className="developer-tool-toggle__control">
        <input type="checkbox" name="menuBarMemoryPercent" role="switch" aria-label={t("메모리 사용량 표시")}
          checked={settings?.showMemoryPercent ?? false} disabled={loading || changing || !settings?.available}
          onChange={(event) => void change(event.currentTarget.checked)} />
        <strong>{loading ? t("확인 중…") : !settings?.available ? t("확인 불가") : settings.showMemoryPercent ? t("켜짐") : t("꺼짐")}</strong>
      </span>
    </label>
    <div className="settings-inline-state menu-bar-settings-actions">
      <p>{t("완전히 종료하려면 메뉴 막대 패널의 종료 또는 ⌘Q를 사용하세요.")}</p>
      <button type="button" className="secondary-button" disabled={loading || !settings?.available} onClick={() => void openPanel()}>{t("패널 열기")}</button>
    </div>
    {changing ? <p className="settings-inline-state" role="status">{t("메뉴 막대 설정 적용 중…")}</p> : null}
    {!loading && settings?.supported && !settings.available ? <p className="settings-inline-error" role="alert">{t("메뉴 막대를 준비하지 못했습니다. 앱을 다시 실행해 주세요.")}</p> : null}
    {failed ? <div className="settings-inline-error" role="alert">
      <p>{t("메뉴 막대 설정을 확인하지 못했습니다. 다시 확인해 주세요.")}</p>
      <button type="button" className="secondary-button" disabled={loading || changing} onClick={() => setRetry((value) => value + 1)}><RefreshCw size={16} aria-hidden="true" />{t("다시 확인")}</button>
    </div> : null}
  </section>;
}
