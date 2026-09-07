import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { StrictMode, useState } from "react";
import { createRoot } from "react-dom/client";
import "./App.css";
import { AppShell } from "./components/AppShell";
import { LanguageProvider } from "./i18n";
import { LANGUAGE_STORAGE_KEY } from "./i18n/preference";
import type { ApplicationDataCandidate, ApplicationInventory, ApplicationInventoryEntry, ApplicationTrashPlan } from "./lib/applicationTypes";
import type { TrashOperationResult } from "./types";
import { ApplicationsView } from "./views/ApplicationsView";

const params = new URLSearchParams(window.location.search);
window.localStorage.setItem(LANGUAGE_STORAGE_KEY, params.get("language") ?? "ko");
mockWindows("main");
const platform = params.get("platform") === "windows" ? "windows" : params.get("platform") === "unsupported" ? "unsupported" : "macos";
let notify = (_value: string) => {};
let counter = 0;
let dispatches = 0;
let settingsOpened = 0;
let preparing = false;
let prepareRequests = 0;
let plan: ApplicationTrashPlan | null = null;
let inventoryId = "fixture-inventory-0";
let currentKind: "bundle" | "data" = "bundle";
const removed = new Set<string>();
let currentAppId = "";

const candidates: ApplicationDataCandidate[] = [
  { id: "cache-exact", path: "/Fixture/Library/Caches/com.example.broom-fixture", kind: "cache", evidence: "Fixture: com.example.broom-fixture 앱 식별자와 정확히 일치하는 전용 경로입니다.", estimatedBytes: 4_194_304 },
  { id: "preferences-exact", path: "/Fixture/Library/Preferences/com.example.broom-fixture.plist", kind: "preferences", evidence: "Fixture: 정확한 앱 식별자의 환경설정 파일입니다. 설정을 잃을 수 있습니다.", estimatedBytes: 2_048 },
];
const applications: ApplicationInventoryEntry[] = Array.from({ length: 57 }, (_, index) => ({
  id: `app-${index}`,
  displayName: index === 0 ? "Fixture Notes" : index === 1 ? "Fixture System App" : index === 2 ? "Fixture Running App" : index === 3 ? "Fixture 앱 — 긴 이름과 경로를 확인하는 도구" : `Fixture App ${String(index + 1).padStart(2, "0")}`,
  displayVersion: index % 3 === 0 ? "1.2.3" : null,
  publisher: "Fixture Publisher",
  installLocation: platform === "windows" ? `C:\\Program Files\\Fixture App ${index + 1}` : `/Applications/Fixture App ${index + 1}.app`,
  estimatedBytes: platform === "windows" ? 33_554_432 : null,
  removalMode: platform === "windows" ? "systemSettings" : index === 1 || index === 2 ? "protected" : "trashBundle",
  protectionReason: index === 1 ? "Fixture: 시스템 앱은 보호됩니다." : index === 2 ? "Fixture: 실행 중인 앱을 먼저 종료하세요." : null,
}));

function log(state: string) {
  notify(`${state} · mock Trash dispatches: ${dispatches} · mock settings opens: ${settingsOpened} · prepares: ${prepareRequests} · actual OS calls: 0`);
}

