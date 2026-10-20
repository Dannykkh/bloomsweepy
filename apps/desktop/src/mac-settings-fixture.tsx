import { mockIPC } from "@tauri-apps/api/mocks";
import { StrictMode, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import "./App.css";
import { LanguageProvider } from "./i18n";
import { LANGUAGE_STORAGE_KEY } from "./i18n/preference";
import type { MacSetting, MacSettingsSnapshot } from "./lib/macSettings";
import { MacManagementView } from "./views/MacManagementView";

// Dedicated fixture entry point only: every native command is intercepted.
const params = new URLSearchParams(window.location.search);
const scenario = params.get("case") ?? "success";
document.body.classList.add("mac-management-fixture-host");
window.localStorage.setItem(LANGUAGE_STORAGE_KEY, params.get("language") ?? "ko");
let notify = (_message: string) => {};
let reads = 0;
let applies = 0;
let restores = 0;
let exclusive = false;
let serial = 0;
let renderCount = 0;
let readSettlements = 0;
let fixtureView = "macos_management";
let lastActivity = "fixture module initialized";
let lastActivityAt = performance.now();
const moduleLoadedAt = lastActivityAt;
// A diagnostic read must not depend on a pending React state commit or trigger one.
Object.defineProperty(window, "__macSettingsFixture", {
  configurable: true,
  get: () => Object.freeze({ scenario, view: fixtureView, reads, readSettlements, applies, restores, exclusive, renderCount, lastActivity, lastActivityAt, moduleLoadedAt, now: performance.now() }),
});
const rows = [
  ["show_extensions", "파일 확장자 표시", ".GlobalPreferences", "AppleShowAllExtensions"],
  ["show_hidden_files", "숨김 파일 표시", "com.apple.finder", "AppleShowAllFiles"],
  ["show_path_bar", "경로 막대 표시", "com.apple.finder", "ShowPathbar"],
  ["show_status_bar", "상태 막대 표시", "com.apple.finder", "ShowStatusBar"],
  ["folders_first", "이름순 정렬 시 폴더 우선", "com.apple.finder", "_FXSortFoldersFirst"],
];
const settings: MacSetting[] = rows.map(([id, title, domain, key], index) => ({ id, title, description: "Synthetic preference only", domain, key, value: index < 2 ? null : index === 2, state: index < 2 ? "unset" : "value", managed: false, canModify: true }));
if (scenario === "mixed") {
  settings[1] = { ...settings[1], state: "managed", managed: true, canModify: false, value: true };
  settings[3] = { ...settings[3], state: "typeMismatch", value: null, canModify: false };
  settings[4] = { ...settings[4], state: "error", value: null, canModify: false };
}
if (scenario === "unset") for (const setting of settings) { setting.value = null; setting.state = "unset"; }
const snapshot: MacSettingsSnapshot = {
  supported: scenario !== "unsupported",
  platform: scenario === "unsupported" ? "windows" : "macos",
  settings: scenario === "unsupported" ? [] : settings,
  warnings: scenario === "mixed" ? ["Fixture: one managed preference and two unreadable preferences. No real OS calls."] : [],
  history: scenario === "history" || scenario === "restore-error" ? [{ id: "fixture-change-initial", settingId: "show_path_bar", previousValue: null, appliedValue: true, createdAtUnixMs: 1_792_440_000_000, status: "applied", canRestore: true }] : [],
};

function log(message = "Ready") {
  lastActivity = message;
  lastActivityAt = performance.now();
  notify(`Fixture only · reads: ${reads} · applies: ${applies} · restores: ${restores} · actual OS calls: 0 · ${message}`);
}
const delay = (writing = false) => new Promise((resolve) => window.setTimeout(resolve, scenario === "slow" && writing ? 1800 : 200));
const copy = () => structuredClone(snapshot);

mockIPC(async (command, payload) => {
  if (command === "set_application_language") return;
  if (command === "get_mac_settings") {
    reads += 1;
    log("Reading synthetic preferences");
    await delay();
    readSettlements += 1;
    if (scenario === "load-error") throw new Error("Fixture: preference read failed");
    log("Read complete");
    return copy();
  }
  if (command !== "apply_mac_setting" && command !== "restore_mac_setting") throw new Error(`Unexpected fixture command: ${command}`);
  if (exclusive) throw new Error("Fixture: a write is already in progress");
  if (!snapshot.supported) throw new Error("Fixture: unsupported platform");
  exclusive = true;
  const request = (payload as { request: Record<string, unknown> }).request;
  try {
    if (command === "apply_mac_setting") {
      applies += 1;
      log("Synthetic apply dispatched");
      await delay(true);
      if (scenario === "write-error") throw new Error("Fixture: write verification failed; refresh required");
      const setting = settings.find((entry) => entry.id === request.settingId);
      if (!setting || setting.managed || !setting.canModify || setting.value !== request.expectedValue) throw new Error("Fixture: preference changed; refresh required");
      if (typeof request.enabled !== "boolean") throw new Error("Fixture: Boolean required");
      if (scenario === "external-change") { setting.value = setting.value === null ? true : !setting.value; setting.state = "value"; throw new Error("Fixture: external change detected; refresh required"); }
      snapshot.history.unshift({ id: `fixture-change-${++serial}`, settingId: setting.id, previousValue: setting.value, appliedValue: request.enabled, createdAtUnixMs: Date.now(), status: "applied", canRestore: true });
      setting.value = request.enabled;
      setting.state = "value";
    } else {
      restores += 1;
      log("Synthetic restore dispatched");
      await delay(true);
      if (scenario === "restore-error") throw new Error("Fixture: restore failed; refresh required");
      const change = snapshot.history.find((entry) => entry.id === request.changeId);
      const setting = settings.find((entry) => entry.id === change?.settingId);
      if (!change || !change.canRestore || !setting || setting.value !== change.appliedValue) throw new Error("Fixture: preference changed after apply");
      setting.value = change.previousValue;
      setting.state = change.previousValue === null ? "unset" : "value";
      change.canRestore = false;
      change.status = "restored";
    }
    log("Completed; no OS changes");
    return copy();
  } finally { exclusive = false; }
});

function Fixture() {
  renderCount += 1;
  const [activity, setActivity] = useState("Fixture only · actual OS calls: 0");
  const [view, setView] = useState("macos_management");
  const [fontDiagnostic, setFontDiagnostic] = useState("Fixture font load pending · Pretendard Variable · 450 / 650");
  useEffect(() => {
    let active = true;
    void Promise.all([450, 650].map(async (weight) => {
      const faces = await document.fonts.load(`${weight} 14px "Pretendard Variable"`, "가나다 BroomSweepy");
      return { weight, count: faces.length, statuses: faces.map((face) => face.status) };
    })).then((weights) => {
      if (!active) return;
      const element = document.querySelector(".mac-management-view") ?? document.documentElement;
      setFontDiagnostic(JSON.stringify({
        fixtureOnly: true,
        state: weights.every((weight) => weight.count > 0 && weight.statuses.every((status) => status === "loaded")) ? "loaded" : "unavailable",
        sample: "가나다 BroomSweepy",
        family: getComputedStyle(element).fontFamily,
        weights,
      }));
    }).catch((reason: unknown) => {
      if (active) setFontDiagnostic(JSON.stringify({ fixtureOnly: true, state: "error", error: reason instanceof Error ? reason.message : String(reason) }));
    });
    return () => { active = false; };
  }, []);
  fixtureView = view;
  notify = setActivity;
  return <main className="mac-management-fixture">
    <header className="utility-header utility-header--compact utility-header--mac-settings"><h1>{view === "macos_management" ? "맥 설정" : "시험 화면"}</h1></header>
    {view === "macos_management" ? <MacManagementView /> : <section data-testid="fixture-route"><h2>Fixture target: {view}</h2><p>Mac management is unmounted. Pending results must not replace this view.</p><button type="button" className="secondary-button" onClick={() => { log("Returning to synthetic Mac settings"); setView("macos_management"); }}>맥 설정 시험으로 돌아가기</button></section>}
    <div aria-label="Fixture-only test controls">
      <button type="button" className="secondary-button" data-testid="fixture-leave" disabled={view !== "macos_management"} onClick={() => { log("Fixture-only unmount requested"); setView("fixture_outside"); }}>시험 화면 나가기</button>
    </div>
    <p className="mac-management-fixture-log" data-testid="fixture-log" role="status">{activity}</p>
    <output className="mac-management-fixture-log" data-testid="fixture-font" aria-label="Fixture-only font loading" style={{ display: "block" }}>{fontDiagnostic}</output>
  </main>;
}
createRoot(document.getElementById("root")!).render(<StrictMode><LanguageProvider><Fixture /></LanguageProvider></StrictMode>);
