import { Trash2 } from "lucide-react";
import { useLanguage } from "../i18n";
import { applicationTrashQuestion } from "../lib/assistantConfirmation";
import type { AppToolResult } from "../types";
import "./AssistantAppToolCard.css";

export function AssistantApplicationConfirmation({ result, busy, onDecision }: {
  result: AppToolResult; busy: boolean; onDecision: (confirmed: boolean) => void;
}) {
  const { t } = useLanguage();
  const question = applicationTrashQuestion(result);
  if (!question) return null;
  const { plan, bundle } = question;
  return <section className="assistant-app-tool-card" aria-label={t("휴지통으로 이동할까요?")} aria-busy={busy}>
    <header><h3>{plan.displayName}</h3><span>{t("확인 대기 · 아직 이동하지 않았습니다.")}</span></header>
    <p>{t("휴지통으로 이동할까요?")}</p>
    {bundle ? <code className="applications-review-path">{plan.path}</code>
      : <ul className="assistant-empty-review__list">{plan.relatedData.map(item => <li key={item.id}><code>{item.path}</code></li>)}</ul>}
    <p>{t(bundle
      ? "앱 본체만 이동합니다. 설정과 문서는 남깁니다. 전용 제거 프로그램이 필요하면 아니오를 선택하세요."
      : "표시한 관련 데이터만 이동합니다. 환경설정이 초기화될 수 있습니다.")}</p>
    {plan.warnings.length ? <details><summary>{t("주의사항")}</summary><ul>{plan.warnings.map((warning, index) => <li key={index}>{warning}</li>)}</ul></details> : null}
    <p>{t("실행 직전 대상 변경 여부를 다시 검사합니다. 휴지통에서 복원을 시도할 수 있습니다.")}</p>
    {busy ? <p role="status">{t("대상을 다시 확인하고 처리 중…")}</p> : null}
    <footer className="assistant-trash-question-actions">
      <button type="button" className="secondary-button" disabled={busy} onClick={() => onDecision(false)}>{t("아니오")}</button>
      <button type="button" className="trash-confirm-button" disabled={busy} onClick={() => onDecision(true)}><Trash2 size={16} aria-hidden="true" />{t(bundle ? "예, 앱 본체만 휴지통으로 이동" : "예, 선택 데이터만 휴지통으로 이동")}</button>
    </footer>
  </section>;
}
