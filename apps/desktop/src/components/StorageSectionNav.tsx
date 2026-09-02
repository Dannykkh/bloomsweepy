import type { ViewId } from "../types";
import { useLanguage, type MessageKey } from "../i18n";

interface StorageSectionNavProps {
  activeView: ViewId;
  onNavigate: (view: ViewId) => void;
}

const items: Array<{ id: ViewId; label: MessageKey }> = [
  { id: "overview", label: "용량 지도" },
  { id: "large-files", label: "큰 파일" },
  { id: "duplicates", label: "중복 파일" },
  { id: "cleanup", label: "정리 후보" },
];

export function StorageSectionNav({
  activeView,
  onNavigate,
}: StorageSectionNavProps) {
  const { t } = useLanguage();
  return (
    <nav className="storage-section-nav" aria-label={t("용량 관리 화면")}>
      {items.map((item) => {
        const active = activeView === item.id;
        return (
          <button
            type="button"
            className={active ? "is-active" : ""}
            aria-current={active ? "page" : undefined}
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
