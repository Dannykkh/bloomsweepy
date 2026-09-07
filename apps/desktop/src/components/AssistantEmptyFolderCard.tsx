import { FolderOpen, ShieldCheck, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";
import { useLanguage } from "../i18n";
import type { AssistantEmptyWorkspace, TrashOperationResult } from "../types";
import { formatCount } from "../lib/format";

interface Props {
  workspace: AssistantEmptyWorkspace;
  busy: boolean;
  onSelect: (ids: string[]) => void;
  onPrepare: () => void;
  onConfirm: () => void;
}

export function AssistantEmptyFolderCard({ workspace, busy, onSelect, onPrepare, onConfirm }: Props) {
  const { t } = useLanguage();
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    if (!workspace.plan) return;
    setNow(Date.now());
    const timer = window.setInterval(() => setNow(Date.now()), 1_000);
    return () => window.clearInterval(timer);
  }, [workspace.plan?.id]);
  const plan = workspace.plan;
  const expired = Boolean(plan && now >= plan.expiresAtUnixMs);
  const selected = new Set(workspace.selectedIds);
  const rows = plan ? workspace.candidates.filter((candidate) => plan.candidateIds.includes(candidate.id)) : workspace.candidates;

  return (
    <section className="assistant-empty-review" aria-label={t("빈 폴더 검토")}>
      <header>
        {plan ? <ShieldCheck size={22} aria-hidden="true" /> : <FolderOpen size={22} aria-hidden="true" />}
        <div>
          <strong>{plan ? t("휴지통 이동 최종 확인") : t("빈 폴더 검토")}</strong>
          <p>{t("발견 {{found}}개 · 검토 가능 {{available}}개 · 선택 {{selected}}개", {
            found: formatCount(workspace.totalFound), available: formatCount(workspace.candidates.length), selected: formatCount(selected.size),
          })}</p>
        </div>
      </header>
      <p>{t("빈 폴더도 앱이나 프로젝트에 필요할 수 있습니다. 아래 경로를 확인하고 보관할 폴더는 제외하세요.")}</p>
      {workspace.omittedCount > 0 ? <p role="status">{t("보호·변경·검사 상한으로 {{count}}개는 검토 목록에서 제외됐습니다. 전체를 삭제하는 작업이 아닙니다.", { count: formatCount(workspace.omittedCount) })}</p> : null}
      {workspace.summary.unreadableEntries > 0 ? <p role="status">{t("읽지 못한 항목이 있어 검사 결과가 완전하지 않을 수 있습니다.")}</p> : null}
      {!plan && rows.length > 0 ? <div className="assistant-empty-review__selection">
        <button type="button" className="text-button" disabled={busy} onClick={() => onSelect(workspace.candidates.map((candidate) => candidate.id))}>{t("모두 선택")}</button>
        <button type="button" className="text-button" disabled={busy} onClick={() => onSelect([])}>{t("선택 해제")}</button>
      </div> : null}
      <ul className="assistant-empty-review__list">
        {rows.map((candidate) => <li key={candidate.id}>
          <label>
            {!plan ? <input type="checkbox" checked={selected.has(candidate.id)} disabled={busy}
              onChange={(event) => onSelect(event.currentTarget.checked
                ? [...workspace.selectedIds, candidate.id] : workspace.selectedIds.filter((id) => id !== candidate.id))} /> : <FolderOpen size={16} aria-hidden="true" />}
            <span><strong>{candidate.number}. {candidate.name}</strong><small dir="auto">{candidate.path}</small></span>
          </label>
        </li>)}
      </ul>
      {rows.length === 0 ? <p>{t("검토할 빈 폴더가 없습니다.")}</p> : null}
      {plan ? <p role="status">{expired ? t("확인 시간이 만료됐습니다. 선택을 다시 검토하세요.") : t("이 목록만 휴지통으로 이동합니다. 실행 직전 다시 검사하며, 변경된 항목이 있으면 중단합니다.")}</p> : null}
      <footer>
        <small>{t("파일 내용은 읽지 않습니다. 빈 폴더 정리는 큰 용량 확보를 보장하지 않습니다.")}</small>
        {plan ? <>
          <button type="button" className="secondary-button" disabled={busy} onClick={() => onSelect(workspace.selectedIds)}>{t("선택 다시 검토")}</button>
          <button type="button" className="trash-confirm-button" disabled={busy || expired} onClick={onConfirm}>
            <Trash2 size={16} aria-hidden="true" />{t("확인한 {{count}}개 휴지통으로 이동", { count: formatCount(plan.candidateIds.length) })}
          </button>
        </> : <button type="button" className="secondary-button" disabled={busy || selected.size === 0} onClick={onPrepare}>{t("선택한 폴더 최종 검토")}</button>}
      </footer>
    </section>
  );
}

export function AssistantTrashResultCard({ result }: { result: TrashOperationResult }) {
  const { t } = useLanguage();
  return <section className="assistant-empty-review" aria-label={t("휴지통 이동 결과")}>
    <strong>{t("휴지통 이동 결과")}</strong>
    <p>{t("요청 {{requested}}개 중 {{moved}}개를 휴지통으로 이동했습니다.", { requested: result.requestedCount, moved: result.movedCount })}</p>
    {result.cancelled ? <p>{t("사용자 취소")}</p> : null}
    {!result.journalComplete ? <p role="alert">{t("작업 기록 저장이 완전하지 않습니다. 실제 휴지통과 작업 기록을 확인하세요.")}</p> : null}
    <details><summary>{t("항목별 결과 보기")}</summary>
      <ul className="assistant-empty-review__list">{result.items.map((item) => <li key={item.path}>
        <strong>{item.status === "moved" ? t("이동 완료") : item.status === "failed" ? t("실패") : t("건너뜀")}</strong>
        <small dir="auto">{item.path}</small>{item.message ? <p>{item.message}</p> : null}
      </li>)}</ul>
    </details>
    <small>{t("필요하면 운영체제 휴지통에서 복원을 시도할 수 있습니다. 추가 정리 전 다시 검사하세요.")}</small>
  </section>;
}
