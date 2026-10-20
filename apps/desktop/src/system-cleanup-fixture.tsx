// Isolated presentation fixture: native commands, file inspection, and deletion
// never leave the mock below. All paths and results are synthetic, not host data.
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { useEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import "./App.css";
import { AppShell } from "./components/AppShell";
import { LanguageProvider, useLanguage } from "./i18n";
import { LANGUAGE_STORAGE_KEY } from "./i18n/preference";
import { CleanupView } from "./views/CleanupView";
import type {
  CleanupCandidate, CleanupScanProgress, CleanupScanReport, CleanupTrashRequest,
  ScanUiState, TrashOperationResult, TrashProgress, VolumeInfo,
} from "./types";

const params = new URLSearchParams(window.location.search);
const scenarios = ["empty", "scanning", "results", "zero", "error", "limited", "blocked", "action-error"] as const;
type Scenario = typeof scenarios[number];
const requested = params.get("case") ?? "empty";
const scenario: Scenario = scenarios.includes(requested as Scenario) ? requested as Scenario : "empty";
const platform = params.get("platform") === "windows" ? "windows" : "macos";
const languages = ["ko", "en", "ja", "zh-CN"] as const;
const requestedLanguage = params.get("language") ?? "ko";
const language = languages.includes(requestedLanguage as typeof languages[number]) ? requestedLanguage as typeof languages[number] : "ko";
window.localStorage.setItem(LANGUAGE_STORAGE_KEY, language);
document.documentElement.dataset.windowMaterial = "opaque";
mockWindows("main");

const bannerCopy = {
  ko: "합성 QA — 실제 IPC·파일 작업·OS 변경·삭제 없음",
  en: "Synthetic QA — no real IPC, file operations, OS changes or deletion",
  ja: "合成 QA — 実際の IPC・ファイル操作・OS 変更・削除は行いません",
  "zh-CN": "合成 QA — 不执行真实 IPC、文件操作、系统更改或删除",
};
const mib = 1024 ** 2;
const day = 24 * 60 * 60 * 1000;
const timestamp = Date.UTC(2026, 9, 21);
const base = platform === "windows" ? "C:\\Synthetic\\BroomSweepy\\Users\\Preview" : "/Synthetic/BroomSweepy/Users/Preview";
const path = (suffix: string) => `${base}${platform === "windows" ? "\\" : "/"}${platform === "windows" ? suffix.split("/").join("\\") : suffix}`;
const candidates: CleanupCandidate[] = [
  { kind: "temporaryEntry", confidence: "likelySafe", name: "Synthetic export logs", path: path("Temporary/Synthetic export logs"), sourceLabel: "Synthetic user temporary root", logicalBytes: 24 * mib, entryCount: 34, inactiveDays: 90, modifiedAtUnixMs: timestamp - 90 * day, evidence: ["Synthetic: unchanged for 90 days", "Synthetic metadata only; a filename or age is not a safety guarantee."] },
  { kind: "temporaryEntry", confidence: "likelySafe", name: "Synthetic capture session.tmp", path: path("Temporary/Synthetic capture session.tmp"), sourceLabel: "Synthetic user temporary root", logicalBytes: 12 * mib, entryCount: 1, inactiveDays: 60, modifiedAtUnixMs: timestamp - 60 * day, evidence: ["Synthetic: stale temporary entry", "No real process or file was inspected."] },
  { kind: "cacheDirectory", confidence: "likelySafe", name: "Synthetic Editor cache", path: path("Caches/Synthetic Editor/Cache"), sourceLabel: "Synthetic known cache root", logicalBytes: 160 * mib, entryCount: 840, inactiveDays: 45, modifiedAtUnixMs: timestamp - 45 * day, evidence: ["Synthetic: known cache location", "Synthetic: 840 entries counted; this is not a host scan."] },
  { kind: "cacheDirectory", confidence: "likelySafe", name: "Synthetic long-path preview cache", path: path("Caches/Very Long Synthetic Application Name With Multiple Workspaces/Preview Renderer/An intentionally long folder name for wrapping without horizontal overflow/Version-2026-10/Cache"), sourceLabel: "Synthetic known cache root", logicalBytes: 72 * mib, entryCount: 214, inactiveDays: 75, modifiedAtUnixMs: timestamp - 75 * day, evidence: ["Synthetic: long path wrapping example", "Synthetic evidence with deliberately longer text: metadata is incomplete and should not be interpreted as proof that an application no longer needs this data."] },
  { kind: "appDataDirectory", confidence: "review", name: "Synthetic retired application settings", path: path("Preferences/Synthetic Retired Application"), sourceLabel: "Synthetic unmatched application data", logicalBytes: 6 * mib, entryCount: 19, inactiveDays: 300, modifiedAtUnixMs: timestamp - 300 * day, evidence: ["Synthetic: no matching app identity in fixture inventory", "Review required: preferences may contain account or application settings."] },
  ...(platform === "macos" ? [
    { kind: "developerCache" as const, confidence: "review" as const, name: "Synthetic Xcode DerivedData", path: path("Library/Developer/Xcode/DerivedData/SyntheticProject-long-build-output-name"), sourceLabel: "Synthetic Xcode DerivedData", logicalBytes: 640 * mib, entryCount: 1280, inactiveDays: 12, modifiedAtUnixMs: timestamp - 12 * day, evidence: ["Synthetic: 12 days old, above the 7-day threshold", "Review required: rebuilding may take time. Age does not prove this project is unused."] },
    { kind: "developerCache" as const, confidence: "review" as const, name: "Synthetic iOS DeviceSupport", path: path("Library/Developer/Xcode/iOS DeviceSupport/Synthetic Device Symbols"), sourceLabel: "Synthetic iOS DeviceSupport", logicalBytes: 1200 * mib, entryCount: 81, inactiveDays: 90, modifiedAtUnixMs: timestamp - 90 * day, evidence: ["Synthetic: 90 days old, above the 30-day threshold", "Review required: device support data may need to be downloaded again."] },
  ] : []),
];
const allowedPaths = new Set(candidates.map((candidate) => candidate.path));
const diagnostics = { reads: 0, inspections: 0, rejected: 0, scans: 0, confirmations: 0, cancellations: 0 };
let publish = (_value: string) => {};
let lastActivity = "Ready";
function record(message: string) {
  lastActivity = message;
  publish(message);
}
Object.defineProperty(window, "__systemCleanupFixture", {
  configurable: true,
  get: () => Object.freeze({ scenario, platform, language, ...diagnostics, actualNativeCalls: 0, actualDeletions: 0, lastActivity }),
});
mockIPC(async (command, payload) => {
  if (command === "set_application_language") { diagnostics.reads += 1; return; }
  if (command === "inspect_local_path" || command === "reveal_local_path") {
    const target = (payload as { path?: unknown } | undefined)?.path;
    if (typeof target !== "string" || !allowedPaths.has(target)) {
      diagnostics.rejected += 1;
      throw new Error("Synthetic fixture rejected a path outside its exact allowlist");
    }
    diagnostics.inspections += 1;
    record(`Mock ${command}; actual OS calls: 0`);
    return command === "inspect_local_path" ? "opened" : undefined;
  }
  diagnostics.rejected += 1;
  record(`Blocked native command: ${command}`);
  throw new Error(`Synthetic fixture blocks native command: ${command}`);
}, { shouldMockEvents: true });

let reportSerial = 0;
function reportFor(kind: Scenario): CleanupScanReport {
  const empty = kind === "zero";
  const limited = kind === "limited";
  return {
    completedAtUnixMs: timestamp + ++reportSerial,
    durationMs: 2340, scannedRoots: platform === "macos" ? 5 : 3, processedEntries: 2469,
    processedBytes: 4_500 * mib, unreadableEntries: limited ? 2 : 0,
    candidateBytes: empty ? 0 : candidates.reduce((total, candidate) => total + candidate.logicalBytes, 0),
    candidates: empty ? [] : candidates.map((candidate) => ({ ...candidate, evidence: [...candidate.evidence] })),
    limitReached: limited, issues: limited ? [{ path: path("Unavailable/Synthetic restricted root"), message: "Synthetic read limit; some locations were not checked" }] : [],
    registryResidues: { supported: platform === "windows", issues: [], candidates: platform === "windows" && !empty ? [
      { displayName: "Synthetic missing uninstall entry", registryPath: "HKEY_CURRENT_USER\\Software\\Synthetic-QA\\Uninstall\\Missing-App", registryScope: "user", evidence: ["Synthetic: installer path and installation folder are both missing", "Read-only evidence; the fixture cannot modify registry data."] },
      { displayName: "Synthetic machine-scope uninstall entry", registryPath: "HKEY_LOCAL_MACHINE\\Software\\Synthetic-QA\\Uninstall\\Missing-App-2", registryScope: "machine", evidence: ["Synthetic: two missing path references", "Review only: not a file-trash target and no registry deletion is available."] },
    ] : [] },
  };
}
const scanProgress: CleanupScanProgress = { message: "Synthetic scan only", processedRoots: 2, totalRoots: platform === "macos" ? 5 : 3, processedEntries: 1536, processedBytes: 2300 * mib, candidatesFound: 4 };
const volume: VolumeInfo = { name: "Synthetic system disk", mountPoint: platform === "windows" ? "C:\\" : "/", fileSystem: platform === "windows" ? "NTFS" : "APFS", totalBytes: 256 * 1024 ** 3, availableBytes: 24 * 1024 ** 3, removable: false, readOnly: false, isDiskImage: false, isSystem: true };

function Fixture() {
  const { t } = useLanguage();
  const [activity, setActivity] = useState(lastActivity);
  const [mobile, setMobile] = useState(false);
  const [report, setReport] = useState<CleanupScanReport | null>(() => ["results", "zero", "limited", "blocked"].includes(scenario) ? reportFor(scenario) : null);
  const [state, setState] = useState<ScanUiState>(scenario === "scanning" ? "scanning" : scenario === "error" ? "error" : report ? "success" : "idle");
  const [progress, setProgress] = useState<CleanupScanProgress | null>(scenario === "scanning" ? scanProgress : null);
  const [error, setError] = useState<string | null>(scenario === "error" ? "Synthetic scan failed. No host locations were read." : null);
  const [actionRunning, setActionRunning] = useState(false);
  const [actionProgress, setActionProgress] = useState<TrashProgress | null>(null);
  const [actionResult, setActionResult] = useState<TrashOperationResult | null>(null);
  const [actionError, setActionError] = useState<string | null>(scenario === "action-error" ? "Synthetic trash preflight failed. No files were moved." : null);
  const generation = useRef(0);
  const mounted = useRef(false);
  const timer = useRef<number | null>(null);
  publish = setActivity;
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      generation.current += 1;
      if (timer.current !== null) window.clearTimeout(timer.current);
    };
  }, []);

  function startScan() {
    if (scenario === "blocked" || state === "scanning" || actionRunning) return;
    diagnostics.scans += 1;
    const revision = ++generation.current;
    setReport(null); setActionResult(null); setActionError(null); setError(null);
    setState("scanning"); setProgress(scanProgress);
    record("Synthetic scan started; no filesystem access");
    timer.current = window.setTimeout(() => {
      if (!mounted.current || revision !== generation.current) return;
      setReport(reportFor(scenario === "zero" || scenario === "limited" ? scenario : "results"));
      setProgress(null); setState("success");
      record("Synthetic scan completed; no filesystem access");
    }, 1200);
  }
  function cancelScan() {
    generation.current += 1;
    if (timer.current !== null) window.clearTimeout(timer.current);
    setState("cancelled"); setProgress(null);
    diagnostics.cancellations += 1;
    record("Synthetic scan cancelled; actual OS calls: 0");
  }
  async function moveToTrash(request: CleanupTrashRequest): Promise<TrashOperationResult> {
    if (scenario === "blocked" || actionRunning || !report) throw new Error("Synthetic operation is unavailable");
    const selected = report.candidates.filter((candidate) => request.paths.includes(candidate.path));
    if (!request.paths.length || new Set(request.paths).size !== request.paths.length || selected.length !== request.paths.length || selected.some((candidate) => !allowedPaths.has(candidate.path))) throw new Error("Synthetic fixture rejects unknown targets");
    if (selected.some((candidate) => candidate.confidence === "review") && !request.allowReviewCandidates) throw new Error("Synthetic review acknowledgement is required");
    const revision = ++generation.current;
    diagnostics.confirmations += 1;
    setActionRunning(true); setActionProgress({ phase: "preflight", message: "Synthetic preflight only", processedItems: 0, totalItems: selected.length });
    record("Synthetic confirmation only; actual deletions: 0");
    await new Promise<void>((resolve) => window.setTimeout(resolve, 240));
    if (!mounted.current || revision !== generation.current) throw new Error("Synthetic operation cancelled; no files were moved");
    if (scenario === "action-error") {
      setActionRunning(false); setActionProgress(null); setReport(null);
      setActionError("Synthetic trash preflight failed. No files were moved.");
      throw new Error("Synthetic trash preflight failed. No files were moved.");
    }
    const result: TrashOperationResult = { operationId: `synthetic-operation-${diagnostics.confirmations}`, requestedCount: selected.length,
      movedCount: selected.length, movedBytes: selected.reduce((total, candidate) => total + candidate.logicalBytes, 0), cancelled: false,
      stoppedEarly: false, journalComplete: true, journalPath: path("Synthetic-Journal/not-written.jsonl"),
      items: selected.map((candidate) => ({ path: candidate.path, logicalBytes: candidate.logicalBytes, status: "moved", message: "Synthetic result only; no file was moved" })) };
    setActionResult(result); setReport(null); setActionProgress(null); setActionRunning(false);
    record("Synthetic result returned; no file or journal was written");
    return result;
  }
  function cancelAction() {
    generation.current += 1;
    setActionRunning(false); setActionProgress(null);
    diagnostics.cancellations += 1;
    record("Synthetic action cancelled; actual deletions: 0");
  }

  return <AppShell activeView="cleanup" root={null} report={null} volume={volume}
    mobileNavigationOpen={mobile} selectionBlocked={scenario === "blocked" || actionRunning} dockerEnabled={false}
    macosAvailable={platform === "macos"} windowsAvailable={platform === "windows"}
    onMobileNavigationChange={setMobile} onNavigate={(view) => record(`Fixture stays on system candidates; navigation requested: ${view}`)}
    onPickFolder={() => record("Fixture blocks native folder selection")}>
    <p data-testid="system-cleanup-fixture-status" role="status" style={{ margin: "0 0 12px", padding: "8px 10px", fontSize: 14, lineHeight: 1.5, color: "var(--text-muted)", border: "1px solid var(--line)", borderRadius: 8, overflowWrap: "anywhere" }}>
      <strong>{bannerCopy[language]}</strong><br />{scenario} · {platform} · {activity}
    </p>
    <div className="view-stack cleanup-workspace">
      <section className="cleanup-source-toolbar" aria-label={t("정리 후보")}>
        <div className="segmented-control cleanup-source-toolbar__modes" role="group" aria-label={t("정리 후보 종류")}>
          <button type="button" aria-pressed={false} disabled={actionRunning} onClick={() => record("Folder-tree navigation is outside this system-candidate fixture")}>{t("폴더 트리")}</button>
          <button type="button" className="is-active" aria-pressed={true} disabled={actionRunning} onClick={() => record("Synthetic system candidates selected")}>{t("시스템 후보")}</button>
        </div>
      </section>
      <CleanupView platform={platform} report={report} progress={progress} state={state} error={error}
        blocked={scenario === "blocked"} actionRunning={actionRunning} actionProgress={actionProgress} actionResult={actionResult} actionError={actionError}
        onStart={startScan} onCancel={cancelScan} onMoveToTrash={moveToTrash} onCancelAction={cancelAction} />
    </div>
  </AppShell>;
}

const fixtureRoot = createRoot(document.getElementById("root")!);
fixtureRoot.render(<LanguageProvider><Fixture /></LanguageProvider>);
if (import.meta.hot) import.meta.hot.dispose(() => { publish = () => {}; fixtureRoot.unmount(); });