mockIPC(async (command, payload) => {
  const args = payload as Record<string, unknown> | undefined;
  if (command === "get_application_inventory") {
    plan = null;
    inventoryId = `fixture-inventory-${++counter}`;
    log("inventory refreshed");
    if (params.has("inventory-error")) throw new Error("Fixture inventory error");
    const result: ApplicationInventory = { platform, inventoryId, applications: platform === "unsupported" ? [] : applications.filter((app) => !removed.has(app.id)), issues: params.has("issues") ? ["Fixture: 일부 앱의 설치 정보를 읽을 수 없습니다."] : [] };
    return result;
  }
  if (command === "prepare_application_trash" || command === "prepare_application_data_trash") {
    prepareRequests += 1;
    if (preparing) throw new Error("Fixture exclusive review already in progress");
    preparing = true;
    try {
    await new Promise((resolve) => window.setTimeout(resolve, 100));
    const request = args?.request as { inventoryId: string; applicationId: string; candidateIds?: string[] };
    if (platform !== "macos" || request.inventoryId !== inventoryId) throw new Error("Fixture invalid inventory");
    const app = applications.find((entry) => entry.id === request.applicationId);
    if (!app || app.removalMode !== "trashBundle") throw new Error("Fixture protected application");
    currentKind = command === "prepare_application_trash" ? "bundle" : "data";
    currentAppId = app.id;
    if (currentKind === "bundle" && removed.has(app.id)) throw new Error("Fixture already moved");
    if (params.has("prepare-error")) throw new Error("Fixture 대상이 변경됐습니다. 목록을 새로 고치세요.");
    plan = {
      planId: `fixture-plan-${++counter}`,
      displayName: app.displayName,
      path: app.installLocation!,
      expiresAtUnixMs: Date.now() + (params.has("expired") ? -1 : 120_000),
      relatedData: params.has("no-data") ? [] : currentKind === "data" ? candidates.filter((candidate) => request.candidateIds?.includes(candidate.id)) : candidates,
      warnings: ["Fixture: 관련 데이터는 별도 선택과 확인 전에는 이동하지 않습니다."],
    };
    log(`${currentKind} plan prepared`);
    return plan;
    } finally { preparing = false; }
  }
  if (command === "dismiss_application_plan") {
    if (args?.planId === plan?.planId) plan = null;
    log("plan dismissed");
    return;
  }
  if (command === "confirm_application_trash" || command === "confirm_application_data_trash") {
    const request = args?.request as { planId: string; bundleOnlyAcknowledged?: boolean; noUninstallerAcknowledged?: boolean; relatedDataAcknowledged?: boolean };
    if (!plan || request.planId !== plan.planId || plan.expiresAtUnixMs <= Date.now()) throw new Error("Fixture invalid or expired plan");
    const bundle = command === "confirm_application_trash";
    if (bundle ? currentKind !== "bundle" || !request.bundleOnlyAcknowledged || !request.noUninstallerAcknowledged : currentKind !== "data" || !request.relatedDataAcknowledged) throw new Error("Fixture acknowledgment required");
    const consumed = plan;
    plan = null;
    dispatches += 1;
    log(`${currentKind} one-shot confirmation`);
    await new Promise((resolve) => window.setTimeout(resolve, params.has("slow") ? 3000 : 200));
    if (params.has("failure")) throw new Error("Fixture: transport interrupted after dispatch");
    const items = bundle ? [{ path: consumed.path, logicalBytes: 0, status: "moved" as const, message: null }] : consumed.relatedData.map((candidate) => ({ path: candidate.path, logicalBytes: candidate.estimatedBytes ?? 0, status: "moved" as const, message: null }));
    if (bundle) removed.add(currentAppId);
    const result: TrashOperationResult = { operationId: `fixture-operation-${counter}`, requestedCount: items.length, movedCount: items.length, movedBytes: items.reduce((sum, item) => sum + item.logicalBytes, 0), cancelled: false, stoppedEarly: false, journalComplete: true, journalPath: "/Fixture/mock-journal-only.json", items };
    return result;
  }
  if (command === "open_application_uninstall_settings") {
    if (platform !== "windows") throw new Error("Fixture unsupported platform");
    settingsOpened += 1;
    log("Windows settings requested; no removal claimed");
    return;
  }
  if (command === "reveal_local_path") { log("mock reveal only"); return; }
  if (command !== "set_application_language") throw new Error(`Unexpected fixture command: ${command}`);
});

function Fixture() {
  const [busy, setBusy] = useState(false);
  const [activity, setActivity] = useState("Fixture only · actual OS calls: 0");
  const [mobile, setMobile] = useState(false);
  notify = setActivity;
  return <AppShell activeView="applications" root="/Fixture" report={null} volume={null} mobileNavigationOpen={mobile}
    selectionBlocked={busy} dockerEnabled={params.has("docker")} onMobileNavigationChange={setMobile} onNavigate={() => {}} onPickFolder={() => {}}>
    <section className="panel"><p role="status">{activity}</p><p>Mock IPC only · 실제 파일·앱 삭제 없음 · 실제 OS 호출 없음</p></section>
    <ApplicationsView busy={busy || params.has("blocked")} onBusyChange={setBusy} onStatus={() => {}} onMutated={() => {}} />
  </AppShell>;
}

createRoot(document.getElementById("root")!).render(<StrictMode><LanguageProvider><Fixture /></LanguageProvider></StrictMode>);
