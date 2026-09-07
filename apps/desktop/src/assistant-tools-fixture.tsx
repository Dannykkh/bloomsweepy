// Synthetic UI integration fixture. No provider, filesystem, or native operation runs.
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { createRoot } from "react-dom/client";
import { AssistantView } from "./views/AssistantView";
import { LanguageProvider } from "./i18n";
import { LANGUAGE_STORAGE_KEY } from "./i18n/preference";
import { confirmAssistantEmptyPlan } from "./lib/bridge";
import { DEFAULT_SCAN_CONFIG } from "./types";
import type { AssistantEmptyWorkspace, AssistantSessionDetail, ControlStatus, TrashOperationResult } from "./types";
import "./App.css";

const params = new URLSearchParams(window.location.search);
window.localStorage.setItem(LANGUAGE_STORAGE_KEY, params.get("language") ?? "ko");
window.localStorage.setItem("bloomsweepy.assistant-provider", "codex");
mockWindows("main");
const now = Date.now();
const summary = { scopeName: "QA Workspace", completedAtUnixMs: now, totalLogicalBytes: 100,
  totalFiles: 1, totalDirectories: 3, unreadableEntries: 0, emptyDirectoryCount: 3, childrenTruncated: false, children: [] };
const session: AssistantSessionDetail = {
  session: { id: "aabbccdd", scopeKind: "folder", scopeRoot: "/Demo/QA Workspace", scopeName: "QA Workspace",
    createdAtUnixMs: now, updatedAtUnixMs: now, messageCount: 0, lastProvider: "codex", lastModel: null },
  folderSummary: summary, messages: [],
};
const seed: AssistantEmptyWorkspace = {
  revision: "fixture-revision", summary, totalFound: 3, omittedCount: 0,
  candidates: ["Empty draft", "Keep this folder", "Long folder name ".repeat(12)].map((name, index) => ({
    id: `candidate-${index + 1}`, number: index + 1, name, path: `/Demo/QA Workspace/${name}`,
  })), selectedIds: ["candidate-1", "candidate-2", "candidate-3"], plan: null,
};
let workspace: AssistantEmptyWorkspace | null = null;
let executions = 0;
const clone = <T,>(value: T): T => structuredClone(value);
const control: ControlStatus = { revision: 1, bridgeAvailable: true, connectedClients: 0,
  lastConnectedAtUnixMs: null, activeOperation: null, lastOperation: null, pendingReview: null, lastError: null,
  protocolVersion: 3, searchAccess: { files: false, documents: false },
  scanAccess: { enabled: false, root: null, approvedAtUnixMs: null }, cleanupAccess: { enabled: false, approvedAtUnixMs: null } };

mockIPC((command, raw) => {
  const args = raw as Record<string, unknown>;
  if (command === "set_application_language") return null;
  if (command === "get_mcp_registration_statuses") return [];
  if (command === "get_assistant_provider_status") return [{ provider: "codex", label: "Codex · QA mock",
    installed: true, authentication: "authenticated", available: true, busy: false, detail: "Synthetic test adapter — no AI requests",
    models: [], state: "ready", executablePath: null, version: "fixture" }];
  if (command === "list_assistant_sessions") return [session.session];
  if (command === "get_assistant_session") return clone(session);
  if (command === "get_assistant_empty_workspace") return clone(workspace);
  if (command === "append_assistant_message") {
    const request = args.request as { role: "user" | "assistant"; content: string; provider: "codex" | null; model: null };
    const message = { sequence: session.messages.length + 1, ...request,
      providerLabel: request.role === "user" ? null : request.provider ? "Codex · QA mock" : "BroomSweepy", createdAtUnixMs: Date.now() };
    session.messages.push(message);
    session.session.messageCount = session.messages.length;
    return clone({ session: session.session, message });
  }
  if (command === "ask_assistant") {
    const request = args.request as { message: string };
    let action: "scan" | "selection" | null = null;
    if (request.message.includes("검사") || request.message.includes("scan")) { workspace = clone(seed); action = "scan"; }
    else if (workspace && request.message.includes("2")) { workspace.selectedIds = workspace.selectedIds.filter(id => id !== "candidate-2"); workspace.plan = null; action = "selection"; }
    return { provider: "codex", label: "Codex · QA mock", model: null, message: "이 메시지는 테스트 응답입니다. 최종 확인 버튼을 사용하세요.",
      dockerContext: null, emptyWorkspace: action ? clone(workspace) : null, toolAction: action };
  }
  if (command === "select_assistant_empty_candidates" && workspace) {
    workspace.selectedIds = args.candidateIds as string[]; workspace.plan = null; return clone(workspace);
  }
  if (command === "prepare_assistant_empty_plan" && workspace) {
    workspace.plan = { id: `plan-${Date.now()}`, candidateIds: [...workspace.selectedIds],
      expiresAtUnixMs: params.has("expired") ? Date.now() - 1 : Date.now() + 300_000 };
    return clone(workspace);
  }
  if (command === "confirm_assistant_empty_plan" && workspace?.plan && workspace.plan.id === args.planId) {
    const items = workspace.candidates.filter(candidate => workspace!.plan!.candidateIds.includes(candidate.id));
    workspace = null; executions += 1;
    document.getElementById("fixture-executions")!.textContent = `Mock executions: ${executions}`;
    return { operationId: "fixture-only", requestedCount: items.length, movedCount: items.length, movedBytes: 0,
      cancelled: false, stoppedEarly: false, journalComplete: true, journalPath: "/Demo/mock-journal",
      items: items.map(item => ({ path: item.path, logicalBytes: 0, status: "moved", message: null })) } satisfies TrashOperationResult;
  }
  throw new Error(`Synthetic fixture rejects command: ${command}`);
}, { shouldMockEvents: true });
const noop = () => undefined;
createRoot(document.getElementById("root")!).render(<LanguageProvider>
  <main style={{ maxWidth: 1100, margin: "0 auto", padding: 20 }}>
    <p role="note">합성 QA · 실제 파일 검사/이동 및 AI 전송 없음 · “검사”, “2번 보관”을 사용</p>
    <output id="fixture-executions">Mock executions: 0</output>
    <AssistantView status={control} canEnableSearch={false} updatingSearchAccess={false} searchAccessError={null}
      onToggleSearchAccess={noop} scanRoot={session.session.scopeRoot} scanConfig={DEFAULT_SCAN_CONFIG} canEnableScan={false}
      updatingScanAccess={false} scanAccessError={null} onToggleScanAccess={noop} canEnableCleanup={false}
      cleanupAccessLocked={false} updatingCleanupAccess={false} cleanupAccessError={null} onToggleCleanupAccess={noop}
      onReviewPending={noop} directoryProgress={null} directoryState="success" volumes={[]} dockerStatus={null}
      launchRequest={null} onLaunchRequestHandled={noop} onPickFolder={async () => null} onConfirmEmptyPlan={confirmAssistantEmptyPlan} />
  </main>
</LanguageProvider>);
