import { AlertTriangle, AppWindow, CheckCircle2, ChevronLeft, ChevronRight, ExternalLink, FolderOpen, LoaderCircle, RefreshCw, Search, ShieldCheck, Trash2 } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { useLanguage } from "../i18n";
import { confirmApplicationDataTrash, confirmApplicationTrash, dismissApplicationPlan, getApplicationInventory, openApplicationUninstallSettings, prepareApplicationDataTrash, prepareApplicationTrash } from "../lib/applicationBridge";
import type { ApplicationDataCandidate, ApplicationInventory, ApplicationInventoryEntry, ApplicationTrashPlan } from "../lib/applicationTypes";
import { revealPath } from "../lib/bridge";
import { formatBytes, formatCount } from "../lib/format";
import type { TrashOperationResult } from "../types";
import "./ApplicationsView.css";

interface ApplicationsViewProps {
  active?: boolean;
  busy: boolean;
  onStatus: (message: string) => void;
  onMutated: () => void;
  onBusyChange?: (busy: boolean) => void;
}

interface ReviewTarget {
  application: ApplicationInventoryEntry;
  inventoryId: string;
  kind: "bundle" | "data";
  candidateIds: string[];
}

interface RelatedReview {
  application: ApplicationInventoryEntry;
  inventoryId: string;
  candidates: ApplicationDataCandidate[];
}

const PAGE_SIZE = 50;
const detail = (reason: unknown) => reason instanceof Error ? reason.message : String(reason);

