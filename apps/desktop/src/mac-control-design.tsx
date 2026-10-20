import { createRoot } from "react-dom/client";
import { useRef, useState } from "react";
import { ChevronRight, Eye, FileType, FolderCog, History, Info, ListFilter, PanelBottom, Route, RotateCcw, type LucideIcon } from "lucide-react";
import "./mac-control-design.css";

const definitions = [
  { id: "extensions", group: "files", title: "파일 확장자 표시", icon: FileType, description: "Finder에서 파일 이름 뒤의 확장자를 표시하는 사용자 설정입니다." },
  { id: "hidden", group: "files", title: "숨김 파일 표시", icon: Eye, description: "숨겨진 파일과 폴더의 표시 설정입니다. 표시해도 삭제가 허용되는 것은 아닙니다." },
  { id: "folders", group: "files", title: "이름순 정렬 시 폴더 우선", icon: ListFilter, description: "이름순으로 정렬할 때 폴더를 파일보다 먼저 표시하는 사용자 설정입니다." },
  { id: "path", group: "information", title: "경로 막대 표시", icon: Route, description: "Finder 창 아래에 현재 폴더의 경로를 표시하는 사용자 설정입니다." },
  { id: "status", group: "information", title: "상태 막대 표시", icon: PanelBottom, description: "Finder 창 아래에 항목 수와 저장공간 정보를 표시하는 사용자 설정입니다." },
] as const satisfies ReadonlyArray<{ id: string; group: string; title: string; icon: LucideIcon; description: string }>;

type SettingId = (typeof definitions)[number]["id"];
type SettingDefinition = (typeof definitions)[number];
type StoredValue = boolean | null;
interface MockChange { id: number; settingId: SettingId; previousValue: StoredValue; appliedValue: boolean; restored: boolean }

function valueLabel(value: StoredValue) {
  return value === null ? "저장값 없음" : value ? "켜짐 · 모의 저장값" : "꺼짐 · 모의 저장값";
}

