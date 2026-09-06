import {
  Activity,
  AlertTriangle,
  CheckCircle2,
  Gauge,
  Info,
  MemoryStick,
  Power,
  RefreshCw,
  ShieldCheck,
  Sparkles,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState, type MouseEvent } from "react";
import { MetricRing } from "../components/MetricRing";
import { ProcessTerminationDialog } from "../components/ProcessTerminationDialog";
import { usePerformanceMonitor } from "../hooks/usePerformanceMonitor";
import {
  cleanAppMemory,
  executeGracefulProcessTermination,
  prepareGracefulProcessTermination,
} from "../lib/bridge";
import { formatBytes, formatDate, formatDateTimeAttribute } from "../lib/format";
import {
  memoryUsagePercent,
  memoryCleanupReleasedBytes,
  performanceProcessKey,
  sortPerformanceProcesses,
  type PerformanceSort,
} from "../lib/performancePresentation";
import type {
  AppMemoryCleanupResult,
  PerformanceProcessUsage,
  TerminationOutcome,
  TerminationPreview,
  TerminationPreviewOutcome,
} from "../types";
import { useLanguage, type Translate } from "../i18n";

export function PerformanceView() {
  const { t } = useLanguage();
  const monitor = usePerformanceMonitor();
  const [sort, setSort] = useState<PerformanceSort>("cpu");
  const [preparingTargetId, setPreparingTargetId] = useState<string | null>(null);
  const [preview, setPreview] = useState<TerminationPreview | null>(null);
  const [terminationBusy, setTerminationBusy] = useState(false);
  const [dialogError, setDialogError] = useState<string | null>(null);
  const [actionMessage, setActionMessage] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [memoryCleanupBusy, setMemoryCleanupBusy] = useState(false);
  const [memoryCleanupResult, setMemoryCleanupResult] = useState<AppMemoryCleanupResult | null>(null);
  const [memoryCleanupError, setMemoryCleanupError] = useState<string | null>(null);
  const [refreshAnnouncement, setRefreshAnnouncement] = useState("");
  const returnFocusRef = useRef<HTMLButtonElement | null>(null);
  const refreshButtonRef = useRef<HTMLButtonElement | null>(null);
  const wasRefreshingRef = useRef(false);
  const snapshot = monitor.snapshot;
  const memoryPercent = memoryUsagePercent(snapshot);
  const processes = useMemo(
    () => sortPerformanceProcesses(snapshot?.processes ?? [], sort),
    [snapshot?.processes, sort],
  );

  useEffect(() => {
    if (monitor.refreshing) {
      setRefreshAnnouncement("");
    } else if (wasRefreshingRef.current && !monitor.error) {
      setRefreshAnnouncement(t("성능 상태를 새로 고쳤습니다."));
    }
    wasRefreshingRef.current = monitor.refreshing;
  }, [monitor.error, monitor.refreshing, t]);

  async function prepareTermination(
    event: MouseEvent<HTMLButtonElement>,
    process: PerformanceProcessUsage,
  ) {
    if (!snapshot || monitor.stale || !process.targetId) return;
    returnFocusRef.current = event.currentTarget;
    setPreparingTargetId(process.targetId);
    setActionMessage(null);
    setActionError(null);
    try {
      const response = await prepareGracefulProcessTermination(
        snapshot.snapshotId,
        process.targetId,
      );
      if (response.outcome === "ready" && response.preview) {
        setDialogError(null);
        setPreview(response.preview);
      } else {
        setActionError(previewOutcomeMessage(response.outcome, t));
        void monitor.refresh();
      }
    } catch (reason) {
      setActionError(performanceActionError(reason, t));
    } finally {
      setPreparingTargetId(null);
    }
  }

  async function cleanMemory() {
    if (memoryCleanupBusy) return;
    setMemoryCleanupBusy(true);
    setMemoryCleanupResult(null);
    setMemoryCleanupError(null);
    try {
      const result = await cleanAppMemory();
      if (result.outcome === "completed") {
        await monitor.refresh();
        setMemoryCleanupResult(result);
      } else if (result.outcome === "busy") {
        setMemoryCleanupError(t("메모리 정리가 이미 진행 중입니다."));
      } else {
        setMemoryCleanupError(t("이 플랫폼에서는 BroomSweepy 메모리 정리를 지원하지 않습니다."));
      }
    } catch (reason) {
      setMemoryCleanupError(memoryCleanupActionError(reason, t));
    } finally {
      setMemoryCleanupBusy(false);
    }
  }

  async function executeTermination() {
    if (!preview || terminationBusy) return;
    setTerminationBusy(true);
    setDialogError(null);
    try {
      const result = await executeGracefulProcessTermination({
        previewId: preview.previewId,
        unsavedWorkAcknowledged: true,
      });
      const message = terminationOutcomeMessage(
        result.outcome,
        result.displayName ?? preview.displayName,
        t,
      );
      setPreview(null);
      setDialogError(null);
      if (isTerminationFailure(result.outcome)) setActionError(message);
      else setActionMessage(message);
      await monitor.refresh();
      restoreDialogFocus();
    } catch (reason) {
      setDialogError(performanceActionError(reason, t));
    } finally {
      setTerminationBusy(false);
    }
  }

  function closeDialog() {
    if (terminationBusy) return;
    setPreview(null);
    setDialogError(null);
    restoreDialogFocus();
  }

  function restoreDialogFocus() {
    requestAnimationFrame(() => {
      const trigger = returnFocusRef.current;
      const target = trigger?.isConnected && !trigger.disabled
        ? trigger
        : refreshButtonRef.current;
      target?.focus({ preventScroll: true });
    });
  }

  return (
    <div className="view-stack performance-view">
      <section className="performance-status-strip" aria-label={t("성능 측정 상태")}>
        <span className={monitor.stale ? "is-stale" : "is-current"}>
          <Activity size={15} aria-hidden="true" />
          {monitor.stale ? t("업데이트 지연") : t("실시간 측정")}
        </span>
        {snapshot ? (
          <time dateTime={formatDateTimeAttribute(snapshot.capturedAtUnixMs)}>
            {t("마지막 측정 {{date}}", { date: formatDate(snapshot.capturedAtUnixMs) })}
          </time>
        ) : null}
        <button
          ref={refreshButtonRef}
          className="icon-button performance-refresh"
          type="button"
          aria-label={t("성능 상태 새로 고침")}
          disabled={monitor.loading || monitor.refreshing}
          onClick={() => void monitor.refresh()}
        >
          <RefreshCw className={monitor.refreshing ? "is-spinning" : undefined} size={17} aria-hidden="true" />
        </button>
      </section>
      <span className="sr-only" role="status" aria-live="polite">
        {refreshAnnouncement}
      </span>

      {monitor.error && !snapshot ? (
        <section className="performance-error" role="alert">
          <AlertTriangle size={20} aria-hidden="true" />
          <div>
            <strong>{t("성능 상태를 확인하지 못했습니다")}</strong>
            <p>{monitor.error}</p>
          </div>
          <button className="secondary-button" type="button" onClick={() => void monitor.refresh()}>
            {t("다시 시도")}
          </button>
        </section>
      ) : null}

      {monitor.showLoading && !snapshot ? (
        <section className="performance-loading" role="status">
          <span className="performance-loading__ring" aria-hidden="true" />
          <strong>{t("성능 상태 측정 중…")}</strong>
          <p>{t("정확한 CPU 사용량을 위해 짧은 표본을 준비하고 있습니다.")}</p>
        </section>
      ) : null}

      {snapshot ? (
        <>
          {monitor.error ? (
            <section className="performance-stale-warning" role="status">
              <AlertTriangle size={17} aria-hidden="true" />
              <span>
                <strong>{t("업데이트 지연")}</strong>
                {t("마지막 정상 측정값을 보여줍니다. 종료 요청은 새 측정 전까지 꺼집니다.")}
              </span>
              <button type="button" onClick={() => void monitor.refresh()}>{t("다시 시도")}</button>
            </section>
          ) : null}

          <section className="performance-instrument glass-panel" aria-labelledby="performance-instrument-title">
            <header className="performance-instrument__header">
              <p className="eyebrow">{t("실시간 상태")}</p>
              <h2 id="performance-instrument-title">{t("시스템 성능")}</h2>
            </header>
            <div className="performance-instrument__visual">
              <MetricRing
                label={t("CPU 사용량")}
                value={snapshot.cpuUsagePercent}
                valueText={`${Math.round(snapshot.cpuUsagePercent)}%`}
                detail={t("{{count}}개 논리 코어", { count: snapshot.logicalCpuCount })}
              />
              <dl className="performance-instrument__metrics">
                <div>
                  <dt>{t("논리 코어")}</dt>
                  <dd>{snapshot.logicalCpuCount.toLocaleString()}</dd>
                </div>
                <div>
                  <dt>{t("표본 시간")}</dt>
                  <dd>{snapshot.sampleWindowMs.toLocaleString()} ms</dd>
                </div>
              </dl>
              <p className="performance-instrument__note">
                <Info size={16} aria-hidden="true" />
                {t("CPU 부하는 실행 중인 작업을 줄이면 낮아집니다. 아래에서 많이 사용하는 앱을 확인하세요.")}
              </p>
            </div>

            <div className="performance-memory">
              <MetricRing
                label={t("메모리 사용량")}
                value={memoryPercent}
                valueText={`${Math.round(memoryPercent)}%`}
                detail={t("{{used}} / {{total}} 사용", {
                  used: formatBytes(snapshot.memory.usedBytes),
                  total: formatBytes(snapshot.memory.totalBytes),
                })}
                tone="memory"
              />
              <dl className="performance-instrument__metrics">
                <div>
                  <dt>{t("사용 가능")}</dt>
                  <dd>{formatBytes(snapshot.memory.availableBytes)}</dd>
                </div>
                <div>
                  <dt>{t("스왑 사용량")}</dt>
                  <dd>{formatBytes(snapshot.memory.usedSwapBytes)} / {formatBytes(snapshot.memory.totalSwapBytes)}</dd>
                </div>
              </dl>
              <div className="performance-memory-cleaner">
                {snapshot.capabilities.appMemoryCleanupAvailable ? (
                  <button
                    className="performance-memory-cleaner__button"
                    type="button"
                    aria-busy={memoryCleanupBusy}
                    aria-describedby="performance-memory-cleaner-scope"
                    disabled={memoryCleanupBusy}
                    onClick={() => void cleanMemory()}
                  >
                    {memoryCleanupBusy ? (
                      <RefreshCw className="is-spinning" size={20} aria-hidden="true" />
                    ) : (
                      <Sparkles size={20} aria-hidden="true" />
                    )}
                    <span>
                      <strong>{memoryCleanupBusy ? t("앱 메모리 정리 중…") : t("앱 메모리 정리")}</strong>
                      <small>{t("BroomSweepy 전용")}</small>
                    </span>
                  </button>
                ) : null}

                <span className="sr-only" role="status" aria-live="polite" aria-atomic="true">
                  {memoryCleanupBusy ? t("앱 메모리 정리 중…") : ""}
                </span>

                {memoryCleanupResult ? (
                  <p
                    className="performance-memory-cleaner__result is-success"
                    role="status"
                    aria-live="polite"
                    aria-atomic="true"
                  >
                    <CheckCircle2 size={17} aria-hidden="true" />
                    {memoryCleanupResultMessage(memoryCleanupResult, t)}
                  </p>
                ) : null}
                {memoryCleanupError ? (
                  <p className="performance-memory-cleaner__result is-error" role="alert">
                    <AlertTriangle size={17} aria-hidden="true" />
                    {memoryCleanupError}
                  </p>
                ) : null}

                <p id="performance-memory-cleaner-scope" className="performance-memory-cleaner__scope">
                  <Info size={16} aria-hidden="true" />
                  {snapshot.capabilities.appMemoryCleanupAvailable
                    ? t("BroomSweepy 본체가 사용하지 않는 메모리만 macOS에 반환합니다. 다른 앱의 메모리는 건드리지 않습니다.")
                    : t("이 플랫폼에서는 사용량만 측정하며 BroomSweepy 메모리 정리는 지원하지 않습니다.")}
                </p>
              </div>
            </div>
          </section>

          <section className="performance-process-panel" aria-labelledby="performance-process-title">
            <header className="performance-process-panel__header">
              <div>
                <p className="eyebrow">{t("실제 측정")}</p>
                <h2 id="performance-process-title">{t("많이 사용하는 앱")}</h2>
                <p>{t("실제 CPU와 메모리 사용량 순으로 비교합니다.")}</p>
              </div>
              <div className="performance-sort" aria-label={t("정렬 기준")}>
                <button
                  type="button"
                  className={sort === "cpu" ? "is-active" : ""}
                  aria-pressed={sort === "cpu"}
                  onClick={() => setSort("cpu")}
                >
                  <Gauge size={15} aria-hidden="true" />
                  {t("CPU순")}
                </button>
                <button
                  type="button"
                  className={sort === "memory" ? "is-active" : ""}
                  aria-pressed={sort === "memory"}
                  onClick={() => setSort("memory")}
                >
                  <MemoryStick size={15} aria-hidden="true" />
                  {t("메모리순")}
                </button>
              </div>
            </header>

            {actionMessage ? (
              <p className="performance-action-message" role="status">
                <CheckCircle2 size={16} aria-hidden="true" />{actionMessage}
              </p>
            ) : null}
            {actionError ? (
              <p className="performance-action-error" role="alert">
                <AlertTriangle size={16} aria-hidden="true" />{actionError}
              </p>
            ) : null}

            {processes.length > 0 ? (
              <table className="performance-process-table" aria-label={t("많이 사용하는 앱")}>
                <thead>
                  <tr className="performance-process-table__head">
                    <th scope="col">{t("앱")}</th>
                    <th scope="col">{t("CPU")}</th>
                    <th scope="col">{t("메모리")}</th>
                    <th scope="col">{t("상태")}</th>
                  </tr>
                </thead>
                <tbody>
                  {processes.map((process) => (
                    <ProcessRow
                      key={performanceProcessKey(process)}
                      process={process}
                      stale={monitor.stale}
                      preparing={preparingTargetId === process.targetId}
                      onPrepare={prepareTermination}
                    />
                  ))}
                </tbody>
              </table>
            ) : (
              <div className="performance-process-empty">
                <Activity size={22} aria-hidden="true" />
                <strong>{t("표시할 실행 중인 앱이 없습니다")}</strong>
                <p>{t("잠시 뒤 다시 측정하거나 새로 고침해 주세요.")}</p>
              </div>
            )}

            <footer className="performance-process-panel__footer">
              <ShieldCheck size={17} aria-hidden="true" />
              <p>
                {snapshot.capabilities.gracefulTerminationAvailable
                  ? t("종료 요청은 macOS의 일반 앱에만 보내며, 대상을 다시 확인하고 강제 종료하지 않습니다.")
                  : t("이 플랫폼에서는 프로세스 사용량만 표시하며 앱 종료 요청은 지원하지 않습니다.")}
                {snapshot.processesTruncated
                  ? ` ${t("CPU와 메모리 상위 항목만 표시합니다.")}`
                  : ""}
              </p>
            </footer>
          </section>
        </>
      ) : null}

      <ProcessTerminationDialog
        preview={preview}
        busy={terminationBusy}
        error={dialogError}
        onConfirm={() => void executeTermination()}
        onClose={closeDialog}
      />
    </div>
  );
}

