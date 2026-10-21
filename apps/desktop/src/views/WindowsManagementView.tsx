import { Activity, AlertTriangle, ArrowUpRight, CheckCircle2, ChevronDown, ExternalLink, FolderCog, HardDrive, Info, RefreshCw } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useLanguage, type MessageKey } from "../i18n";
import { formatDate } from "../lib/format";
import { canOpenWindowsSettingsTarget, getWindowsSettings, openWindowsSettingsTarget, windowsSettingStateMessage, windowsSettingTitle, windowsStoredValue, type WindowsSetting, type WindowsSettingsSnapshot, type WindowsSettingsTargetId } from "../lib/windowsSettings";
import type { ViewId } from "../types";
import "./WindowsManagementView.css";

interface Props { onNavigate?: (view: ViewId) => void }

const settingDescriptions: Record<string, MessageKey> = {
  explorer_hide_extensions: "파일 탐색기의 알려진 파일 확장자 숨김 저장값입니다. 실제 표시 상태를 추정하지 않습니다.",
  explorer_hidden_files: "파일 탐색기의 숨김 파일 표시 저장값입니다. 숨김 항목의 삭제를 허용하지 않습니다.",
  explorer_protected_files: "보호된 운영체제 파일 표시와 관련된 저장값입니다. 시스템 파일은 변경하지 않습니다.",
  explorer_full_path: "파일 탐색기 제목 표시줄의 전체 경로 저장값입니다. 실제 화면 반영은 별도입니다.",
  legacy_copilot: "폐기된 이전 Copilot 정책입니다. 새 Copilot 앱의 설치나 동작 상태를 뜻하지 않습니다.",
  recall_snapshots: "컴퓨터 전체의 Recall 저장 정책값입니다. 현재 사용자의 동의나 스냅샷 상태와 합성하지 않습니다.",
  recall_snapshots_user: "현재 사용자의 Recall 저장 정책값입니다. 컴퓨터 정책과 별도로 표시하며 실제 적용 우선순위를 추정하지 않습니다.",
  recall_availability: "Recall 사용 가능 정책의 저장값입니다. 설치 여부나 이 기기의 지원 여부를 확정하지 않습니다.",
  notepad_ai: "메모장 AI 기능의 컴퓨터 정책값입니다. 운영체제와 메모장 버전에 따른 지원 확인이 필요합니다.",
  paint_image_creator: "그림판 이미지 생성 정책의 저장값입니다. 운영체제 빌드와 에디션에 따른 지원 확인이 필요합니다.",
};

const targetCopy: Record<WindowsSettingsTargetId, MessageKey> = {
  folder_options: "파일 탐색기 옵션",
  storage: "저장소 설정",
  startup: "시작 프로그램 설정",
  apps: "설치된 앱",
  taskbar: "작업 표시줄 설정",
  privacy: "개인 정보 설정",
  about: "시스템 정보",
  optional_features: "선택적 기능",
};

