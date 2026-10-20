import { AlertTriangle, CheckCircle2, ChevronDown, Eye, FileType, FolderCog, History, Info, ListFilter, PanelBottom, RefreshCw, RotateCcw, Route, type LucideIcon } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useLanguage, type MessageKey } from "../i18n";
import { formatDate } from "../lib/format";
import { applyMacSetting, canStageMacSetting, getMacSettings, hasMacSettingChange, restoreMacSetting, type MacSetting, type MacSettingsSnapshot } from "../lib/macSettings";
import "./MacManagementView.css";

const settingCopy: Record<string, { title: MessageKey; description: MessageKey }> = {
  show_extensions: { title: "파일 확장자 표시", description: "Finder에서 파일 이름 뒤의 확장자를 표시합니다." },
  show_hidden_files: { title: "숨김 파일 표시", description: "Finder에서 숨겨진 파일과 폴더를 표시합니다. 표시해도 삭제가 허용되는 것은 아닙니다." },
  show_path_bar: { title: "경로 막대 표시", description: "Finder 창 아래에 현재 폴더의 경로를 표시합니다." },
  show_status_bar: { title: "상태 막대 표시", description: "Finder 창 아래에 항목 수와 저장공간 정보를 표시합니다." },
  folders_first: { title: "이름순 정렬 시 폴더 우선", description: "Finder에서 이름순으로 정렬할 때 폴더를 파일보다 먼저 표시합니다." },
};

const settingGroups: { id: string; title: MessageKey; ids: string[] }[] = [
  { id: "files", title: "파일 표시", ids: ["show_extensions", "show_hidden_files", "folders_first"] },
  { id: "information", title: "경로와 정보", ids: ["show_path_bar", "show_status_bar"] },
];
const settingIcons: Record<string, LucideIcon> = {
  show_extensions: FileType, show_hidden_files: Eye, folders_first: ListFilter,
  show_path_bar: Route, show_status_bar: PanelBottom,
};