function ProcessRow({
  process,
  stale,
  preparing,
  onPrepare,
}: {
  process: PerformanceProcessUsage;
  stale: boolean;
  preparing: boolean;
  onPrepare: (event: MouseEvent<HTMLButtonElement>, process: PerformanceProcessUsage) => void;
}) {
  const { t } = useLanguage();
  const status = processStatus(process, t);
  return (
    <tr className="performance-process-row">
      <td className="performance-process-row__identity">
        <span className="performance-process-row__icon" aria-hidden="true">
          {process.displayName.slice(0, 1).toLocaleUpperCase()}
        </span>
        <span>
          <strong>{process.displayName}</strong>
          <small>
            PID {process.pid}
            {process.processCount > 1
              ? ` · ${t("{{count}}개 프로세스 포함", { count: process.processCount })}`
              : ""}
          </small>
        </span>
      </td>
      <td className="performance-process-row__metric" data-label={t("CPU")}>
        <b>{process.cpuMachinePercent.toFixed(1)}%</b>
        <span aria-hidden="true"><i style={{ transform: `scaleX(${Math.min(1, process.cpuMachinePercent / 100)})` }} /></span>
      </td>
      <td className="performance-process-row__metric" data-label={t("메모리")}>
        <b>{formatBytes(process.residentBytes)}</b>
      </td>
      <td className="performance-process-row__action">
        {process.canRequestTermination && process.targetId ? (
          <button
            type="button"
            disabled={stale || preparing}
            onClick={(event) => onPrepare(event, process)}
          >
            <Power size={15} aria-hidden="true" />
            {preparing ? t("확인 중…") : t("종료 요청")}
          </button>
        ) : (
          <span className={`performance-eligibility is-${process.terminationEligibility}`}>
            {status}
          </span>
        )}
      </td>
    </tr>
  );
}

