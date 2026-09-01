import { File, ShieldCheck } from "lucide-react";
import { useState } from "react";
import { inspectFile } from "../lib/bridge";
import {
  fileParent,
  formatBytes,
  formatDate,
  formatDateTimeAttribute,
} from "../lib/format";
import type { FileEntry } from "../types";

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
  const [inspectionMessage, setInspectionMessage] = useState<string | null>(null);
  const selectable = Boolean(selectedPaths && onSelectionChange);

  if (files.length === 0) {
    return <div className="table-empty">{emptyMessage}</div>;
  }

  async function openForInspection(file: FileEntry) {
    try {
      const outcome = await inspectFile(file.path);
      setInspectionMessage(
        outcome === "opened"
          ? `${file.name} 파일을 기본 앱으로 열었습니다.`
          : `${file.name}은 직접 열도록 허용한 문서·미디어 형식이 아니라 폴더에서 위치만 표시했습니다.`,
      );
    } catch (reason) {
      const message = reason instanceof Error ? reason.message : String(reason);
      setInspectionMessage(`${file.name} 파일을 열지 못했습니다: ${message}`);
    }
  }

  return (
    <div className="file-table-shell">
      <div
        className={`file-table ${selectable ? "has-selection" : ""}`}
        role="table"
        aria-label="파일 분석 결과"
      >
        <div className="file-table__head" role="row">
          {selectable ? <span role="columnheader">선택</span> : null}
          <span role="columnheader">파일</span>
          <span role="columnheader">수정</span>
          <span role="columnheader">크기</span>
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
            aria-label={`${file.name}, 더블클릭하거나 Enter 키를 눌러 확인`}
            title="더블클릭하여 기본 앱으로 열기"
            key={file.path}
            onDoubleClick={() => void openForInspection(file)}
            onKeyDown={(event) => {
              if (event.target === event.currentTarget && event.key === "Enter") {
                void openForInspection(file);
              }
            }}
          >
            {selectable ? (
              <label
                className="file-selection"
                title={
                  selectionUnavailable && !selected
                    ? "이 그룹에는 보관할 파일을 하나 이상 남겨야 합니다"
                    : "휴지통으로 이동할 파일 선택"
                }
                onClick={(event) => event.stopPropagation()}
                onDoubleClick={(event) => event.stopPropagation()}
              >
                <input
                  type="checkbox"
                  checked={selected}
                  disabled={selectionUnavailable}
                  aria-label={`${file.name} 휴지통 이동 대상으로 선택`}
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
          </div>
          );
        })}
      </div>
      <p className="file-table__hint">더블클릭하면 기본 앱으로 엽니다. 실행 파일과 스크립트는 안전을 위해 폴더에서만 표시합니다.</p>
      {inspectionMessage ? (
        <p className="file-table__status" role="status" aria-live="polite">
          {inspectionMessage}
        </p>
      ) : null}
    </div>
  );
}