export function ApplicationsView({ active = true, busy, onStatus, onMutated, onBusyChange }: ApplicationsViewProps) {
  const { t } = useLanguage();
  const [inventory, setInventory] = useState<ApplicationInventory | null>(null);
  const [loading, setLoading] = useState(false);
  const [query, setQuery] = useState("");
  const [page, setPage] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const [review, setReview] = useState<ReviewTarget | null>(null);
  const [related, setRelated] = useState<RelatedReview | null>(null);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [movedApps, setMovedApps] = useState<Set<string>>(new Set());
  const [result, setResult] = useState<TrashOperationResult | null>(null);
  const [uncertain, setUncertain] = useState(false);
  const [opening, setOpening] = useState(false);
  const started = useRef(false);
  const loadingRef = useRef(false);
  const openingRef = useRef(false);
  const reviewRef = useRef(false);
  const triggerRef = useRef<HTMLButtonElement | null>(null);
  const refreshRef = useRef<HTMLButtonElement>(null);
  const relatedRef = useRef<HTMLElement>(null);
  const mounted = useRef(true);

  useEffect(() => { mounted.current = true; return () => { mounted.current = false; }; }, []);
  useEffect(() => {
    if (!active) {
      reviewRef.current = false;
      setReview(null);
    }
  }, [active]);
  useEffect(() => {
    if (active && !busy && !started.current) {
      started.current = true;
      void refresh();
    }
  }, [active, busy]);

  async function refresh() {
    if (busy || loadingRef.current || reviewRef.current || openingRef.current) return;
    loadingRef.current = true;
    setLoading(true);
    setError(null);
    setStatus(null);
    // Refresh deliberately discards old candidate IDs and previous selection.
    setRelated(null);
    setSelected(new Set());
    setInventory(null);
    setResult(null);
    try {
      const next = await getApplicationInventory();
      if (!mounted.current) return;
      setInventory(next);
      setMovedApps(new Set());
      setUncertain(false);
      setPage(0);
      const message = t("설치된 앱 {{count}}개를 확인했습니다.", { count: formatCount(next.applications.length) });
      setStatus(message);
      onStatus(message);
    } catch (reason) {
      if (mounted.current) setError(t("앱 목록을 불러오지 못했습니다: {{detail}}", { detail: detail(reason) }));
    } finally {
      loadingRef.current = false;
      if (mounted.current) setLoading(false);
    }
  }

  async function openLocation(path: string) {
    if (openingRef.current) return;
    openingRef.current = true;
    setOpening(true);
    setError(null);
    try {
      await revealPath(path);
      if (mounted.current) setStatus(t("파일 탐색기에 위치를 표시했습니다."));
    } catch (reason) {
      if (mounted.current) setError(t("위치를 표시하지 못했습니다: {{detail}}", { detail: detail(reason) }));
    } finally {
      openingRef.current = false;
      if (mounted.current) setOpening(false);
    }
  }

  async function openSettings() {
    if (busy || openingRef.current || inventory?.platform !== "windows") return;
    openingRef.current = true;
    setOpening(true);
    setError(null);
    try {
      await openApplicationUninstallSettings();
      if (mounted.current) {
        const message = t("Windows 제거 화면을 열었습니다. 앱을 직접 선택해 제거한 뒤 목록을 새로 고치세요. 제거 완료를 확인한 것은 아닙니다.");
        setStatus(message);
        onStatus(message);
      }
    } catch (reason) {
      if (mounted.current) setError(t("제거 화면을 열지 못했습니다: {{detail}}", { detail: detail(reason) }));
    } finally {
      openingRef.current = false;
      if (mounted.current) setOpening(false);
    }
  }

  function beginReview(target: ReviewTarget, trigger: HTMLButtonElement) {
    if (busy || loadingRef.current || reviewRef.current || uncertain) return;
    reviewRef.current = true;
    triggerRef.current = trigger;
    setReview(target);
  }

  function closeReview() {
    const showRelated = review?.kind === "bundle" && related?.application.id === review.application.id && movedApps.has(review.application.id);
    reviewRef.current = false;
    setReview(null);
    window.requestAnimationFrame(() => {
      if (showRelated && relatedRef.current) {
        relatedRef.current.focus({ preventScroll: true });
        relatedRef.current.scrollIntoView({ block: "start" });
        return;
      }
      const trigger = triggerRef.current;
      if (trigger?.isConnected && !trigger.disabled) trigger.focus();
      else refreshRef.current?.focus();
    });
  }

  function completed(target: ReviewTarget, plan: ApplicationTrashPlan, outcome: TrashOperationResult | null) {
    if (outcome) {
      setResult(outcome);
      if (target.kind === "bundle" && outcome.movedCount > 0) {
        setMovedApps((current) => new Set(current).add(target.application.id));
        setRelated({ application: target.application, inventoryId: target.inventoryId, candidates: plan.relatedData });
        setSelected(new Set());
      } else if (target.kind === "data") {
        // Consume this selection, not the untouched candidates. A removed app
        // is absent from the next inventory, so dropping them would strand the
        // separate opt-in review. Any retry still prepares a new native plan.
        const movedPaths = new Set(outcome.items.filter((item) => item.status === "moved").map((item) => item.path));
        setSelected(new Set());
        setRelated((current) => current ? {
          ...current,
          candidates: current.candidates.filter((candidate) => !movedPaths.has(candidate.path)),
        } : null);
      }
      const message = t("요청 {{requested}}개 중 {{moved}}개를 휴지통으로 이동했습니다.", { requested: formatCount(outcome.requestedCount), moved: formatCount(outcome.movedCount) });
      setStatus(message);
      onStatus(message);
    } else {
      setUncertain(true);
      setRelated(null);
      setSelected(new Set());
    }
    onMutated();
  }

  const filtered = useMemo(() => {
    const needle = query.trim().toLocaleLowerCase();
    return inventory?.applications.filter((app) => !needle || `${app.displayName}\n${app.publisher ?? ""}\n${app.installLocation ?? ""}`.toLocaleLowerCase().includes(needle)) ?? [];
  }, [inventory, query]);
  const pageCount = Math.max(1, Math.ceil(filtered.length / PAGE_SIZE));
  const currentPage = Math.min(page, pageCount - 1);
  const visibleApps = filtered.slice(currentPage * PAGE_SIZE, (currentPage + 1) * PAGE_SIZE);
  const blocked = busy || loading || opening || !!review;

  return <section className="applications-view" aria-labelledby="applications-title">
    <header className="applications-header">
      <div><p className="eyebrow">{t("앱 관리")}</p><h2 id="applications-title"><AppWindow size={27} aria-hidden="true" />{t("설치된 앱")}</h2>
        <p>{t("앱을 확인하고, 운영체제에 맞는 제거 절차로 정리합니다.")}</p></div>
      <button ref={refreshRef} type="button" className="secondary-button" disabled={blocked} onClick={() => void refresh()}>
        <RefreshCw size={17} aria-hidden="true" className={loading ? "spin" : undefined} />{t("목록 새로 고침")}
      </button>
    </header>
    <div className="applications-guidance">
      <ShieldCheck size={21} aria-hidden="true" />
      <div><strong>{t("정식 제거 절차를 우선 사용하세요.")}</strong>
        <p>{inventory?.platform === "windows"
          ? t("Windows에서는 설치된 앱 설정에서 제거합니다. 앱 폴더나 AppData를 추측해 삭제하지 않습니다.")
          : t("전용 제거 프로그램이 있으면 먼저 사용하세요. 일반 Mac 앱만 본체를 휴지통으로 옮기며, 설정과 문서는 자동 삭제하지 않습니다.")}</p>
        <p>{t("목록은 설치 정보만 조회합니다. 앱 전체 용량과 아이콘을 미리 읽지 않습니다.")}</p>
      </div>
    </div>
    {inventory?.platform === "windows" ? <button type="button" className="primary-button applications-settings" disabled={blocked} onClick={() => void openSettings()}>
      <ExternalLink size={17} aria-hidden="true" />{t("Windows 제거 화면 열기")}
    </button> : null}
    <div className="applications-searchbar">
      <label className="applications-search"><Search size={18} aria-hidden="true" /><span className="sr-only">{t("앱 이름·제작사·경로 검색")}</span>
        <input type="search" value={query} placeholder={t("앱 이름·제작사·경로 검색")} onChange={(event) => { setQuery(event.target.value); setPage(0); }} />
      </label>
      <span>{t("{{count}}개", { count: formatCount(filtered.length) })}</span>
    </div>
    {error ? <p className="applications-error" role="alert">{error}</p> : null}
    {status ? <p className="applications-status" role="status">{status}</p> : null}
    {uncertain ? <p className="applications-error" role="alert">{t("결과를 확인하지 못했습니다. 휴지통과 작업 기록을 먼저 확인한 뒤 목록을 새로 고치세요. 같은 요청을 자동으로 재시도하지 않습니다.")}</p> : null}
    {loading ? <p className="applications-empty" role="status"><LoaderCircle className="spin" size={21} aria-hidden="true" />{t("설치 정보를 불러오는 중입니다.")}</p> : null}
    {!loading && inventory?.platform === "unsupported" ? <p className="applications-empty">{t("이 운영체제에서는 앱 관리 기능을 지원하지 않습니다.")}</p> : null}
    {!loading && inventory?.platform !== "unsupported" && inventory && visibleApps.length === 0 ? <p className="applications-empty">{t("표시할 앱이 없습니다. 검색어나 목록을 확인하세요.")}</p> : null}
    <ul className="applications-list" aria-label={t("설치된 앱")}>
      {visibleApps.map((app) => <li key={app.id} className="applications-row">
        <span className="applications-icon"><AppWindow size={24} aria-hidden="true" /></span>
        <div className="applications-identity"><strong>{app.displayName}</strong><p>{app.displayVersion ? t("버전 {{version}}", { version: app.displayVersion }) : t("버전 미확인")}{app.publisher ? ` · ${app.publisher}` : ""}</p>
          {app.installLocation ? <code>{app.installLocation}</code> : <p>{t("설치 위치 미확인")}</p>}
          {app.protectionReason ? <p className="applications-protection"><ShieldCheck size={14} aria-hidden="true" />{app.protectionReason}</p> : null}
        </div>
        <span className="applications-size">{app.estimatedBytes === null ? t("용량 미측정") : formatBytes(app.estimatedBytes)}</span>
        <div className="applications-row-actions">
          {app.installLocation ? <button type="button" className="secondary-button" disabled={blocked} onClick={() => void openLocation(app.installLocation!)} aria-label={t("{{name}} 위치 표시", { name: app.displayName })}>
            <FolderOpen size={16} aria-hidden="true" />{t("위치 표시")}
          </button> : null}
          {inventory?.platform === "macos" && app.removalMode === "trashBundle" ? movedApps.has(app.id)
            ? <span className="applications-moved"><CheckCircle2 size={17} aria-hidden="true" />{t("앱 본체 이동 완료")}</span>
            : <button type="button" className="secondary-button" disabled={blocked || uncertain} onClick={(event) => beginReview({ application: app, inventoryId: inventory.inventoryId, kind: "bundle", candidateIds: [] }, event.currentTarget)} aria-label={t("{{name}} 앱 본체 정리 검토", { name: app.displayName })}>
              <Trash2 size={16} aria-hidden="true" />{t("앱 본체 정리 검토")}
            </button> : null}
        </div>
      </li>)}
    </ul>
    {filtered.length > PAGE_SIZE ? <nav className="applications-pagination" aria-label={t("앱 목록 페이지")}>
      <button type="button" className="secondary-button" disabled={currentPage === 0} onClick={() => setPage(currentPage - 1)}><ChevronLeft size={17} aria-hidden="true" />{t("이전")}</button>
      <span>{t("{{current}} / {{total}} 페이지", { current: formatCount(currentPage + 1), total: formatCount(pageCount) })}</span>
      <button type="button" className="secondary-button" disabled={currentPage + 1 >= pageCount} onClick={() => setPage(currentPage + 1)}>{t("다음")}<ChevronRight size={17} aria-hidden="true" /></button>
    </nav> : null}
    {inventory?.issues.length ? <details className="applications-issues"><summary>{t("일부 설치 정보를 확인하지 못했습니다.")}</summary><ul>{inventory.issues.map((issue, index) => <li key={index}>{issue}</li>)}</ul></details> : null}
    {result ? <ApplicationResult result={result} /> : null}
    {related ? <section ref={relatedRef} className="applications-related" tabIndex={-1} aria-labelledby="applications-related-title">
      <h2 id="applications-related-title">{t("{{name}}의 관련 데이터 검토", { name: related.application.displayName })}</h2>
      <p>{t("앱 본체와 별개입니다. 정확한 앱 식별자로 연결된 캐시·환경설정만 후보이며, 필요할 수 있으므로 직접 선택하세요.")}</p>
      <p>{t("문서·공유 데이터·Application Support·Containers는 이 목록에 포함하지 않습니다.")}</p>
      {related.candidates.length === 0 ? <p>{t("현재 확인된 관련 데이터 후보가 없습니다. 모든 잔여 데이터가 없다는 뜻은 아닙니다.")}</p> : <>
        <ul className="applications-candidates">{related.candidates.map((candidate) => <li key={candidate.id}>
          <label><input type="checkbox" checked={selected.has(candidate.id)} disabled={blocked || uncertain} onChange={(event) => setSelected((current) => {
            const next = new Set(current);
            if (event.target.checked) next.add(candidate.id); else next.delete(candidate.id);
            return next;
          })} /><CandidateDetails candidate={candidate} /></label>
          <button type="button" className="secondary-button" disabled={blocked} onClick={() => void openLocation(candidate.path)} aria-label={t("{{name}} 위치 표시", { name: candidate.path })}><FolderOpen size={16} aria-hidden="true" />{t("위치 표시")}</button>
        </li>)}</ul>
        <div className="applications-related-actions"><span>{t("선택 {{count}}개", { count: formatCount(selected.size) })}</span>
          <button type="button" className="secondary-button" disabled={blocked || !selected.size || uncertain} onClick={(event) => beginReview({ application: related.application, inventoryId: related.inventoryId, kind: "data", candidateIds: [...selected] }, event.currentTarget)}>
            <Trash2 size={16} aria-hidden="true" />{t("선택 데이터 정리 검토")}
          </button>
          <button type="button" className="secondary-button" disabled={blocked} onClick={() => { setRelated(null); setSelected(new Set()); }}>{t("데이터 유지하고 닫기")}</button>
        </div>
      </>}
    </section> : null}
    {review ? <ApplicationReview target={review} busy={busy} onBusyChange={onBusyChange} onClose={closeReview} onCompleted={completed} /> : null}
  </section>;
}

