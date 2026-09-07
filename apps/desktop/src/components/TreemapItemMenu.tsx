import { ExternalLink, FolderOpen, MapPin, Trash2 } from "lucide-react";
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { useLanguage } from "../i18n";
import type { DirectoryNode } from "../types";

export interface TreemapMenuTarget {
  node: DirectoryNode;
  x: number;
  y: number;
  trigger: HTMLElement;
}

export function TreemapItemMenu({ target, canTrash, onOpen, onInspect, onReveal, onTrash, onClose }: {
  target: TreemapMenuTarget;
  canTrash: boolean;
  onOpen: () => void;
  onInspect: () => void;
  onReveal: () => void;
  onTrash: () => void;
  onClose: () => void;
}) {
  const { t } = useLanguage();
  const menuRef = useRef<HTMLDivElement>(null);
  const [position, setPosition] = useState({ x: target.x, y: target.y });
  useLayoutEffect(() => {
    const rect = menuRef.current?.getBoundingClientRect();
    if (!rect) return;
    setPosition({
      x: Math.max(8, Math.min(target.x, window.innerWidth - rect.width - 8)),
      y: Math.max(8, Math.min(target.y, window.innerHeight - rect.height - 8)),
    });
    menuRef.current?.querySelector<HTMLButtonElement>('[role="menuitem"]')?.focus();
  }, [target]);

  useEffect(() => {
    const dismissOutside = (event: PointerEvent) => {
      if (!menuRef.current?.contains(event.target as Node)) onClose();
    };
    const dismissScroll = (event: Event) => {
      if (event.target instanceof Node && menuRef.current?.contains(event.target)) return;
      onClose();
    };
    document.addEventListener("pointerdown", dismissOutside);
    document.addEventListener("scroll", dismissScroll, true);
    window.addEventListener("resize", onClose);
    return () => {
      document.removeEventListener("pointerdown", dismissOutside);
      document.removeEventListener("scroll", dismissScroll, true);
      window.removeEventListener("resize", onClose);
    };
  }, [onClose]);

  return createPortal(
    <div ref={menuRef} className="treemap-item-menu" role="menu"
      aria-label={t("{{name}} 작업", { name: target.node.name })}
      style={{ left: position.x, top: position.y }}
      onKeyDown={(event) => {
        if (event.key === "Escape" || event.key === "Tab") {
          event.preventDefault();
          event.stopPropagation();
          onClose();
          return;
        }
        const items = [...(menuRef.current?.querySelectorAll<HTMLButtonElement>('[role="menuitem"]') ?? [])];
        const current = items.indexOf(document.activeElement as HTMLButtonElement);
        const next = event.key === "Home" ? 0 : event.key === "End" ? items.length - 1
          : event.key === "ArrowDown" ? (current + 1) % items.length
          : event.key === "ArrowUp" ? (current - 1 + items.length) % items.length : null;
        if (next !== null) {
          event.preventDefault();
          items[next]?.focus();
        }
      }}>
      <p title={target.node.path}>{target.node.name}</p>
      {target.node.isDirectory ? <button type="button" role="menuitem" tabIndex={-1} onClick={onOpen}>
        <FolderOpen size={17} aria-hidden="true" />{t("하위 폴더 탐색")}
      </button> : null}
      <button type="button" role="menuitem" tabIndex={-1} onClick={onInspect}>
        {target.node.isDirectory ? <FolderOpen size={17} aria-hidden="true" /> : <ExternalLink size={17} aria-hidden="true" />}
        {target.node.isDirectory ? t("폴더 열기") : t("열기")}
      </button>
      <button type="button" role="menuitem" tabIndex={-1} onClick={onReveal}>
        <MapPin size={17} aria-hidden="true" />{t("위치 표시")}
      </button>
      {canTrash ? <button className="treemap-item-menu__danger"
        type="button" role="menuitem" tabIndex={-1} onClick={onTrash}>
        <Trash2 size={17} aria-hidden="true" />{t("휴지통 이동 검토")}
      </button> : null}
    </div>, document.body,
  );
}
