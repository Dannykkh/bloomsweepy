import { AlertTriangle, RotateCcw, ShieldCheck } from "lucide-react";
import { formatBytes, formatCount } from "../lib/format";
import type { TrashOperationResult } from "../types";
import { useLanguage } from "../i18n";

interface TrashResultPanelProps {
  result: TrashOperationResult;
  onRescan: () => void;
}

export function TrashResultPanel({ result, onRescan }: TrashResultPanelProps) {
  const { t } = useLanguage();
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
          <p className="eyebrow">{t("작업 결과")}</p>
          <h2>
            {result.cancelled
              ? t("휴지통 이동을 중단했습니다")
              : hasWarning
                ? t("일부 항목만 처리했습니다")
                : t("휴지통 이동을 완료했습니다")}
          </h2>
          <p>{t("실제로 확보된 공간이 아니라 휴지통으로 옮긴 파일 크기의 합계입니다.")}</p>
        </div>
      </div>

      <div className="trash-result__metrics">
        <div><span>{t("이동")}</span><strong>{t("{{count}}개", { count: formatCount(result.movedCount) })}</strong></div>
        <div><span>{t("이동한 용량")}</span><strong>{formatBytes(result.movedBytes)}</strong></div>
        <div><span>{t("실패")}</span><strong>{t("{{count}}개", { count: formatCount(failed.length) })}</strong></div>
        <div><span>{t("건너뜀")}</span><strong>{t("{{count}}개", { count: formatCount(skipped.length) })}</strong></div>
      </div>

      {failed.length > 0 ? (
        <div className="trash-result__failures">
          <strong>{t("확인이 필요한 항목")}</strong>
          {failed.slice(0, 8).map((item) => (
            <p key={item.path}><span title={item.path}>{item.path}</span><small>{item.message}</small></p>
          ))}
        </div>
      ) : null}

      <div className="trash-result__journal">
        <span>{t("작업 기록")}</span>
        <code title={result.journalPath}>{result.journalPath}</code>
        {!result.journalComplete ? <small>{t("마지막 기록 동기화를 완료하지 못했습니다.")}</small> : null}
      </div>

      <div className="trash-result__actions">
        <p>{t("파일 상태가 바뀌었으므로 기존 분석 결과는 폐기했습니다. 계속하려면 다시 스캔하세요.")}</p>
        <button className="primary-button" type="button" onClick={onRescan}>
          <RotateCcw size={16} aria-hidden="true" />
          {t("다시 스캔")}
        </button>
      </div>
    </section>
  );
}
