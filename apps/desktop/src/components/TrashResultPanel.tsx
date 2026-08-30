import { AlertTriangle, RotateCcw, ShieldCheck } from "lucide-react";
import { formatBytes, formatCount } from "../lib/format";
import type { TrashOperationResult } from "../types";

interface TrashResultPanelProps {
  result: TrashOperationResult;
  onRescan: () => void;
}

export function TrashResultPanel({ result, onRescan }: TrashResultPanelProps) {
  const failed = result.items.filter((item) => item.status === "failed");
  const skipped = result.items.filter((item) => item.status === "skipped");
  const hasWarning = failed.length > 0 || skipped.length > 0 || !result.journalComplete;

  return (
    <section className={`trash-result ${hasWarning ? "has-warning" : ""}`} aria-live="polite">
      <div className="trash-result__heading">
        <span aria-hidden="true">
          {hasWarning ? <AlertTriangle size={22} /> : <ShieldCheck size={22} />}
        </span>
        <div>
          <p className="eyebrow">ACTION RESULT</p>
          <h2>
            {result.cancelled
              ? "휴지통 이동을 중단했습니다"
              : hasWarning
                ? "일부 항목만 처리했습니다"
                : "휴지통 이동을 완료했습니다"}
          </h2>
          <p>이 결과는 삭제된 용량이 아니라 휴지통으로 이동한 논리 용량입니다.</p>
        </div>
      </div>

      <div className="trash-result__metrics">
        <div><span>이동</span><strong>{formatCount(result.movedCount)}개</strong></div>
        <div><span>이동한 용량</span><strong>{formatBytes(result.movedBytes)}</strong></div>
        <div><span>실패</span><strong>{formatCount(failed.length)}개</strong></div>
        <div><span>건너뜀</span><strong>{formatCount(skipped.length)}개</strong></div>
      </div>

      {failed.length > 0 ? (
        <div className="trash-result__failures">
          <strong>확인이 필요한 항목</strong>
          {failed.slice(0, 8).map((item) => (
            <p key={item.path}><span title={item.path}>{item.path}</span><small>{item.message}</small></p>
          ))}
        </div>
      ) : null}

      <div className="trash-result__journal">
        <span>작업 기록</span>
        <code title={result.journalPath}>{result.journalPath}</code>
        {!result.journalComplete ? <small>마지막 기록 동기화를 완료하지 못했습니다.</small> : null}
      </div>

      <div className="trash-result__actions">
        <p>파일 상태가 바뀌었으므로 기존 분석 결과는 폐기했습니다. 계속하려면 다시 스캔하세요.</p>
        <button className="primary-button" type="button" onClick={onRescan}>
          <RotateCcw size={16} aria-hidden="true" />
          다시 스캔
        </button>
      </div>
    </section>
  );
}
