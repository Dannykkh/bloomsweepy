import { File, ShieldCheck } from "lucide-react";
import { FileInspectionStatus, FileOpenActions, useFileInspectionActions } from "./FileOpenActions";
import {
  fileParent,
  formatBytes,
  formatDate,
  formatDateTimeAttribute,
} from "../lib/format";
import type { FileEntry } from "../types";
import { useLanguage } from "../i18n";

interface FileTableProps {
  files: FileEntry[];
  emptyMessage: string;
  verified?: boolean;
  selectedPaths?: ReadonlySet<string>;
  onSelectionChange?: (file: FileEntry, selected: boolean) => void;
  isSelectionDisabled?: (file: FileEntry) => boolean;
  selectionDisabled?: boolean;
}

export function FileTable({
  files,
  emptyMessage,
  verified = false,
  selectedPaths,
  onSelectionChange,
  isSelectionDisabled,
  selectionDisabled = false,
}: FileTableProps) {
  const { t } = useLanguage();
  const inspection = useFileInspectionActions({ scopeKey: files });
  const selectable = Boolean(selectedPaths && onSelectionChange);

  if (files.length === 0) {
    return <div className="table-empty">{emptyMessage}</div>;
  }

  return (
    <div className="file-table-shell">
      <FileInspectionStatus message={inspection.message} error={inspection.error} />
      <div
        className={`file-table ${selectable ? "has-selection" : ""}`}
        role="table"
        aria-label={t("파일 분석 결과")}
      >
        <div className="file-table__head" role="row">
          {selectable ? <span role="columnheader">{t("선택")}</span> : null}
          <span role="columnheader">{t("파일")}</span>
          <span role="columnheader">{t("수정")}</span>
          <span role="columnheader">{t("크기")}</span>
          <span role="columnheader" className="file-table__inspection-heading">{t("작업")}</span>
        </div>
        {files.map((file) => {
          const selected = selectedPaths?.has(file.path) ?? false;
          const selectionUnavailable =
            selectionDisabled || Boolean(isSelectionDisabled?.(file));
          return (
            <div
            className={`file-table__row is-openable ${selected ? "is-selected" : ""}`}
            role="row"
            tabIndex={0}
            aria-selected={selectable ? selected : undefined}
            aria-label={t("{{name}}, 더블클릭하거나 Enter 키를 눌러 확인", { name: file.name })}
            title={t("더블클릭하여 기본 앱으로 열기")}
            key={file.path}
            onDoubleClick={() => void inspection.run(file.path, file.name)}
            onKeyDown={(event) => {
              if (event.target === event.currentTarget && event.key === "Enter" && !event.repeat) {
                event.preventDefault();
                void inspection.run(file.path, file.name);
              }
            }}
          >
            {selectable ? (
              <label
                className="file-selection"
                title={
                  selectionUnavailable && !selected
                    ? t("이 그룹에는 보관할 파일을 하나 이상 남겨야 합니다")
                    : t("휴지통으로 이동할 파일 선택")
                }
                onClick={(event) => event.stopPropagation()}
                onDoubleClick={(event) => event.stopPropagation()}
              >
                <input
                  type="checkbox"
                  checked={selected}
                  disabled={selectionUnavailable}
                  aria-label={t("{{name}} 휴지통 이동 대상으로 선택", { name: file.name })}
                  onChange={(event) => onSelectionChange?.(file, event.currentTarget.checked)}
                />
              </label>
            ) : null}
            <div className="file-identity" role="cell">
              <span className="file-identity__icon" aria-hidden="true">
                {verified ? <ShieldCheck size={17} /> : <File size={17} />}
              </span>
              <span className="file-identity__copy">
                <strong title={file.name}>{file.name}</strong>
                <span title={fileParent(file.path)}>{fileParent(file.path)}</span>
              </span>
            </div>
            <time role="cell" dateTime={formatDateTimeAttribute(file.modifiedAtUnixMs)}>
              {formatDate(file.modifiedAtUnixMs)}
            </time>
            <strong className="file-size" role="cell">
              {formatBytes(file.logicalBytes)}
            </strong>
            <div role="cell" className="file-table__inspection-cell">
              <FileOpenActions name={file.name} disabled={inspection.busy}
                onOpen={() => void inspection.run(file.path, file.name)}
                onReveal={() => void inspection.run(file.path, file.name, "reveal")} />
            </div>
          </div>
          );
        })}
      </div>
      <p className="file-table__hint">
        {t("더블클릭하면 기본 앱으로 엽니다. 실행 파일과 스크립트는 안전을 위해 폴더에서만 표시합니다.")}
      </p>
    </div>
  );
}