export function WindowsManagementView({ onNavigate }: Props) {
  const { t, appText } = useLanguage();
  const [snapshot, setSnapshot] = useState<WindowsSettingsSnapshot | null>(null);
  const [busy, setBusy] = useState<string | null>("refresh");
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<MessageKey | null>(null);
  const [stale, setStale] = useState(false);
  const [checkedAt, setCheckedAt] = useState<number | null>(null);
  const mounted = useRef(false);
  const generation = useRef(0);
  const requestLock = useRef(false);
  const pendingRead = useRef<Promise<WindowsSettingsSnapshot> | null>(null);

  useEffect(() => {
    mounted.current = true;
    void refresh(true);
    return () => { mounted.current = false; generation.current += 1; };
  }, []);

  async function refresh(reusePending = false) {
    if (requestLock.current && !(reusePending && pendingRead.current)) return;
    requestLock.current = true;
    const request = ++generation.current;
    setBusy("refresh");
    setError(null);
    setNotice(null);
    // Reusing the promise preserves single-flight reads across StrictMode effect reconnection.
    const promise = pendingRead.current ?? Promise.resolve().then(getWindowsSettings);
    pendingRead.current = promise;
    try {
      const result = await promise;
      if (!mounted.current || request !== generation.current) return;
      setSnapshot(result);
      setStale(false);
      setCheckedAt(Date.now());
      setNotice("설정 상태를 새로 확인했습니다.");
    } catch (reason) {
      if (!mounted.current || request !== generation.current) return;
      setError(errorDetail(reason));
      setStale(snapshot !== null);
    } finally {
      if (pendingRead.current === promise) pendingRead.current = null;
      if (mounted.current && request === generation.current) {
        requestLock.current = false;
        setBusy(null);
      }
    }
  }

  async function openTarget(targetId: string) {
    if (requestLock.current || !snapshot || !canOpenWindowsSettingsTarget(snapshot, targetId)) return;
    requestLock.current = true;
    const request = ++generation.current;
    setBusy(targetId);
    setError(null);
    setNotice(null);
    try {
      const result = await openWindowsSettingsTarget(targetId);
      if (!mounted.current || request !== generation.current) return;
      if (!result.requested || result.changed !== false || result.targetId !== targetId) {
        setError(t("설정 화면 열기 요청을 확인하지 못했습니다."));
        return;
      }
      setNotice("요청을 전달했습니다. 설정 변경 완료를 의미하지 않습니다.");
    } catch (reason) {
      if (!mounted.current || request !== generation.current) return;
      setError(errorDetail(reason));
    } finally {
      if (mounted.current && request === generation.current) {
        requestLock.current = false;
        setBusy(null);
      }
    }
  }

  function settingTitle(setting: WindowsSetting) {
    return t(windowsSettingTitle(setting.id) ?? "알 수 없는 설정");
  }

  function renderSetting(setting: WindowsSetting) {
    const description = Object.prototype.hasOwnProperty.call(settingDescriptions, setting.id) ? settingDescriptions[setting.id] : undefined;
    const value = windowsStoredValue(setting);
    const support: MessageKey = setting.support === "deprecated" ? "폐기된 정책"
      : setting.support === "requiresVerification" ? "지원 확인 필요" : "저장값만 확인";
    const issue = setting.state !== "value" && setting.state !== "unset";
    return <li className="windows-management-setting" key={setting.id}>
      <div className="windows-management-setting__copy">
        <h4 id={`windows-setting-${setting.id}`}>{settingTitle(setting)}</h4>
        <p>{t(description ?? "알려진 설정 목록에 없는 항목입니다. 실제 적용 상태를 추정하지 않습니다.")}</p>
        <div className="windows-management-setting__metadata">
          <span>{t(setting.scope === "machine" ? "컴퓨터 전체 정책" : "현재 사용자")}</span>
          <span className={setting.support === "storedValueOnly" ? "" : "has-issue"}>{t(support)}</span>
        </div>
      </div>
      <div className={`windows-management-setting__value${issue ? " has-issue" : ""}`}>
        <span>{t(windowsSettingStateMessage(setting))}</span>
        {value !== null ? <strong>{value}</strong> : null}
        {setting.valueType ? <small>{setting.valueType}</small> : null}
        {setting.category === "aiPolicy" && setting.targetId && snapshot ? <button type="button" className="secondary-button" disabled={!!busy || !canOpenWindowsSettingsTarget(snapshot, setting.targetId)} onClick={() => void openTarget(setting.targetId!)} aria-label={`${settingTitle(setting)} · ${t("관련 설정 열기")}`}><ExternalLink size={16} aria-hidden="true" />{t("관련 설정 열기")}</button> : null}
      </div>
    </li>;
  }

  const explorer = snapshot?.settings.filter((setting) => setting.category === "explorer") ?? [];
  const aiPolicies = snapshot?.settings.filter((setting) => setting.category === "aiPolicy") ?? [];
  const availableTargets = Object.keys(targetCopy) as WindowsSettingsTargetId[];

  return <div className="windows-management-view">
    <header className="windows-management-header">
      <div><h2><FolderCog size={25} aria-hidden="true" />{t("탐색기와 시스템 설정")}</h2><p>{t("저장된 설정값을 확인하고 공식 설정 화면으로 이동합니다.")}</p></div>
      <button type="button" className="secondary-button" disabled={!!busy} onClick={() => void refresh()}><RefreshCw size={17} aria-hidden="true" />{busy === "refresh" ? t("상태 확인 중…") : t("상태 새로 확인")}</button>
    </header>

    <section className="windows-management-scope" aria-label={t("관리 범위")}>
      <Info size={20} aria-hidden="true" /><div><strong>{t("조회와 공식 설정 연결만 제공합니다.")}</strong><p>{t("저장값은 실제 기능의 켜짐·꺼짐이나 정책 적용 여부가 아닙니다. 이 앱은 Windows 설정을 직접 변경하지 않습니다.")}</p></div>
    </section>

    {snapshot ? <dl className="windows-management-platform" aria-label={t("현재 운영체제 정보")}>
      <div><dt>{t("운영체제")}</dt><dd>{metadata(snapshot.platform, t("미확인"))}</dd></div>
      <div><dt>{t("빌드")}</dt><dd>{metadata(snapshot.osBuild, t("미확인"))}</dd></div>
      <div><dt>{t("개정")}</dt><dd>{typeof snapshot.osRevision === "number" && Number.isInteger(snapshot.osRevision) && snapshot.osRevision >= 0 ? snapshot.osRevision : t("미확인")}</dd></div>
      <div><dt>{t("에디션")}</dt><dd>{metadata(snapshot.edition, t("미확인"))}</dd></div>
    </dl> : null}

    <div className="windows-management-status" role="status" aria-live="polite">
      {busy ? <><RefreshCw size={17} aria-hidden="true" /><span>{busy === "refresh" ? t("Windows 설정값을 읽고 있습니다. 변경하지 않습니다.") : t("설정 화면 열기를 요청하고 있습니다.")}</span></> : notice ? <><CheckCircle2 size={17} aria-hidden="true" /><span>{t(notice)}</span></> : null}
    </div>

    {error ? <div className="windows-management-error" role="alert"><AlertTriangle size={20} aria-hidden="true" /><div><strong>{t("Windows 설정을 확인하거나 열지 못했습니다.")}</strong><p>{appText(error)}</p>{stale ? <p>{t("표시된 값은 이전 조회 결과입니다. 상태를 새로 확인하세요.")}</p> : null}</div></div> : null}

    {snapshot && !snapshot.supported ? <section className="windows-management-unsupported"><Info size={22} aria-hidden="true" /><div><h3>{t("이 운영체제에서는 Windows 설정을 지원하지 않습니다.")}</h3><p>{t("Mac에서는 맥 설정을 이용하세요. 다른 관리 기능은 해당 운영체제 기능으로 제공됩니다.")}</p></div></section> : null}

    {snapshot?.supported ? <>
      <section className="windows-management-targets" aria-labelledby="windows-targets-title">
        <div className="windows-management-section-heading"><h3 id="windows-targets-title">{t("공식 설정 화면")}</h3></div>
        <p>{t("화면 열기를 요청할 뿐, 실제 창 열림이나 설정 변경 완료를 확인한 것은 아닙니다.")}</p>
        <div className="windows-management-target-list">{availableTargets.map((targetId) => <button key={targetId} type="button" className="windows-management-target" disabled={!!busy || !canOpenWindowsSettingsTarget(snapshot, targetId)} onClick={() => void openTarget(targetId)} aria-label={`${t(targetCopy[targetId])} · ${t("설정 화면 열기")}`}><span>{t(targetCopy[targetId])}{!canOpenWindowsSettingsTarget(snapshot, targetId) ? <small>{t("연결 사용 불가")}</small> : null}</span><ExternalLink size={17} aria-hidden="true" /></button>)}</div>
      </section>

      <section className={stale ? "windows-management-settings is-stale" : "windows-management-settings"} aria-labelledby="windows-explorer-title" aria-busy={busy === "refresh"}>
        <div className="windows-management-section-heading"><h3 id="windows-explorer-title">{t("파일 탐색기 설정값")}</h3>{checkedAt ? <span>{t("마지막 확인")} · <time dateTime={new Date(checkedAt).toISOString()}>{formatDate(checkedAt)}</time>{stale ? ` · ${t("새로 확인 필요")}` : ""}</span> : null}</div>
        <ul className="windows-management-setting-list">{explorer.map(renderSetting)}</ul>
        <div className="windows-management-section-action"><button type="button" className="secondary-button" disabled={!!busy || !canOpenWindowsSettingsTarget(snapshot, "folder_options")} onClick={() => void openTarget("folder_options")}><ExternalLink size={17} aria-hidden="true" />{t("파일 탐색기 옵션 열기")}</button></div>
      </section>

      <details className="windows-management-policies" aria-busy={busy === "refresh"}>
        <summary><span><strong>{t("Windows AI 정책 저장값 {{count}}개", { count: aiPolicies.length })}</strong><small>{t("실제 적용 상태 미확인")} · {t("지원 확인 필요")}</small></span><ChevronDown size={20} aria-hidden="true" /></summary>
        <p className="windows-management-footnote">{t("정책은 Windows 빌드·에디션·앱 버전에 따라 지원이 다릅니다. 미설정은 기능이 꺼졌다는 뜻이 아닙니다.")}</p>
        <ul className="windows-management-setting-list">{aiPolicies.map(renderSetting)}</ul>
      </details>
    </> : null}

    {snapshot?.warnings.length ? <details className="windows-management-warnings"><summary><AlertTriangle size={17} aria-hidden="true" />{t("확인할 사항")} ({snapshot.warnings.length})</summary><ul>{snapshot.warnings.map((warning, index) => <li key={index}>{appText(warning)}</li>)}</ul></details> : null}

    {onNavigate ? <section className="windows-management-connections" aria-label={t("다른 관리 기능")}><h3>{t("상태와 저장공간 관리")}</h3><div>{([{ view: "performance", label: "시스템 성능", icon: Activity }, { view: "applications", label: "앱 관리", icon: FolderCog }, { view: "cleanup", label: "공간 정리", icon: HardDrive }] as const).map(({ view, label, icon: Icon }) => <button type="button" className="windows-management-target" key={view} onClick={() => onNavigate(view)}><Icon size={20} aria-hidden="true" /><span>{t(label)}</span><ArrowUpRight size={17} aria-hidden="true" /></button>)}</div></section> : null}
  </div>;
}

function metadata(value: string | null, unknown: string): string {
  const text = value?.trim();
  return text && text.toLowerCase() !== "unknown" ? text.slice(0, 160) : unknown;
}

function errorDetail(reason: unknown): string {
  if (reason instanceof Error) return reason.message.slice(0, 600);
  if (typeof reason === "string") return reason.slice(0, 600);
  return "Windows settings request failed";
}