export function MacManagementView() {
  const { t } = useLanguage();
  const tr = (key: MessageKey) => t(key);
  const [snapshot, setSnapshot] = useState<MacSettingsSnapshot | null>(null);
  const [drafts, setDrafts] = useState<Record<string, boolean>>({});
  const [busy, setBusy] = useState<string | null>("refresh");
  const [error, setError] = useState<string | null>(null);
  const [stale, setStale] = useState(false);
  const [notice, setNotice] = useState<MessageKey | null>(null);
  const [checkedAt, setCheckedAt] = useState<number | null>(null);
  const generation = useRef(0);
  const actionLock = useRef(false);
  const mounted = useRef(false);
  const historyHeading = useRef<HTMLHeadingElement>(null);

  useEffect(() => {
    mounted.current = true;
    void refresh();
    return () => { mounted.current = false; generation.current += 1; };
  }, []);

  async function refresh() {
    if (actionLock.current) return;
    const request = ++generation.current;
    setBusy("refresh");
    setError(null);
    setNotice(null);
    try {
      const result = await getMacSettings();
      if (!mounted.current || request !== generation.current) return;
      accept(result);
      setNotice("설정 상태를 새로 확인했습니다.");
    } catch (reason) {
      if (!mounted.current || request !== generation.current) return;
      setError(errorDetail(reason));
      setStale(true);
    } finally {
      if (mounted.current && request === generation.current) setBusy(null);
    }
  }

  function accept(result: MacSettingsSnapshot) {
    setSnapshot(result);
    setDrafts({});
    setStale(false);
    setCheckedAt(Date.now());
  }

  async function mutate(id: string, action: () => Promise<MacSettingsSnapshot>, success: MessageKey) {
    // Ref lock closes the same-render double-click window before React disables controls.
    if (actionLock.current || busy || stale || !snapshot?.supported) return;
    actionLock.current = true;
    const request = ++generation.current;
    setBusy(id);
    setError(null);
    setNotice(null);
    try {
      const result = await action();
      if (!mounted.current || request !== generation.current) return;
      accept(result);
      setNotice(success);
    } catch (reason) {
      if (!mounted.current || request !== generation.current) return;
      setError(errorDetail(reason));
      setStale(true);
    } finally {
      actionLock.current = false;
      if (mounted.current && request === generation.current) setBusy(null);
    }
  }

  function apply(setting: MacSetting) {
    const enabled = drafts[setting.id];
    if (typeof enabled !== "boolean" || !hasMacSettingChange(setting, enabled)) return;
    void mutate(setting.id, () => applyMacSetting(setting.id, enabled, setting.value), "설정값을 저장하고 다시 확인했습니다. Finder 화면에는 다시 열 때 반영될 수 있습니다.");
  }

  function title(id: string) {
    const copy = settingCopy[id];
    return copy ? tr(copy.title) : snapshot?.settings.find((setting) => setting.id === id)?.title ?? id;
  }

  function valueLabel(value: boolean | null) { return tr(value === null ? "사용자 설정 없음" : value ? "켜짐" : "꺼짐"); }

  function stateLabel(setting: MacSetting) {
    if (setting.managed || setting.state === "managed") return tr("조직에서 관리");
    switch (setting.state) {
      case "value": return valueLabel(setting.value);
      case "unset": return tr("사용자 설정 없음");
      case "typeMismatch": return tr("설정 형식 불일치");
      case "error": return tr("조회 실패");
      case "unsupported": return tr("지원하지 않음");
    }
  }

  function historyStatus(status: string) {
    const known: Record<string, MessageKey> = { applied: "적용 확인", restored: "복원 확인", pending: "처리 확인 필요", prepared: "처리 확인 필요", restorePrepared: "처리 확인 필요", failed: "처리 실패", conflict: "현재 값이 달라 복원 불가", restoreFailed: "복원 실패", superseded: "이후 변경으로 대체됨" };
    return known[status] ? tr(known[status]) : status;
  }

  const locked = !!busy || stale || !snapshot?.supported;

  function stage(setting: MacSetting, enabled: boolean) {
    if (locked || actionLock.current || !canStageMacSetting(setting)) return;
    setDrafts((previous) => {
      const next = { ...previous };
      if (previous[setting.id] === enabled) delete next[setting.id];
      else next[setting.id] = enabled;
      return next;
    });
  }

  function showHistory() {
    const heading = historyHeading.current;
    if (!heading) return;
    const reducedMotion = window.matchMedia?.("(prefers-reduced-motion: reduce)")?.matches !== false;
    heading.scrollIntoView({ block: "start", behavior: reducedMotion ? "auto" : "smooth" });
    heading.focus({ preventScroll: true });
  }

  function groupSettings(ids: string[]) {
    return ids.map((id) => snapshot?.settings.find((setting) => setting.id === id))
      .filter((setting): setting is MacSetting => !!setting);
  }

  function renderSetting(setting: MacSetting) {
    const Icon = settingIcons[setting.id] ?? FolderCog;
    const value = drafts[setting.id] ?? (setting.state === "value" ? setting.value : null);
    const pending = hasMacSettingChange(setting, drafts[setting.id]);
    const issue = setting.managed || ["managed", "error", "typeMismatch", "unsupported"].includes(setting.state);
    return <li className="mac-management-setting" key={setting.id}>
      <Icon className="mac-management-setting-icon" size={21} aria-hidden="true" />
      <div className="mac-management-setting-copy">
        <h3 id={`mac-setting-${setting.id}`}>{title(setting.id)}</h3>
        <span id={`mac-value-${setting.id}`} className={`mac-management-value${issue ? " has-issue" : ""}`}>{tr("현재 값")} · {stateLabel(setting)}</span>
        {pending ? <span id={`mac-pending-${setting.id}`} className="mac-management-pending">{tr("아직 적용 안 됨")}</span> : null}
      </div>
      <div className="mac-management-setting-controls">
        <div className="mac-management-choice-field">
          <span className="mac-management-choice-label">{tr("변경할 값")}{typeof value !== "boolean" ? <small id={`mac-choice-hint-${setting.id}`}>{tr("미선택")}</small> : null}</span>
          <div className="mac-management-value-choice" role="group" aria-label={`${title(setting.id)} · ${tr("변경할 값")}`} aria-describedby={`mac-value-${setting.id}${pending ? ` mac-pending-${setting.id}` : ""}`}>
            {([false, true] as const).map((enabled) => <button type="button" key={String(enabled)} disabled={locked || !canStageMacSetting(setting)} aria-pressed={value === enabled} aria-label={`${title(setting.id)} · ${tr(enabled ? "켜기" : "끄기")}`} onClick={() => stage(setting, enabled)}>{tr(enabled ? "켜기" : "끄기")}</button>)}
          </div>
        </div>
        <button type="button" className={pending ? "primary-button" : "secondary-button"} disabled={locked || !pending} onClick={() => apply(setting)} aria-label={`${title(setting.id)} · ${tr("적용")}`}>
          {busy === setting.id ? tr("적용 중…") : tr("적용")}
        </button>
      </div>
    </li>;
  }

  return <div className="mac-management-view">
    <div className="mac-management-toolbar">
      <div className="mac-management-context"><FolderCog size={19} aria-hidden="true" /><div><strong>{tr("Finder 설정")}</strong>{checkedAt ? <span>{tr("마지막 확인")} · <time dateTime={new Date(checkedAt).toISOString()}>{formatDate(checkedAt)}</time>{stale ? ` · ${tr("새로 확인 필요")}` : ""}</span> : null}</div></div>
      <div className="mac-management-toolbar-actions">
        <button type="button" className="secondary-button mac-management-history-trigger" aria-label={tr("설정 변경 기록")} onClick={showHistory} disabled={!snapshot?.supported}><History size={17} aria-hidden="true" />{tr("설정 변경 기록")}<span aria-hidden="true">{snapshot?.history.length ?? 0}</span></button>
        <button type="button" className="secondary-button" onClick={() => void refresh()} disabled={!!busy}><RefreshCw size={17} aria-hidden="true" />{busy === "refresh" ? tr("상태 확인 중…") : tr("상태 새로 확인")}</button>
      </div>
    </div>

    <div className="mac-management-status" role="status" aria-live="polite">
      {busy ? <><RefreshCw size={16} aria-hidden="true" /><span>{busy === "refresh" ? tr("설정 상태를 읽고 있습니다. Mac 설정은 변경하지 않습니다.") : tr("설정 처리 중… 다른 변경은 완료 후 진행할 수 있습니다.")}</span></> : notice ? <><CheckCircle2 size={17} aria-hidden="true" /><span>{tr(notice)}</span></> : null}
    </div>

    {error ? <div className="mac-management-error" role="alert"><AlertTriangle size={20} aria-hidden="true" /><div><strong>{tr("설정을 확인하거나 처리하지 못했습니다.")}</strong><p>{error}</p><p>{tr("현재 값이 달라졌을 수 있습니다. 상태를 새로 확인한 뒤 다시 선택하세요.")}</p></div></div> : null}

    {snapshot && !snapshot.supported ? <section className="mac-management-unsupported"><Info size={22} aria-hidden="true" /><div><h2>{tr("이 운영체제에서는 Finder 설정을 지원하지 않습니다.")}</h2><p>{tr("시스템 성능·앱 관리·공간 정리는 해당 운영체제 기능으로 이용할 수 있습니다.")}</p></div></section> : null}

    {snapshot?.supported ? <div className={`mac-management-groups${stale ? " is-stale" : ""}`} aria-busy={!!busy}>
      {settingGroups.map((group) => <section className="mac-management-settings" aria-labelledby={`mac-settings-${group.id}`} key={group.id}>
        <h2 id={`mac-settings-${group.id}`} className="mac-management-group-title">{tr(group.title)}</h2>
        <ul className="mac-management-setting-list">{groupSettings(group.ids).map(renderSetting)}</ul>
      </section>)}
      <p className="mac-management-null-note">{tr("저장값 없음은 꺼짐이 아닙니다.")}</p>
    </div> : null}

    {snapshot?.warnings.length ? <section className="mac-management-warnings" aria-labelledby="mac-warnings-title"><h2 id="mac-warnings-title"><AlertTriangle size={17} aria-hidden="true" />{tr("확인할 사항")} ({snapshot.warnings.length})</h2><ul>{snapshot.warnings.map((warning, index) => <li key={index}>{warning}</li>)}</ul></section> : null}

    {snapshot?.supported ? <section className="mac-management-history" aria-labelledby="mac-history-title">
      <div className="mac-management-section-heading"><h2 id="mac-history-title" ref={historyHeading} tabIndex={-1}><History size={18} aria-hidden="true" />{tr("설정 변경 기록")}</h2><span>{tr("이 앱에서 변경한 항목만 표시합니다.")}</span></div>
      <p>{tr("현재 값이 앱의 적용값과 다르면 복원을 거부합니다.")}</p>
      {snapshot.history.length === 0 ? <p className="mac-management-history-empty">{tr("아직 이 앱에서 변경한 설정이 없습니다.")}</p> : <ol className="mac-management-history-list">{snapshot.history.map((change) => <li key={change.id}>
        <div className="mac-management-history-copy"><strong>{title(change.settingId)}</strong><p>{valueLabel(change.previousValue)} <span aria-hidden="true">→</span> {valueLabel(change.appliedValue)}</p><p><time dateTime={new Date(change.createdAtUnixMs).toISOString()}>{formatDate(change.createdAtUnixMs)}</time> · {historyStatus(change.status)}</p></div>
        {change.canRestore ? <div className="mac-management-restore"><button type="button" className="secondary-button" disabled={locked} onClick={() => void mutate(`restore:${change.id}`, () => restoreMacSetting(change.id), "원래 사용자 설정으로 복원하고 다시 확인했습니다.")} aria-label={`${title(change.settingId)} · ${tr("복원")}`}><RotateCcw size={16} aria-hidden="true" />{busy === `restore:${change.id}` ? tr("복원 중…") : tr("원래 설정으로 복원")}</button></div> : <span className="mac-management-history-readonly">{tr("기록만 표시")}</span>}
      </li>)}</ol>}
    </section> : null}

    <details className="mac-management-explanations"><summary><Info size={17} aria-hidden="true" />{tr("설정 설명")}<ChevronDown size={17} aria-hidden="true" /></summary><div>
      <p>{tr("저장된 사용자 설정을 확인합니다. 설정 없음은 꺼짐이 아니며, macOS 기본 동작과 화면 반영은 다를 수 있습니다.")}</p>
      <p>{tr("Finder는 자동으로 재시작하지 않습니다. 저장 확인은 화면이 이미 바뀌었다는 뜻은 아닙니다.")}</p>
      <dl>{settingGroups.flatMap((group) => group.ids).map((id) => <div key={id}><dt>{title(id)}</dt><dd>{tr(settingCopy[id].description)}</dd></div>)}</dl>
      <p>{tr("원래 설정이 없었다면 저장값을 제거해 사용자 설정 없음으로 복원합니다.")}</p>
      <p>{tr("Apple Intelligence·백그라운드 서비스·보안 설정은 변경하지 않습니다. AI 채팅에서는 상태만 조회합니다.")}</p>
    </div></details>
  </div>;
}

function errorDetail(reason: unknown): string {
  if (reason instanceof Error) return reason.message.slice(0, 600);
  if (typeof reason === "string") return reason.slice(0, 600);
  return "Mac settings request failed";
}
