import { ExternalLink, FolderOpen, MapPin } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useLanguage } from "../i18n";
import { createInspectionGate, inspectFile, revealFile, type FileInspectionOutcome } from "../lib/fileInspectionBridge";
import "./FileOpenActions.css";

type InspectionAction = "open" | "reveal";

export function useFileInspectionActions({
  inspect = inspectFile,
  reveal = revealFile,
  scopeKey,
}: {
  inspect?: (path: string) => Promise<FileInspectionOutcome>;
  reveal?: (path: string) => Promise<void>;
  scopeKey?: unknown;
} = {}) {
  const { t } = useLanguage();
  const gate = useRef(createInspectionGate());
  const mounted = useRef(true);
  const scopeRevision = useRef(0);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    mounted.current = true;
    return () => { mounted.current = false; };
  }, []);
  useEffect(() => {
    scopeRevision.current += 1;
    setMessage(null);
    setError(null);
  }, [scopeKey]);

  async function run(path: string, name: string, action: InspectionAction = "open") {
    await gate.current.run(async () => {
      const revision = scopeRevision.current;
      setBusy(true);
      setMessage(null);
      setError(null);
      try {
        const outcome = action === "reveal"
          ? (await reveal(path), "revealed")
          : await inspect(path);
        if (mounted.current && revision === scopeRevision.current) setMessage(action === "reveal"
          ? t("{{name}} 위치를 파일 탐색기에서 표시했습니다.", { name })
          : outcome === "opened"
            ? t("{{name}} 열기를 운영체제에 요청했습니다.", { name })
            : t("{{name}}은 직접 열지 않고 파일 탐색기에서 위치만 표시했습니다.", { name }));
      } catch (reason) {
        if (mounted.current && revision === scopeRevision.current) setError(t("{{name}} 열기 요청을 완료하지 못했습니다: {{detail}}", {
          name,
          detail: reason instanceof Error ? reason.message : String(reason),
        }));
      } finally {
        if (mounted.current) setBusy(false);
      }
    });
  }

  return { run, busy, message, error };
}

export function FileInspectionStatus({ message, error }: { message: string | null; error: string | null }) {
  if (error) return <p className="file-open-status is-error" role="alert">{error}</p>;
  if (message) return <p className="file-open-status" role="status">{message}</p>;
  return null;
}

export function FileOpenActions({ name, directory = false, disabled = false, onOpen, onReveal }: {
  name: string;
  directory?: boolean;
  disabled?: boolean;
  onOpen: () => void;
  onReveal: () => void;
}) {
  const { t } = useLanguage();
  return <div className="file-open-actions"
    onClick={(event) => event.stopPropagation()}
    onDoubleClick={(event) => event.stopPropagation()}
    onKeyDown={(event) => {
      event.stopPropagation();
      if (event.repeat && (event.key === "Enter" || event.key === " ")) event.preventDefault();
    }}>
    <button type="button" disabled={disabled}
      aria-label={t("{{name}} 열기", { name })}
      title={t("문서·미디어는 기본 앱으로, 일반 폴더는 파일 탐색기로 엽니다. 프로그램과 링크는 위치만 표시합니다.")}
      onClick={(event) => { if (event.detail <= 1) onOpen(); }}>
      {directory ? <FolderOpen size={16} aria-hidden="true" /> : <ExternalLink size={16} aria-hidden="true" />}
      {directory ? t("폴더 열기") : t("열기")}
    </button>
    <button type="button" disabled={disabled}
      aria-label={t("{{name}} 위치 표시", { name })}
      onClick={(event) => { if (event.detail <= 1) onReveal(); }}>
      <MapPin size={16} aria-hidden="true" />{t("위치 표시")}
    </button>
  </div>;
}
