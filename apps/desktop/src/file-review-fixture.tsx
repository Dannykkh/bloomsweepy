// The production panel receives synthetic metadata; no native file command is invoked.
import { useState } from "react";
import { createRoot } from "react-dom/client";
import "./App.css";
import { AppShell } from "./components/AppShell";
import { FileChangeReviewPanel } from "./components/FileChangeReviewPanel";
import { LanguageProvider } from "./i18n";
import { LANGUAGE_STORAGE_KEY } from "./i18n/preference";
import type { FileChangeReviewApi, FileChangeReviewEntry, FileChangeReviewPage, FileChangeReviewPhase, FileChangeReviewQuery } from "./lib/fileChangeReview";

const params = new URLSearchParams(window.location.search);
window.localStorage.setItem(LANGUAGE_STORAGE_KEY, params.get("language") ?? "ko");
const root = params.has("windows") ? "C:\\Fixture\\검토 폴더" : "/Fixture/검토 폴더";
const delayMs = params.has("slow") ? 2_500 : 120;
let revision = 0;
let epoch = 0;
let reads = 0;
let mutations = 0;
let current: FileChangeReviewPage;
let notify = (_value: string) => {};
const wait = (ms = delayMs) => new Promise<void>((resolve) => window.setTimeout(resolve, ms));

function syntheticEntries(count = 25): FileChangeReviewEntry[] {
  return Array.from({ length: count }, (_, index) => {
    const names = ["거래명세서.pdf.exe", "회의 메모.md", "작업 도구.app", "옮긴 보고서.pdf", "이전 초안.txt"];
    const name = index === 5 ? "한글과-English가-섞인-아주-긴-파일명-".repeat(6) + ".txt" : names[index] ?? `자료 ${index + 1}.txt`;
    const skipped = index % 6 === 1;
    return { id: `synthetic-${revision}-${index}`, path: `${root}/자료/${name}`, name,
      previousPath: index === 3 ? `${root}/초안/보고서.pdf` : null,
      kind: index === 3 ? "renamed" : index === 4 ? "removed" : index % 2 === 0 ? "created" : "modified",
      reviewState: skipped ? "skipped" : "review_needed", decisionSource: "rule",
      reasons: index === 0 ? ["executable", "double_extension"] : index === 2 ? ["executable"] : index === 4 ? ["removed"] : skipped ? ["no_rule_match"] : ["metadata_changed", "bulk_change"],
      isDirectory: index === 2,
      logicalBytes: index === 2 ? null : (index + 1) * 12_480,
      modifiedAtUnixMs: index === 4 ? null : Date.now() - (index + 1) * 60_000 };
  });
}

function recount() {
  current.status.historyCount = current.entries.length;
  current.status.reviewNeededCount = current.entries.filter((entry) => entry.reviewState === "review_needed").length;
  current.status.skippedCount = current.entries.filter((entry) => entry.reviewState === "skipped").length;
  current.status.revision = ++revision;
  notify(`합성 조회 ${reads}회 · 설정/검토 요청 ${mutations}회 · 실제 파일/OS 작업 0회`);
}

function scenario(name: string) {
  epoch += 1;
  const phase: FileChangeReviewPhase = ["baseline", "checking", "incomplete", "error", "busy"].includes(name) ? name as FileChangeReviewPhase : name === "off" ? "off" : "idle";
  const enabled = phase !== "off";
  current = { offset: 0, total: 0, entries: ["off", "baseline", "empty", "unavailable"].includes(name) ? [] : syntheticEntries(name === "paging" ? 45 : name === "long" ? 8 : 6),
    status: { config: { enabled, root: name === "off" ? null : root, intervalMinutes: 15 }, phase,
      baselineEntries: phase === "baseline" ? 0 : enabled ? 128 : 0,
      lastCheckedAtUnixMs: phase === "baseline" || !enabled ? null : Date.now() - 60_000,
      nextCheckAtUnixMs: enabled ? Date.now() + 14 * 60_000 : null,
      historyCount: 0, reviewNeededCount: 0, skippedCount: 0,
      historyTruncated: name === "truncated",
      error: phase === "incomplete" ? "합성 상태: 클라우드 전용 항목 또는 검사 상한" : phase === "error" ? "합성 오류: 대상 폴더를 읽을 수 없음" : null,
      revision: ++revision } };
  recount();
}

function publishedPage(query: FileChangeReviewQuery): FileChangeReviewPage {
  const filtered = current.entries.filter((entry) => query.filter === "all" || entry.reviewState === query.filter);
  return { status: { ...current.status, config: { ...current.status.config } }, offset: query.offset, total: filtered.length,
    entries: filtered.slice(query.offset, query.offset + 20).map((entry) => ({ ...entry, reasons: [...entry.reasons] })) };
}

