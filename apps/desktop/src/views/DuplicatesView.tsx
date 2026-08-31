import {
  AlertTriangle,
  Camera,
  ChevronDown,
  ChevronRight,
  Copy,
  FolderTree,
  ShieldCheck,
  Trash2,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { FileTable } from "../components/FileTable";
import { SafetyActionDialog } from "../components/SafetyActionDialog";
import { TrashResultPanel } from "../components/TrashResultPanel";
import { fileParent, formatBytes, formatCount } from "../lib/format";
import type {
  DuplicateTrashRequest,
  FileEntry,
  ScanReport,
  TrashOperationResult,
  TrashProgress,
} from "../types";

interface DuplicatesViewProps {
  report: ScanReport | null;
  scanning: boolean;
  actionRunning: boolean;
  actionProgress: TrashProgress | null;
  actionResult: TrashOperationResult | null;
  actionError: string | null;
  onStartScan: () => void;
  onMoveToTrash: (request: DuplicateTrashRequest) => Promise<TrashOperationResult>;
  onCancelAction: () => void;
}

export function DuplicatesView({
  report,
  scanning,
  actionRunning,
  actionProgress,
  actionResult,
  actionError,
  onStartScan,
  onMoveToTrash,
  onCancelAction,
}: DuplicatesViewProps) {
  const [expandedHash, setExpandedHash] = useState<string | null>(null);
  const [filter, setFilter] = useState<"all" | "photos">("all");
  const [selectedPaths, setSelectedPaths] = useState<Set<string>>(new Set());
  const [dialogOpen, setDialogOpen] = useState(false);
  const [dialogError, setDialogError] = useState<string | null>(null);

  useEffect(() => {
    setSelectedPaths(new Set());
    setDialogOpen(false);
    setDialogError(null);
  }, [report?.completedAtUnixMs]);

  const photoGroups = useMemo(
    () => report?.duplicateGroups.filter((group) => group.files.some((file) => isPhoto(file.name))) ?? [],
    [report],
  );
  const visibleGroups = filter === "photos" ? photoGroups : report?.duplicateGroups ?? [];
  const crossFolderGroups = useMemo(
    () => visibleGroups.filter((group) => parentDirectories(group.files.map((file) => file.path)).length > 1),
    [visibleGroups],
  );
  const selectedFiles = useMemo(() => {
    if (!report) return [];
    const files: FileEntry[] = [];
    for (const group of report.duplicateGroups) {
      for (const file of group.files) {
        if (selectedPaths.has(file.path)) files.push(file);
      }
    }
    return files;
  }, [report, selectedPaths]);
  const selectedBytes = selectedFiles.reduce(
    (total, file) => total + file.logicalBytes,
    0,
  );

  function updateSelection(file: FileEntry, selected: boolean) {
    setSelectedPaths((current) => {
      const next = new Set(current);
      if (selected) next.add(file.path);
      else next.delete(file.path);
      return next;
    });
  }

  async function confirmTrashMove() {
    if (!report || selectedPaths.size === 0) return;
    const request: DuplicateTrashRequest = {
      groups: report.duplicateGroups
        .map((group) => ({
          contentHash: group.contentHash,
          paths: group.files
            .filter((file) => selectedPaths.has(file.path))
            .map((file) => file.path),
        }))
        .filter((group) => group.paths.length > 0),
    };
    setDialogError(null);
    try {
      await onMoveToTrash(request);
      setDialogOpen(false);
    } catch (reason) {
      setDialogError(normalizeError(reason));
    }
  }

  if (!report) {
    if (actionResult) {
      return <TrashResultPanel result={actionResult} onRescan={onStartScan} />;
    }
    return (
      <div className="view-stack">
        {actionError ? (
          <div className="notice-panel trash-action-error" role="alert">
            <AlertTriangle size={18} aria-hidden="true" />
            <p>{actionError} 기존 결과는 안전을 위해 폐기했습니다. 다시 스캔하세요.</p>
          </div>
        ) : null}
        <div className="empty-panel empty-panel--page">
          <Copy size={28} aria-hidden="true" />
          <strong>중복 파일을 확인하려면 먼저 스캔하세요</strong>
          <p>파일 크기로 후보를 줄인 뒤 파일 내용을 처음부터 끝까지 비교합니다.</p>
          <button className="primary-button" type="button" disabled={scanning} onClick={onStartScan}>
            <Copy size={17} aria-hidden="true" />
            스캔 시작
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="view-stack">
      <section className="verification-strip">
        <ShieldCheck size={22} aria-hidden="true" />
        <div>
          <strong>전체 내용 검증 완료</strong>
          <p>
            {formatCount(report.duplicateGroups.length)}개 그룹에서 {formatBytes(report.duplicateWasteBytes)}를 중복으로 확인했습니다.
          </p>
        </div>
      </section>

      {report.hardLinkIdentityLimitReached ? (
        <div className="notice-panel duplicate-folder-notice" role="status">
          <AlertTriangle size={18} aria-hidden="true" />
          <p>하드링크 식별자 안전 상한에 도달했습니다. 이후 하드링크 파일은 중복 분석에서 제외했습니다.</p>
        </div>
      ) : null}

      <section className="duplicate-toolbar" aria-label="중복 결과 필터">
        <div className="segmented-control" role="group" aria-label="중복 파일 종류">
          <button
            type="button"
            className={filter === "all" ? "is-active" : ""}
            aria-pressed={filter === "all"}
            onClick={() => setFilter("all")}
          >
            <Copy size={15} aria-hidden="true" />
            모든 파일 {formatCount(report.duplicateGroups.length)}
          </button>
          <button
            type="button"
            className={filter === "photos" ? "is-active" : ""}
            aria-pressed={filter === "photos"}
            onClick={() => setFilter("photos")}
          >
            <Camera size={15} aria-hidden="true" />
            동일 사진 {formatCount(photoGroups.length)}
          </button>
        </div>
        <span>사진 보기는 내용이 완전히 같은 이미지 파일만 포함합니다.</span>
      </section>

      {selectedPaths.size > 0 ? (
        <section className="trash-action-bar" aria-label="휴지통 이동 선택 요약">
          <div>
            <span>선택</span>
            <strong>{formatCount(selectedPaths.size)}개 · {formatBytes(selectedBytes)}</strong>
            <small>각 그룹의 보관본 한 개는 선택할 수 없습니다.</small>
          </div>
          <button
            className="trash-action-button"
            type="button"
            disabled={scanning}
            onClick={() => {
              setDialogError(null);
              setDialogOpen(true);
            }}
          >
            <Trash2 size={16} aria-hidden="true" />
            휴지통으로 이동
          </button>
        </section>
      ) : null}

      {crossFolderGroups.length > 0 ? (
        <div className="notice-panel duplicate-folder-notice" role="status">
          <FolderTree size={18} aria-hidden="true" />
          <p>서로 다른 폴더에 흩어진 중복이 {formatCount(crossFolderGroups.length)}개 그룹 있습니다. 각 위치를 비교한 뒤 이동 대상을 선택하세요.</p>
        </div>
      ) : null}

      {visibleGroups.length === 0 ? (
        <div className="empty-panel empty-panel--page">
          <ShieldCheck size={28} aria-hidden="true" />
          <strong>{filter === "photos" ? "내용이 같은 사진이 없습니다" : "검증된 중복 파일이 없습니다"}</strong>
          <p>{filter === "photos" ? "비슷하게 찍힌 사진은 아직 포함하지 않고, 파일 내용이 완전히 같은 사진만 보여줍니다." : "크기만 같고 내용이 다른 파일은 중복으로 표시하지 않았습니다."}</p>
        </div>
      ) : (
        <section className="duplicate-list" aria-label="중복 파일 그룹">
          {visibleGroups.map((group, index) => {
            const expanded = expandedHash === group.contentHash;
            const directories = parentDirectories(group.files.map((file) => file.path));
            const groupSelectionCount = group.files.filter((file) => selectedPaths.has(file.path)).length;
            return (
              <article className="duplicate-group" key={group.contentHash}>
                <button
                  className="duplicate-group__summary"
                  type="button"
                  aria-expanded={expanded}
                  onClick={() => setExpandedHash(expanded ? null : group.contentHash)}
                >
                  <span className="duplicate-group__index">{String(index + 1).padStart(2, "0")}</span>
                  <span className="duplicate-group__copy">
                    <strong>{group.files[0]?.name ?? "중복 그룹"}</strong>
                    <small>
                      내용 확인 번호 {group.contentHash.slice(0, 16)} · 전체 내용 비교 완료
                      {directories.length > 1 ? ` · 서로 다른 폴더 ${directories.length}곳` : ""}
                      {groupSelectionCount > 0 ? ` · ${groupSelectionCount}개 선택` : ""}
                    </small>
                  </span>
                  <span className="duplicate-group__metric">
                    <strong>{formatBytes(group.wastedBytes)}</strong>
                    <small>{group.files.length}개 파일</small>
                  </span>
                  {expanded ? <ChevronDown size={18} aria-hidden="true" /> : <ChevronRight size={18} aria-hidden="true" />}
                </button>
                {expanded ? (
                  <FileTable
                    files={group.files}
                    emptyMessage="그룹에 표시할 파일이 없습니다."
                    verified
                    selectedPaths={selectedPaths}
                    onSelectionChange={updateSelection}
                    selectionDisabled={scanning}
                    isSelectionDisabled={(file) =>
                      !selectedPaths.has(file.path) && groupSelectionCount >= group.files.length - 1
                    }
                  />
                ) : null}
              </article>
            );
          })}
        </section>
      )}

      <SafetyActionDialog
        open={dialogOpen}
        title="선택한 중복 파일을 휴지통으로 이동할까요?"
        itemCount={selectedPaths.size}
        logicalBytes={selectedBytes}
        busy={actionRunning}
        progress={actionProgress}
        error={dialogError ?? actionError}
        onConfirm={() => void confirmTrashMove()}
        onCancel={onCancelAction}
        onClose={() => setDialogOpen(false)}
      />
    </div>
  );
}

const photoExtensions = new Set([
  "arw", "avif", "bmp", "cr2", "cr3", "dng", "gif", "heic", "heif", "jpeg", "jpg",
  "nef", "orf", "png", "raf", "raw", "rw2", "tif", "tiff", "webp",
]);

function isPhoto(name: string): boolean {
  const extension = name.split(".").pop()?.toLocaleLowerCase("en-US");
  return extension ? photoExtensions.has(extension) : false;
}

function parentDirectories(paths: string[]): string[] {
  return [...new Set(paths.map(fileParent))];
}

function normalizeError(reason: unknown): string {
  if (reason instanceof Error) return reason.message;
  return typeof reason === "string" ? reason : "휴지통 이동을 완료하지 못했습니다";
}
