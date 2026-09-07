import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { useState } from "react";
import { createRoot } from "react-dom/client";
import "./App.css";
import { AppShell } from "./components/AppShell";
import { StorageSectionNav } from "./components/StorageSectionNav";
import { EmptyTrashControl } from "./components/EmptyTrashControl";
import { LanguageProvider } from "./i18n";
import { LANGUAGE_STORAGE_KEY } from "./i18n/preference";
import type { EmptyTrashPlan } from "./types";

const params = new URLSearchParams(window.location.search);
window.localStorage.setItem(LANGUAGE_STORAGE_KEY, params.get("language") ?? "ko");
mockWindows("main");
let plan: EmptyTrashPlan | null = null;
let counter = 0;
let calls = 0;
let notify = (_value: string) => {};
mockIPC(async (command, payload) => {
  const args = payload as Record<string, unknown> | undefined;
  if (command === "prepare_empty_system_trash") {
    plan = { id: `fixture-${++counter}`, expiresAtUnixMs: Date.now() + (params.has("expired") ? -1 : 120_000) };
    return plan;
  }
  if (command === "dismiss_empty_system_trash") {
    if (args?.planId === plan?.id) plan = null;
    notify(`dismissed · mock dispatches: ${calls} · actual OS calls: 0`);
    return;
  }
  if (command === "open_system_trash") {
    notify(`mock open only · mock dispatches: ${calls} · actual OS calls: 0`);
    return;
  }
  if (command === "confirm_empty_system_trash") {
    const request = args?.request as { planId: string; irreversibleAcknowledged: boolean };
    if (!plan || request.planId !== plan.id || plan.expiresAtUnixMs <= Date.now() || !request.irreversibleAcknowledged) throw "invalidPlan";
    plan = null;
    calls += 1;
    notify(`mock dispatches: ${calls} · actual OS calls: 0`);
    await new Promise((resolve) => window.setTimeout(resolve, params.has("slow") ? 3000 : 200));
    if (params.has("failure")) throw new Error("fixture transport interrupted");
    return params.get("result") ?? "requested";
  }
  if (command !== "set_application_language") throw new Error(`Unexpected fixture command: ${command}`);
});

function Fixture() {
  const [busy, setBusy] = useState(false);
  const [activity, setActivity] = useState("Fixture only · actual OS calls: 0");
  const [mobile, setMobile] = useState(false);
  notify = setActivity;
  return <AppShell activeView="cleanup" root="/Fixture/Downloads" report={null} volume={null}
    mobileNavigationOpen={mobile} selectionBlocked={busy} dockerEnabled={false}
    onMobileNavigationChange={setMobile} onNavigate={() => {}} onPickFolder={() => {}}>
    <StorageSectionNav activeView="cleanup" onNavigate={() => {}} />
    <EmptyTrashControl visible platform={params.get("platform") ?? "macos"} blocked={busy || params.has("blocked")}
      onBusyChange={setBusy} onSettled={() => {}} />
    <section className="panel"><h2>휴지통 비우기 안전성 테스트</h2><p role="status">{activity}</p>
      <p>실제 OS 호출 없음 · 삭제 없음 · 운영체제 권한 요청 없음</p>
    </section>
  </AppShell>;
}

createRoot(document.getElementById("root")!).render(<LanguageProvider><Fixture /></LanguageProvider>);