function processStatus(process: PerformanceProcessUsage, t: Translate): string {
  switch (process.terminationEligibility) {
    case "selfApp":
      return t("현재 앱");
    case "protectedSystemApp":
      return t("보호됨");
    case "otherUser":
      return t("다른 사용자");
    case "identityUnavailable":
      return t("측정만 가능");
    case "notRegularApplication":
      return t("보조 앱");
    case "unsupported":
      return t("측정만 가능");
    default:
      return t("정상 종료 가능");
  }
}

function previewOutcomeMessage(outcome: TerminationPreviewOutcome, t: Translate): string {
  switch (outcome) {
    case "staleSnapshot":
      return t("측정값이 오래되어 종료하지 않았습니다. 목록을 새로 고쳐 주세요.");
    case "staleTarget":
      return t("앱 대상이 바뀌어 종료하지 않았습니다. 목록을 새로 고쳐 주세요.");
    case "protectedTarget":
      return t("보호된 앱은 종료할 수 없습니다.");
    case "unsupported":
      return t("이 플랫폼에서는 앱 종료 요청을 지원하지 않습니다.");
    default:
      return t("종료 확인을 준비하지 못했습니다.");
  }
}

function terminationOutcomeMessage(
  outcome: TerminationOutcome,
  displayName: string,
  t: Translate,
): string {
  switch (outcome) {
    case "terminated":
      return t("{{name}}이 종료됐습니다.", { name: displayName });
    case "requestSent":
      return t("{{name}}에 정상 종료 요청을 보냈습니다. 저장 확인 창이 열려 있을 수 있습니다.", { name: displayName });
    case "requestRejected":
      return t("{{name}}이 종료 요청을 받지 않았습니다.", { name: displayName });
    case "alreadyExited":
      return t("앱이 이미 종료됐습니다.");
    case "staleTarget":
      return t("앱 대상이 바뀌어 종료하지 않았습니다. 목록을 새로 고쳐 주세요.");
    case "protectedTarget":
      return t("보호된 앱은 종료할 수 없습니다.");
    case "previewExpired":
      return t("종료 확인이 만료됐습니다. 다시 선택해 주세요.");
    case "previewAlreadyUsed":
      return t("이미 사용한 종료 확인입니다.");
    case "acknowledgementRequired":
      return t("저장하지 않은 작업 위험 확인이 필요합니다.");
    case "unsupported":
      return t("이 플랫폼에서는 앱 종료 요청을 지원하지 않습니다.");
  }
}

