import type { ViewId } from "../types";

interface StorageSectionNavProps {
  activeView: ViewId;
  onNavigate: (view: ViewId) => void;
}

const items: Array<{ id: ViewId; label: string }> = [
  { id: "overview", label: "용량 지도" },
  { id: "large-files", label: "큰 파일" },
  { id: "duplicates", label: "중복 파일" },
  { id: "cleanup", label: "정리 후보" },
];

export function StorageSectionNav({
  activeView,
  onNavigate,
}: StorageSectionNavProps) {
  return (
    <nav className="storage-section-nav" aria-label="용량 관리 화면">
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
            {item.label}
          </button>
        );
      })}
    </nav>
  );
}
