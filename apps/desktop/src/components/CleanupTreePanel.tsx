import { AlertTriangle, ChevronDown, ChevronRight, File, Folder, RefreshCw, ShieldAlert, Trash2 } from "lucide-react";
import { useEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import { useLanguage } from "../i18n";
import { formatBytes, formatCount, formatDate } from "../lib/format";
import { cleanupPlanIsLive, knownCleanupRootIds, visibleCleanupRows } from "../lib/cleanupTree";
import type { CleanupTreeBranch, CleanupTreeNode, CleanupTreePanelProps } from "../types/cleanupTree";
import { FileOpenActions } from "./FileOpenActions";
import { SafetyActionDialog } from "./SafetyActionDialog";
import { TrashResultPanel } from "./TrashResultPanel";
import "./CleanupTreePanel.css";

function TreeCheckbox({ node, disabled, onChange, label }: {
  node: CleanupTreeNode; disabled: boolean; onChange: (checked: boolean) => void; label: string;
}) {
  const input = useRef<HTMLInputElement>(null);
  useEffect(() => { if (input.current) input.current.indeterminate = node.selectionState === "mixed"; }, [node.selectionState]);
  return <label className="cleanup-tree__check">
    <input ref={input} type="checkbox" checked={node.selectionState === "checked"}
      aria-label={label} disabled={disabled || !node.eligible}
      onChange={(event) => onChange(event.currentTarget.checked)} />
  </label>;
}

export function CleanupTreePanel({ tree, busy, error, result = null, progress = null,
  onLoadChildren, onSelectionChange, onPrepare, onConfirm, onDismissPlan, onRefresh, onCancel,
  onOpen, onReveal,
}: CleanupTreePanelProps) {
  const { t } = useLanguage();
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [localBusy, setLocalBusy] = useState(false);
  const [localError, setLocalError] = useState<string | null>(null);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [planUsed, setPlanUsed] = useState(false);
  const [now, setNow] = useState(Date.now);
  const requestInFlight = useRef(false);
  const mounted = useRef(true);
  const activeTree = useRef(tree?.treeId);
  activeTree.current = tree?.treeId;
  const returnFocus = useRef<HTMLButtonElement | null>(null);
  const titleRef = useRef<HTMLHeadingElement | null>(null);
  const resultRef = useRef<HTMLDivElement | null>(null);
  const locked = busy || localBusy;
  const expired = Boolean(tree && now >= tree.expiresAtUnixMs);
  const planLive = tree ? cleanupPlanIsLive(tree, now) : false;
  const display = useMemo(() => tree ? visibleCleanupRows(tree, expanded) : { rows: [], limited: false }, [tree, expanded]);
  const depths = useMemo(() => new Map(display.rows.map((row) => [row.node.id, row.depth])), [display]);
  const grouped = useMemo(() => {
    const groups = new Map<string | null, CleanupTreeNode[]>();
    for (const { node } of display.rows) {
      const rows = groups.get(node.parentId) ?? [];
      rows.push(node); groups.set(node.parentId, rows);
    }
    return groups;
  }, [display]);
  const branches = useMemo(() => new Map(tree?.branches.map((branch) => [branch.parentId, branch]) ?? []), [tree]);

  useEffect(() => { mounted.current = true; return () => { mounted.current = false; }; }, []);
  useEffect(() => { if (result) resultRef.current?.focus({ preventScroll: true }); }, [result]);
  useEffect(() => {
    setExpanded(new Set()); setDialogOpen(false); setLocalError(null); setPlanUsed(false);
  }, [tree?.treeId]);
  useEffect(() => {
    if (!tree?.plan) { setDialogOpen(false); return; }
    setPlanUsed(false); setLocalError(null); setDialogOpen(true);
  }, [tree?.plan?.id]);
  useEffect(() => {
    setNow(Date.now());
    if (!tree) return;
    const timer = window.setInterval(() => setNow(Date.now()), 1_000);
    return () => window.clearInterval(timer);
  }, [tree?.treeId, tree?.plan?.id]);

  async function run(action: () => Promise<unknown> | void) {
    if (requestInFlight.current || busy) return;
    requestInFlight.current = true;
    const sourceId = activeTree.current;
    setLocalBusy(true); setLocalError(null);
    try { await action(); }
    catch (reason) {
      if (mounted.current && activeTree.current === sourceId) setLocalError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      requestInFlight.current = false;
      if (mounted.current) setLocalBusy(false);
    }
  }

  function closeDialog() {
    if (locked) return;
    setDialogOpen(false);
    void run(onDismissPlan);
    requestAnimationFrame(() => {
      if (returnFocus.current?.isConnected) returnFocus.current.focus({ preventScroll: true });
      else titleRef.current?.focus({ preventScroll: true });
    });
  }

  function toggleBranch(node: CleanupTreeNode) {
    if (locked || expired) return;
    const opening = !expanded.has(node.id);
    setExpanded((current) => {
      const next = new Set(current);
      if (opening) next.add(node.id); else next.delete(node.id);
      return next;
    });
    if (opening && !branches.has(node.id)) void run(() => onLoadChildren(node.id, 0));
  }

  function pagination(branch: CleanupTreeBranch | undefined) {
    if (!branch || (!branch.hasMore && branch.offset === 0)) return null;
    return <div className="cleanup-tree__pages" aria-label={t("검색 목록")}>
      <button type="button" className="text-button" disabled={locked || expired || branch.offset === 0}
        onClick={() => void run(() => onLoadChildren(branch.parentId, Math.max(0, branch.offset - branch.limit)))}>{t("이전")}</button>
      <span>{formatCount(branch.offset + 1)}–{formatCount(branch.offset + branch.childIds.length)} / {formatCount(branch.totalEntries)}</span>
      <button type="button" className="text-button" disabled={locked || expired || !branch.hasMore}
        onClick={() => void run(() => onLoadChildren(branch.parentId, branch.offset + branch.limit))}>{t("다음")}</button>
    </div>;
  }

  function renderBranch(parentId: string | null): React.ReactNode {
    const rows = grouped.get(parentId) ?? [];
    const branch = branches.get(parentId);
    return <>
      <ul className="cleanup-tree__list">
        {rows.map((node) => <li key={node.id}>
          <div className={`cleanup-tree__row is-${node.selectionState}${!node.eligible ? " is-blocked" : ""}`}>
            {node.isDirectory ? <button type="button" className="cleanup-tree__disclosure" aria-expanded={expanded.has(node.id)}
              aria-controls={`cleanup-children-${node.id}`} disabled={locked || expired}
              aria-label={t(expanded.has(node.id) ? "{{name}} 하위 항목 접기" : "{{name}} 하위 항목 펼치기", { name: node.name })}
              onClick={() => toggleBranch(node)}>{expanded.has(node.id) ? <ChevronDown size={18} aria-hidden="true" /> : <ChevronRight size={18} aria-hidden="true" />}</button>
              : <span className="cleanup-tree__disclosure-space" />}
            <TreeCheckbox node={node} disabled={locked || expired}
              label={t("{{name}} 휴지통 이동 대상으로 선택", { name: node.name })}
              onChange={(checked) => void run(() => onSelectionChange(checked ? [node.id] : [], checked ? [] : [node.id]))} />
            <span className="cleanup-tree__kind" aria-hidden="true">{node.isDirectory ? <Folder size={18} /> : <File size={18} />}</span>
            <div className="cleanup-tree__identity"><strong>{node.name}</strong><span dir="auto">{node.path}</span>
              {node.selectionState === "mixed" ? <small>{t("하위 항목 일부만 선택했습니다. 상위 폴더 자체는 이동하지 않습니다.")}</small> : null}
              {!node.eligible ? <small className="cleanup-tree__warning">{t("보호 항목")} · {node.blockedReason}</small> : null}
            </div>
            <span className="cleanup-tree__size">{node.logicalBytes === null ? t("용량 미측정") : formatBytes(node.logicalBytes)}</span>
            {onOpen && onReveal ? <div className="cleanup-tree__open"><FileOpenActions name={node.name} directory={node.isDirectory}
              disabled={locked || expired} onOpen={() => void run(() => onOpen(node.path))} onReveal={() => void run(() => onReveal(node.path))} /></div> : null}
          </div>
          {node.isDirectory ? <div id={`cleanup-children-${node.id}`} hidden={!expanded.has(node.id)} className="cleanup-tree__children"
            style={{ "--cleanup-indent": (depths.get(node.id) ?? 0) < 6 ? "24px" : "0px" } as CSSProperties}>
            {expanded.has(node.id) ? branches.has(node.id) ? renderBranch(node.id) : <div className="cleanup-tree__branch-state" role="status">
              {locked ? t("하위 목록을 불러오는 중…") : <button type="button" className="text-button" disabled={expired}
                onClick={() => void run(() => onLoadChildren(node.id, 0))}>{t("후보 다시 불러오기")}</button>}
            </div> : null}
          </div> : null}
        </li>)}
      </ul>
      {branch && (!branch.complete || branch.truncated || branch.unreadableEntries > 0) ? <p className="cleanup-tree__branch-note" role="note">
        <ShieldAlert size={16} aria-hidden="true" />{t("미검사 항목")} · {t("부분 결과입니다. 생략된 항목이나 읽지 못한 자료가 있을 수 있습니다.")}
      </p> : null}
      {branch?.complete && branch.totalEntries === 0 ? <p className="cleanup-tree__branch-note">{t("검사한 후보가 없습니다.")}</p> : null}
      {pagination(branch)}
    </>;
  }

  if (result) return <div ref={resultRef} tabIndex={-1} aria-label={t("작업 결과")}>
    <TrashResultPanel result={result} onRescan={() => void run(onRefresh)} />
  </div>;
  const rootIds = tree ? knownCleanupRootIds(tree) : [];
  const failure = localError ?? error;
  const plan = tree?.plan;
  return <>
    <section className="cleanup-tree" aria-labelledby="cleanup-tree-title" aria-busy={locked} inert={dialogOpen ? true : undefined}>
      <header className="cleanup-tree__header">
        <div><p className="eyebrow">{t("정리 후보")}</p><h2 id="cleanup-tree-title" ref={titleRef} tabIndex={-1}>{tree?.rootName ?? t("정리 후보 트리")}</h2>
          {tree ? <><p dir="auto">{tree.rootPath}</p><p>{t("검사 기준 · {{date}}", { date: formatDate(tree.capturedAtUnixMs) })}</p></> : null}
        </div>
        <button type="button" className="secondary-button" disabled={locked} onClick={() => void run(onRefresh)}><RefreshCw size={16} aria-hidden="true" />{t("후보 다시 불러오기")}</button>
      </header>
      {failure ? <p className="cleanup-tree__notice is-error" role="alert"><AlertTriangle size={18} aria-hidden="true" />{failure}</p> : null}
      {locked ? <div className="cleanup-tree__branch-state" role="status">{t("확인 중…")}
        {onCancel ? <button type="button" className="text-button" onClick={onCancel}>{t("작업 중단 요청")}</button> : null}
      </div> : null}
      {expired ? <p className="cleanup-tree__notice" role="status"><ShieldAlert size={18} aria-hidden="true" />{t("확인 시간이 만료됐습니다. 후보를 다시 불러오세요.")}</p> : null}
      {!tree ? <div className="empty-panel"><Folder size={28} aria-hidden="true" /><strong>{locked ? t("하위 목록을 불러오는 중…") : t("검사한 후보가 없습니다.")}</strong>
        <p>{t("용량만으로 삭제 안전성을 판단할 수 없습니다. 하위 항목·필요 여부·백업을 먼저 확인하세요.")}</p></div> : <>
        <div className="cleanup-tree__toolbar">
          <button type="button" className="secondary-button" disabled={locked || expired || !rootIds.length}
            onClick={() => void run(() => onSelectionChange(rootIds, []))}>{t("현재 확인된 후보 전체 선택")}</button>
          <button type="button" className="text-button" disabled={locked || expired || !tree.nodes.some((node) => node.selectionState !== "unchecked")}
            onClick={() => void run(() => onSelectionChange([], tree.nodes.filter((node) => node.selectionState !== "unchecked").map((node) => node.id)))}>{t("모두 해제")}</button>
        </div>
        <p className="cleanup-tree__scope">{t("현재 확인된 상위 후보를 선택합니다. 폴더 선택은 아직 펼치지 않은 하위 항목도 포함하며, 보호 항목은 최종 검토에서 차단합니다.")}</p>
        <div className="cleanup-tree__viewport">{renderBranch(null)}</div>
        {display.limited ? <p className="cleanup-tree__notice" role="status"><ShieldAlert size={18} aria-hidden="true" />{t("표시 상한에 도달했습니다. 다른 폴더를 접거나 페이지를 바꾸세요.")}</p> : null}
        <footer className="cleanup-tree__selection">
          <div><strong>{t("선택 {{count}}개 · 중복 제외 {{size}}", { count: formatCount(tree.selection.targetCount), size: formatBytes(tree.selection.knownLogicalBytes) })}</strong>
            {tree.selection.unknownTargets > 0 ? <span>{t("미측정 용량")} · {formatCount(tree.selection.unknownTargets)}</span> : null}
            <span>{t("선택 변경 시 기존 확인 계획은 무효화됩니다.")}</span>
            <span>{t("휴지통 이동 후 운영체제에서 복원을 시도할 수 있습니다. 즉시 여유 공간이 늘어나는 것은 아닙니다.")}</span>
          </div>
          <button type="button" className="primary-button" disabled={locked || expired || tree.selection.targetCount === 0}
            onClick={(event) => { returnFocus.current = event.currentTarget; void run(onPrepare); }}><Trash2 size={16} aria-hidden="true" />{t("선택한 항목 최종 검토")}</button>
        </footer>
      </>}
    </section>
    <SafetyActionDialog open={dialogOpen && Boolean(plan)} title={t("선택한 정리 후보를 휴지통으로 이동할까요?")}
      itemCount={plan?.entries.length ?? 0} logicalBytes={plan?.logicalBytes ?? 0}
      items={plan?.entries.map((entry) => ({ path: entry.path, logicalBytes: entry.logicalBytes ?? 0 }))}
      reviewCount={plan?.requiresNestedAck ? 1 : 0}
      reviewAcknowledgementLabel={t("선택한 폴더와 그 안의 모든 항목이 함께 휴지통으로 이동함을 확인했습니다.")}
      intro={t("이 목록만 휴지통으로 이동합니다. 실행 직전 다시 검사하며, 변경된 항목이 있으면 중단합니다.")}
      busy={locked} progress={progress} error={failure ?? (!planLive ? t("확인 시간이 만료됐습니다. 선택을 다시 검토하세요.") : null)}
      confirmDisabled={!planLive || planUsed}
      onConfirm={(acknowledged) => {
        if (!planLive || planUsed || locked || (plan?.requiresNestedAck && !acknowledged)) return;
        setPlanUsed(true); void run(() => onConfirm(acknowledged));
      }} onCancel={onCancel ?? (() => undefined)} onClose={closeDialog} />
  </>;
}
