import type { ViewId } from "../types";
import { useLanguage, type MessageKey } from "../i18n";

interface FileSectionNavProps {
  activeView: ViewId;
  onNavigate: (view: ViewId) => void;
}

const items: Array<{ id: ViewId; label: MessageKey }> = [
  { id: "files", label: "파일 이름 찾기" },
  { id: "documents", label: "문서 내용 찾기" },
];

export function FileSectionNav({ activeView, onNavigate }: FileSectionNavProps) {
  const { t } = useLanguage();

  return (
    <nav className="file-section-nav" aria-label={t("파일 관리 화면")}>
      {items.map((item) => {
        const current = activeView === item.id;
        return (
          <button
            type="button"
            className={current ? "is-active" : ""}
            aria-current={current ? "page" : undefined}
            key={item.id}
            onClick={() => onNavigate(item.id)}
          >
            {t(item.label)}
          </button>
        );
      })}
    </nav>
  );
}