function isTerminationFailure(outcome: TerminationOutcome): boolean {
  return outcome !== "terminated" && outcome !== "requestSent" && outcome !== "alreadyExited";
}

function performanceActionError(reason: unknown, t: Translate): string {
  const detail = reason instanceof Error
    ? reason.message
    : typeof reason === "string"
      ? reason
      : t("알 수 없는 오류가 발생했습니다");
  return t("성능 작업을 완료하지 못했습니다. {{detail}}", { detail });
}

function memoryCleanupResultMessage(
  result: AppMemoryCleanupResult,
  t: Translate,
): string {
  if (memoryCleanupReleasedBytes(result.allocatorReleasedBytes)) {
    return t("BroomSweepy가 {{amount}}의 자체 메모리를 macOS에 반환했습니다.", {
      amount: formatBytes(result.allocatorReleasedBytes),
    });
  }
  return t("메모리 정리를 완료했습니다. 현재 반환 가능한 자체 메모리는 없습니다.");
}

function memoryCleanupActionError(reason: unknown, t: Translate): string {
  const detail = reason instanceof Error
    ? reason.message
    : typeof reason === "string"
      ? reason
      : t("알 수 없는 오류가 발생했습니다");
  return t("메모리 정리를 완료하지 못했습니다. {{detail}}", { detail });
}