function MacControlWireframe() {
  const [values, setValues] = useState<Record<SettingId, StoredValue>>({ extensions: null, hidden: null, folders: null, path: null, status: null });
  const [drafts, setDrafts] = useState<Partial<Record<SettingId, boolean>>>({});
  const [history, setHistory] = useState<MockChange[]>([]);
  const [notice, setNotice] = useState("");
  const [inspected, setInspected] = useState<SettingId>("extensions");
  const nextChange = useRef(1);
  const historyRegion = useRef<HTMLElement>(null);
  const parameters = new URLSearchParams(window.location.search);
  const narrow = parameters.get("narrow") === "1";
  const wireframe = parameters.get("wireframe") === "1";
  const candidate = parameters.get("candidate") === "b" ? "b" : parameters.get("candidate") === "c" ? "c" : "a";
  const selectedDefinition = definitions.find((definition) => definition.id === inspected)!;
  const latestChanges = new Map<SettingId, number>();
  for (const change of history) if (!change.restored && !latestChanges.has(change.settingId)) latestChanges.set(change.settingId, change.id);

  function select(id: SettingId, enabled: boolean) {
    setDrafts((previous) => {
      const next = { ...previous };
      if (previous[id] === enabled || values[id] === enabled) delete next[id];
      else next[id] = enabled;
      return next;
    });
    setNotice("");
  }

  function apply(id: SettingId) {
    const selected = drafts[id];
    if (typeof selected !== "boolean" || selected === values[id]) return;
    const change: MockChange = { id: nextChange.current++, settingId: id, previousValue: values[id], appliedValue: selected, restored: false };
    setValues((previous) => ({ ...previous, [id]: selected }));
    setDrafts((previous) => { const next = { ...previous }; delete next[id]; return next; });
    setHistory((previous) => [change, ...previous]);
    setNotice("모의 적용을 완료하고 변경 기록에 남겼습니다. 실제 Mac 설정은 바뀌지 않았습니다.");
  }

  function restore(change: MockChange) {
    if (change.restored || latestChanges.get(change.settingId) !== change.id || values[change.settingId] !== change.appliedValue) return;
    setValues((previous) => ({ ...previous, [change.settingId]: change.previousValue }));
    setDrafts((previous) => { const next = { ...previous }; delete next[change.settingId]; return next; });
    setHistory((previous) => previous.map((item) => item.id === change.id ? { ...item, restored: true } : item));
    setNotice(change.previousValue === null ? "모의 복원 완료 · 원래의 저장값 없음 상태로 되돌렸습니다." : "원래의 모의 저장값으로 복원했습니다.");
  }

  function showHistory() {
    historyRegion.current?.scrollIntoView({ block: "start", behavior: "auto" });
    historyRegion.current?.focus({ preventScroll: true });
  }

  function controls(definition: SettingDefinition) {
    const draft = drafts[definition.id];
    const choice = typeof draft === "boolean" ? draft : values[definition.id];
    const pending = typeof draft === "boolean" && draft !== values[definition.id];
    return <div className="preview-setting-actions">
      <div className="preview-choice-field">
        {!wireframe ? <span className="preview-choice-label">변경할 값{choice === null ? <small>미선택</small> : null}</span> : null}
        <div className="preview-value-choice" role="group" aria-label={`${definition.title} · 선택할 값`}>
          <button type="button" aria-pressed={choice === false} onClick={() => select(definition.id, false)}>끄기</button>
          <button type="button" aria-pressed={choice === true} onClick={() => select(definition.id, true)}>켜기</button>
        </div>
      </div>
      <button type="button" className="preview-apply" disabled={!pending} aria-label={`${definition.title} · 적용`} onClick={() => apply(definition.id)}>적용</button>
    </div>;
  }

  function renderSetting(definition: SettingDefinition) {
    const Icon = definition.icon;
    const pending = typeof drafts[definition.id] === "boolean" && drafts[definition.id] !== values[definition.id];
    return <li className={`preview-setting-row preview-setting-row--${definition.id}`} key={definition.id}>
      <Icon className="preview-setting-icon" size={22} aria-hidden="true" />
      <div className="preview-setting-copy"><h3 id={`preview-setting-${definition.id}`}>{definition.title}</h3><p>현재 · {valueLabel(values[definition.id])}</p>{pending ? <strong className="preview-pending">아직 적용 안 됨</strong> : null}</div>
      {controls(definition)}
    </li>;
  }

  function inspector() {
    const Icon = selectedDefinition.icon;
    const pending = typeof drafts[inspected] === "boolean" && drafts[inspected] !== values[inspected];
    return <div className="preview-inspector">
      <nav className="preview-inspector-list" aria-label="Finder 설정 항목">
        {([{ id: "files", title: "파일 표시" }, { id: "information", title: "경로와 정보" }] as const).map((group) => <div key={group.id}><h2 id={`preview-group-${group.id}`}>{group.title}</h2><div>{definitions.filter((definition) => definition.group === group.id).map((definition) => {
          const FeatureIcon = definition.icon;
          const draft = drafts[definition.id];
          const hasDraft = typeof draft === "boolean" && draft !== values[definition.id];
          return <button type="button" key={definition.id} aria-current={inspected === definition.id ? "true" : undefined} aria-controls="preview-inspector-detail" onClick={() => setInspected(definition.id)}><FeatureIcon size={21} aria-hidden="true" /><span><strong>{definition.title}</strong><small>{hasDraft ? "아직 적용 안 됨" : valueLabel(values[definition.id])}</small></span><ChevronRight size={16} aria-hidden="true" /></button>;
        })}</div></div>)}
      </nav>
      <section className="preview-inspector-detail" id="preview-inspector-detail" aria-labelledby="preview-inspector-title">
        <div className="preview-inspector-feature"><Icon size={60} strokeWidth={1.5} aria-hidden="true" /><span>{selectedDefinition.group === "files" ? "파일 표시" : "경로와 정보"}</span><h2 id="preview-inspector-title">{selectedDefinition.title}</h2></div>
        <div className="preview-inspector-state"><span>현재 사용자 설정</span><strong>{valueLabel(values[inspected])}</strong>{pending ? <span className="preview-pending">아직 적용 안 됨</span> : <p>저장값 없음은 꺼짐이 아닙니다.</p>}</div>
        {controls(selectedDefinition)}
      </section>
    </div>;
  }

  return <div className={`mac-control-preview${wireframe ? " is-wireframe" : ` has-style candidate-${candidate}`}${narrow ? " is-narrow" : ""}`}>
    <p className="preview-disclosure" role="note">디자인 미리보기 · 실제 Mac 설정은 변경하지 않습니다</p>
    {!wireframe ? <div className="preview-direction-toolbar"><strong>디자인 방향 선택</strong><nav aria-label="디자인 시안 선택">{([{ id: "a", label: "A · 시스템 설정형" }, { id: "b", label: "B · 기능 우선형" }, { id: "c", label: "C · 인스펙터형" }] as const).map((direction) => <a href={`?candidate=${direction.id}${narrow ? "&narrow=1" : ""}`} key={direction.id} aria-current={candidate === direction.id ? "page" : undefined}>{direction.label}</a>)}</nav></div> : null}
    <div className="preview-workspace">
      <aside className="preview-sidebar">
        <strong className="preview-brand">BroomSweepy</strong>
        <nav className="preview-section-links" aria-label="설정 분류"><a href="#preview-group-files">파일 표시</a><a href="#preview-group-information">경로와 정보</a><button type="button" onClick={showHistory}><History size={17} aria-hidden="true" />변경 기록</button></nav>
      </aside>

      <main className="preview-main">
        <header className="preview-header">
          <div><h1>맥 설정</h1><p>macOS 26.4 · Apple Silicon <span className="preview-mock-label">모의 표시</span></p></div>
          <button type="button" className="preview-history-trigger" onClick={showHistory}><History size={18} aria-hidden="true" />변경 기록<span>{history.length}</span></button>
        </header>
        <div className="preview-location"><FolderCog size={17} aria-hidden="true" /><span>현재 위치</span><strong>Finder</strong></div>

        {!wireframe && candidate === "c" ? inspector() : ([{ id: "files", title: "파일 표시" }, { id: "information", title: "경로와 정보" }] as const).map((group) => <section className={`preview-setting-group preview-setting-group--${group.id}`} aria-labelledby={`preview-group-${group.id}`} key={group.id}>
          <h2 id={`preview-group-${group.id}`}>{group.title}</h2>
          <ul>{definitions.filter((definition) => definition.group === group.id).map(renderSetting)}</ul>
        </section>)}

        <div className="preview-feedback" role="status" aria-live="polite">{notice || "선택만으로 바뀌지 않습니다. 적용을 눌러 모의 저장할 수 있습니다."}</div>

        <section className="preview-history" aria-labelledby="preview-history-title" ref={historyRegion} tabIndex={-1}>
          <div className="preview-history-heading"><h2 id="preview-history-title">변경 기록</h2><span>이 미리보기에서 적용한 모의 기록만 표시합니다.</span></div>
          {history.length === 0 ? <p className="preview-empty-history">아직 모의 변경 기록이 없습니다.</p> : <ol>{history.map((change) => {
            const title = definitions.find((definition) => definition.id === change.settingId)!.title;
            const canRestore = !change.restored && latestChanges.get(change.settingId) === change.id && values[change.settingId] === change.appliedValue;
            return <li key={change.id}><div><strong>{title}</strong><p>{valueLabel(change.previousValue)} <span aria-hidden="true">→</span> {valueLabel(change.appliedValue)}</p><span>{change.restored ? "모의 복원 완료" : "모의 적용 완료"}</span></div>{canRestore ? <button type="button" onClick={() => restore(change)} aria-label={`${title} · 원래 값으로 복원`}><RotateCcw size={17} aria-hidden="true" />원래 값으로 복원</button> : !change.restored ? <p>더 최근 변경을 먼저 복원하세요.</p> : null}</li>;
          })}</ol>}
        </section>

        <details className="preview-explanations"><summary><Info size={17} aria-hidden="true" />설정 설명</summary><div><p>저장값 없음은 사용자 설정이 없는 상태이며, 꺼짐이 아닙니다. 운영체제의 기본값이나 실제 Finder 화면을 추정하지 않습니다.</p><dl>{definitions.map((definition) => <div key={definition.id}><dt>{definition.title}</dt><dd>{definition.description}</dd></div>)}</dl></div></details>
      </main>
    </div>
  </div>;
}

createRoot(document.getElementById("root")!).render(<MacControlWireframe />);