function CandidateDetails({ candidate }: { candidate: ApplicationDataCandidate }) {
  const { t } = useLanguage();
  return <span className="applications-candidate-detail"><strong>{t(candidate.kind === "cache" ? "캐시" : "환경설정")} · {candidate.estimatedBytes === null ? t("용량 미측정") : formatBytes(candidate.estimatedBytes)}</strong>
    <code>{candidate.path}</code><span>{candidate.evidence}</span></span>;
}

function ApplicationResult({ result }: { result: TrashOperationResult }) {
  const { t } = useLanguage();
  return <section className="applications-result" aria-label={t("휴지통 이동 결과")}>
    <h2>{t("휴지통 이동 결과")}</h2>
    <p>{t("요청 {{requested}}개 중 {{moved}}개를 휴지통으로 이동했습니다.", { requested: formatCount(result.requestedCount), moved: formatCount(result.movedCount) })}</p>
    <p>{t("휴지통 이동만으로는 디스크 공간이 확보되지 않을 수 있습니다.")}</p>
    {!result.journalComplete ? <p className="applications-error">{t("작업 기록 저장이 완전하지 않습니다. 실제 휴지통과 작업 기록을 확인하세요.")}</p> : null}
    <details><summary>{t("항목별 결과 보기")}</summary><ul>{result.items.map((item, index) => <li key={index}><code>{item.path}</code><span>{t(item.status === "moved" ? "이동 완료" : item.status === "failed" ? "실패" : "건너뜀")}</span>{item.message ? <p>{item.message}</p> : null}</li>)}</ul></details>
  </section>;
}