const fixtureApi: FileChangeReviewApi = {
  getPage: async (query) => {
    reads += 1;
    const result = publishedPage(query);
    await wait();
    notify(`합성 조회 ${reads}회 · 설정/검토 요청 ${mutations}회 · 실제 파일/OS 작업 0회`);
    if (params.has("failure")) throw new Error("합성 상태 조회 실패");
    return result;
  },
  pickFolder: async () => { await wait(); return params.has("cancel") ? null : root; },
  setConfig: async (config) => {
    mutations += 1;
    await wait();
    const rootChanged = current.status.config.root !== config.root;
    current.status.config = { ...config };
    current.status.phase = config.enabled ? "baseline" : "off";
    current.status.baselineEntries = 0;
    current.status.lastCheckedAtUnixMs = null;
    current.status.nextCheckAtUnixMs = config.enabled ? Date.now() + config.intervalMinutes * 60_000 : null;
    current.status.error = null;
    if (rootChanged) current.entries = [];
    recount();
    const requestedEpoch = ++epoch;
    if (config.enabled) window.setTimeout(() => {
      if (epoch !== requestedEpoch) return;
      current.status.phase = "idle";
      current.status.baselineEntries = 128;
      current.status.lastCheckedAtUnixMs = Date.now();
      recount();
    }, 1_500);
  },
  checkNow: async () => {
    mutations += 1;
    current.status.phase = "checking";
    recount();
    const requestedEpoch = ++epoch;
    await wait(600);
    if (epoch !== requestedEpoch) return;
    current.status.phase = "idle";
    current.status.lastCheckedAtUnixMs = Date.now();
    current.status.nextCheckAtUnixMs = Date.now() + current.status.config.intervalMinutes * 60_000;
    if (!current.entries.length) current.entries = syntheticEntries(6);
    recount();
  },
  setDisposition: async (entryId, reviewState) => {
    mutations += 1;
    await wait();
    const entry = current.entries.find((item) => item.id === entryId);
    if (!entry) throw new Error("합성 변경 기록이 이미 바뀌었습니다.");
    entry.reviewState = reviewState;
    entry.decisionSource = "manual";
    recount();
  },
};

scenario(params.get("state") ?? "changes");

function Fixture() {
  const [selectedScenario, setSelectedScenario] = useState(params.get("state") ?? "changes");
  const [version, setVersion] = useState(0);
  const [activity, setActivity] = useState("합성 테스트 화면 · 실제 파일/OS 작업 0회");
  const [mobile, setMobile] = useState(false);
  notify = setActivity;
  return <AppShell activeView="settings" root={null} report={null} volume={null} mobileNavigationOpen={mobile}
    selectionBlocked={false} dockerEnabled={false} onMobileNavigationChange={setMobile} onNavigate={() => {}} onPickFolder={() => {}}>
    <section className="panel" style={{ marginBottom: 16, padding: 18 }}>
      <h2>변경 파일 검토 합성 QA</h2><p>제품 컴포넌트에 합성 메타데이터만 주입합니다. 실제 폴더 검사·파일 열기·전송·삭제가 없습니다.</p>
      <div style={{ display: "flex", flexWrap: "wrap", gap: 12, marginBlock: 12 }}>
        <label style={{ display: "flex", alignItems: "center", gap: 10 }}>합성 상태
          <select aria-label="합성 상태" style={{ minHeight: 44, fontSize: 14, paddingInline: 10 }} value={selectedScenario} onChange={(event) => {
            setSelectedScenario(event.currentTarget.value); scenario(event.currentTarget.value); setVersion((value) => value + 1);
          }}>
            {["off", "baseline", "changes", "paging", "long", "empty", "checking", "incomplete", "error", "busy", "truncated", "unavailable"].map((name) => <option key={name} value={name}>{name}</option>)}
          </select>
        </label>
        <button type="button" className="secondary-button" onClick={() => {
          const entry = current.entries.find((item) => item.decisionSource === "manual") ?? current.entries[0];
          if (!entry) return;
          entry.id = `synthetic-changed-again-${++revision}`;
          entry.reviewState = "review_needed";
          entry.decisionSource = "rule";
          entry.kind = "modified";
          entry.reasons = ["metadata_changed"];
          if (entry.logicalBytes !== null) entry.logicalBytes += 64;
          entry.modifiedAtUnixMs = Date.now();
          recount();
        }}>같은 파일 다시 변경</button>
      </div>
      <p data-fixture-activity>{activity}</p>
    </section>
    <div className="settings-layout"><FileChangeReviewPanel key={version} api={fixtureApi} isAvailable={selectedScenario !== "unavailable"} /></div>
  </AppShell>;
}

createRoot(document.getElementById("root")!).render(<LanguageProvider><Fixture /></LanguageProvider>);