function ApplicationReview({ target, busy, onBusyChange, onClose, onCompleted }: {
  target: ReviewTarget;
  busy: boolean;
  onBusyChange?: (busy: boolean) => void;
  onClose: () => void;
  onCompleted: (target: ReviewTarget, plan: ApplicationTrashPlan, result: TrashOperationResult | null) => void;
}) {
  const { t } = useLanguage();
  const dialogRef = useRef<HTMLDialogElement>(null);
  const cancelRef = useRef<HTMLButtonElement>(null);
  const submitting = useRef(false);
  const consumedRef = useRef(false);
  const [plan, setPlan] = useState<ApplicationTrashPlan | null>(null);
  const [preparing, setPreparing] = useState(true);
  const [running, setRunning] = useState(false);
  const [consumed, setConsumed] = useState(false);
  const [acknowledged, setAcknowledged] = useState(false);
  const [noUninstaller, setNoUninstaller] = useState(false);
  const [now, setNow] = useState(Date.now());
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<TrashOperationResult | null>(null);
  const bundle = target.kind === "bundle";

  useEffect(() => {
    let disposed = false;
    let issued: ApplicationTrashPlan | null = null;
    const dialog = dialogRef.current;
    dialog?.showModal();
    cancelRef.current?.focus();
    // StrictMode replays effect setup/cleanup. Do not start a native exclusive
    // operation until this setup survives cleanup, or the replay would compete
    // with (and discard) its own first review.
    const preparation = Promise.resolve().then(() => {
      if (disposed) return null;
      return target.kind === "bundle"
        ? prepareApplicationTrash(target.inventoryId, target.application.id)
        : prepareApplicationDataTrash(target.inventoryId, target.application.id, target.candidateIds);
    });
    void preparation.then((next) => {
      if (!next) return;
      issued = next;
      if (disposed) void dismissApplicationPlan(next.planId).catch(() => {});
      else { setPlan(next); setNow(Date.now()); }
    }).catch((reason: unknown) => {
      if (!disposed) setError(t("검토를 준비하지 못했습니다: {{detail}}", { detail: detail(reason) }));
    }).finally(() => { if (!disposed) setPreparing(false); });
    const timer = window.setInterval(() => setNow(Date.now()), 500);
    return () => {
      disposed = true;
      window.clearInterval(timer);
      if (issued && !consumedRef.current) void dismissApplicationPlan(issued.planId).catch(() => {});
      dialog?.close();
    };
  }, [target]);

  useEffect(() => { if (consumed && !running) cancelRef.current?.focus(); }, [consumed, running]);

  async function confirm() {
    if (busy || submitting.current || consumedRef.current || !plan || plan.expiresAtUnixMs <= Date.now() || !acknowledged || (bundle && !noUninstaller)) return;
    submitting.current = true;
    consumedRef.current = true;
    setConsumed(true);
    setRunning(true);
    setError(null);
    onBusyChange?.(true);
    try {
      const outcome = bundle ? await confirmApplicationTrash(plan.planId) : await confirmApplicationDataTrash(plan.planId);
      setResult(outcome);
      onCompleted(target, plan, outcome);
    } catch (reason) {
      setError(t("결과를 확인하지 못했습니다. 휴지통과 작업 기록을 확인하세요: {{detail}}", { detail: detail(reason) }));
      onCompleted(target, plan, null);
    } finally {
      submitting.current = false;
      setRunning(false);
      onBusyChange?.(false);
    }
  }

  const expired = !!plan && plan.expiresAtUnixMs <= now;
  return <dialog ref={dialogRef} className="safety-dialog applications-dialog" aria-labelledby="application-review-title" aria-describedby="application-review-scope"
    onCancel={(event) => { event.preventDefault(); if (!submitting.current) onClose(); }}>
    <header><span><AlertTriangle size={23} aria-hidden="true" /></span><div><small>{t("휴지통 이동 최종 확인")}</small><h2 id="application-review-title">{t(bundle ? "앱 본체 정리 검토" : "선택 데이터 정리 검토")}</h2></div></header>
    <p className="applications-review-name">{plan?.displayName ?? target.application.displayName}</p>
    {bundle ? <code className="applications-review-path">{plan?.path ?? target.application.installLocation}</code> : null}
    <div className="safety-dialog__warning" id="application-review-scope"><AlertTriangle size={18} aria-hidden="true" /><p>{bundle
      ? t("이 앱 본체만 휴지통으로 옮깁니다. 설정·문서·별도 서비스는 제거하지 않습니다. 전용 제거 프로그램이 필요한 앱은 먼저 정식 제거 절차를 사용하세요.")
      : t("아래에서 확인한 데이터만 휴지통으로 옮깁니다. 환경설정이 초기화될 수 있으며, 사용자 문서나 다른 앱의 공유 데이터는 선택하면 안 됩니다.")}</p></div>
    {preparing ? <p className="safety-dialog__intro" role="status"><LoaderCircle size={17} className="spin" aria-hidden="true" />{t("대상과 실행 중 여부를 확인하고 있습니다.")}</p> : null}
    {!bundle && plan ? <ul className="applications-candidates applications-review-candidates">{plan.relatedData.map((candidate) => <li key={candidate.id}><CandidateDetails candidate={candidate} /></li>)}</ul> : null}
    {plan?.warnings.length ? <ul className="applications-review-warnings">{plan.warnings.map((warning, index) => <li key={index}>{warning}</li>)}</ul> : null}
    {bundle ? <label className="review-acknowledgement"><input type="checkbox" checked={noUninstaller} disabled={preparing || consumed || expired || !plan} onChange={(event) => setNoUninstaller(event.target.checked)} />
      <span>{t("이 앱에 전용 제거 프로그램이 필요하지 않음을 확인했습니다.")}</span></label> : null}
    <label className="review-acknowledgement"><input type="checkbox" checked={acknowledged} disabled={preparing || consumed || expired || !plan} onChange={(event) => setAcknowledged(event.target.checked)} />
      <span>{t(bundle ? "표시한 앱 본체만 이동하며, 설정과 문서는 남겨 둠을 확인했습니다." : "선택 데이터의 경로와 영향을 확인했으며, 해당 데이터만 이동합니다.")}</span></label>
    {expired && !consumed ? <p className="safety-dialog__error" role="alert">{t("확인 시간이 만료됐습니다. 창을 닫고 다시 검토하세요.")}</p> : null}
    {running ? <p className="safety-dialog__intro" role="status"><LoaderCircle size={17} className="spin" aria-hidden="true" />{t("휴지통 이동을 처리하고 있습니다. 완료할 때까지 기다려 주세요.")}</p> : null}
    {result ? <div role="status"><ApplicationResult result={result} />{bundle && result.movedCount > 0 ? <p>{t("창을 닫으면 관련 데이터를 별도로 검토할 수 있습니다. 자동 선택하거나 삭제하지 않습니다.")}</p> : null}</div> : null}
    {error ? <p className="safety-dialog__error" role="alert">{error}</p> : null}
    <footer><button ref={cancelRef} type="button" className="secondary-button" disabled={running} onClick={onClose}>{t(consumed ? "닫기" : "취소")}</button>
      {!consumed ? <button type="button" className="trash-confirm-button" disabled={busy || preparing || !plan || expired || !acknowledged || (bundle && !noUninstaller) || (!bundle && !plan?.relatedData.length)} onClick={() => void confirm()}>
        <Trash2 size={16} aria-hidden="true" />{t(bundle ? "앱 본체만 휴지통으로 이동" : "선택 데이터만 휴지통으로 이동")}
      </button> : null}
    </footer>
  </dialog>;
}
