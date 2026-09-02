import {
  AlertTriangle,
  ChevronRight,
  File,
  Folder,
  FolderOpen,
  HardDrive,
  RefreshCw,
  Search,
  X,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState, type RefObject } from "react";
import { formatBytes, formatCount, formatDate, formatDuration } from "../lib/format";
import type {
  DirectoryBreadcrumb,
  DirectoryNode,
  DirectoryScanProgress,
  DirectoryScanReport,
  ScanUiState,
} from "../types";
import { useLanguage } from "../i18n";

interface StorageTreemapPanelProps {
  root: string | null;
  report: DirectoryScanReport | null;
  progress: DirectoryScanProgress | null;
  state: ScanUiState;
  error: string | null;
  breadcrumbs: DirectoryBreadcrumb[];
  blocked: boolean;
  showAction?: boolean;
  onPickFolder: () => void;
  onStart: (path: string, breadcrumbs?: DirectoryBreadcrumb[]) => void;
  onCancel: () => void;
}

interface DisplayItem {
  id: string;
  name: string;
  path: string | null;
  logicalBytes: number;
  itemCount: number;
  node: DirectoryNode | null;
}

interface LayoutItem extends DisplayItem {
  x: number;
  y: number;
  width: number;
  height: number;
  tone: number;
}

const MAX_TREEMAP_ITEMS = 15;

export function StorageTreemapPanel({
  root,
  report,
  progress,
  state,
  error,
  breadcrumbs,
  blocked,
  showAction = true,
  onPickFolder,
  onStart,
  onCancel,
}: StorageTreemapPanelProps) {
  const { t } = useLanguage();
  const scanning = state === "scanning";
  const mapItems = useMemo(
    () => prepareDisplayItems(report, (count) => t("기타 {{count}}개", { count })),
    [report, t],
  );
  const [canvasRef, canvasSize] = useElementSize<HTMLDivElement>(mapItems.length > 0);
  const layout = useMemo(
    () => layoutTreemap(mapItems, canvasSize.width, canvasSize.height),
    [canvasSize.height, canvasSize.width, mapItems],
  );

  function openNode(node: DirectoryNode) {
    if (!node.isDirectory || scanning || blocked) return;
    onStart(node.path, [
      ...breadcrumbs,
      { name: node.name, path: node.path },
    ]);
  }

  function navigateTo(index: number) {
    const target = breadcrumbs[index];
    if (!target || scanning || blocked) return;
    onStart(target.path, breadcrumbs.slice(0, index + 1));
  }

  return (
    <section className="storage-map" id="storage-map" aria-labelledby="storage-map-title">
      <header className="storage-map__header">
        <div>
          <p className="eyebrow">{t("저장공간 트리맵")}</p>
          <h2 id="storage-map-title">{t("폴더 용량 지도")}</h2>
          <p>{t("사각형이 클수록 더 많은 용량을 사용합니다. 폴더를 누르면 안쪽으로 이동합니다.")}</p>
        </div>
        {showAction ? <div className="storage-map__actions">
          {scanning ? (
            <button className="secondary-button danger-outline" type="button" onClick={onCancel}>
              <X size={16} aria-hidden="true" />
              {t("분석 취소")}
            </button>
          ) : (
            <button
              className="primary-button"
              type="button"
              disabled={blocked}
              onClick={() => {
                if (root) onStart(root);
                else onPickFolder();
              }}
            >
              {report ? (
                <RefreshCw size={16} aria-hidden="true" />
              ) : root ? (
                <Search size={16} aria-hidden="true" />
              ) : (
                <FolderOpen size={16} aria-hidden="true" />
              )}
              {report ? t("처음 폴더 다시 보기") : root ? t("지도 다시 만들기") : t("폴더 선택")}
            </button>
          )}
        </div> : <span className="storage-map__mode">{t("읽기 전용")}</span>}
      </header>

      {breadcrumbs.length ? (
        <nav className="storage-map__breadcrumbs" aria-label={t("저장공간 맵 경로")}>
          {breadcrumbs.map((crumb, index) => (
            <span key={`${crumb.path}-${index}`}>
              {index > 0 ? <ChevronRight size={13} aria-hidden="true" /> : null}
              <button
                type="button"
                aria-current={index === breadcrumbs.length - 1 ? "location" : undefined}
                disabled={scanning || blocked || index === breadcrumbs.length - 1}
                onClick={() => navigateTo(index)}
                title={crumb.path}
              >
                {crumb.name}
              </button>
            </span>
          ))}
        </nav>
      ) : null}

      {scanning ? (
        <div className="storage-map__progress" role="status" aria-live="polite">
          <span className="drive-progress__spinner" aria-hidden="true" />
          <div>
            <strong>{t("폴더 구조를 분석하고 있습니다")}</strong>
            <small>
              {t("{{count}}개 항목 · {{size}} 확인", {
                count: formatCount(progress?.processedEntries ?? 0),
                size: formatBytes(progress?.processedBytes ?? 0),
              })}
            </small>
          </div>
        </div>
      ) : null}

      {error ? (
        <div className="storage-map__error" role="alert">
          <AlertTriangle size={16} aria-hidden="true" />
          {error}
        </div>
      ) : null}

      {report ? (
        <>
          <div className="storage-map__summary" aria-label={t("현재 폴더 요약")}>
            <StorageMapMetric label={t("현재 범위")} value={formatBytes(report.totalLogicalBytes)} />
            <StorageMapMetric label={t("직계 항목")} value={t("{{count}}개", { count: formatCount(report.directChildCount) })} />
            <StorageMapMetric label={t("하위 파일")} value={t("{{count}}개", { count: formatCount(report.totalFiles) })} />
            <StorageMapMetric label={t("빈 폴더")} value={t("{{count}}개", { count: formatCount(report.emptyDirectoryCount) })} />
          </div>

          {mapItems.length ? (
            <div className="storage-map__workspace">
              <div className="storage-map__canvas" ref={canvasRef} aria-label={t("파일과 폴더 크기 비교 지도")}>
                {layout.map((item) => {
                  const canOpen = item.node?.isDirectory === true;
                  const showName = item.width >= 70 && item.height >= 34;
                  const showSize = item.width >= 92 && item.height >= 56;
                  return (
                    <button
                      className={`storage-map-cell storage-map-cell--tone-${item.tone}`}
                      key={item.id}
                      type="button"
                      aria-disabled={!canOpen}
                      tabIndex={canOpen ? 0 : -1}
                      onClick={() => {
                        if (item.node) openNode(item.node);
                      }}
                      style={{
                        left: item.x + 2,
                        top: item.y + 2,
                        width: Math.max(1, item.width - 4),
                        height: Math.max(1, item.height - 4),
                      }}
                      title={`${item.name} · ${formatBytes(item.logicalBytes)}${canOpen ? ` · ${t("하위 폴더 탐색")}` : ""}`}
                    >
                      {showName ? (
                        <span className="storage-map-cell__name">
                          {canOpen ? <Folder size={14} aria-hidden="true" /> : <File size={14} aria-hidden="true" />}
                          {item.name}
                        </span>
                      ) : null}
                      {showSize ? (
                        <span className="storage-map-cell__size">{formatBytes(item.logicalBytes)}</span>
                      ) : null}
                    </button>
                  );
                })}
              </div>

              <aside className="storage-map__ranking" aria-label={t("현재 폴더 용량 순위")}>
                <div className="storage-map__ranking-heading">
                  <strong>{t("용량 순위")}</strong>
                  <small>{t("폴더를 선택하면 하위로 이동")}</small>
                </div>
                <div className="storage-map__ranking-list">
                  {report.children.slice(0, 12).map((node, index) => (
                    <button
                      type="button"
                      key={node.path}
                      disabled={!node.isDirectory || scanning || blocked}
                      onClick={() => openNode(node)}
                      title={node.path}
                    >
                      <span className={`storage-map__rank storage-map__rank--tone-${index % 6}`}>
                        {index + 1}
                      </span>
                      <span>
                        <strong>{node.name}</strong>
                        <small>
                          {node.isDirectory
                            ? t("{{files}}개 파일 · {{folders}}개 폴더", {
                                files: formatCount(node.fileCount),
                                folders: formatCount(node.directoryCount),
                              })
                            : t("파일")}
                        </small>
                      </span>
                      <span>
                        <strong>{formatBytes(node.logicalBytes)}</strong>
                        {node.isDirectory ? <ChevronRight size={13} aria-hidden="true" /> : null}
                      </span>
                    </button>
                  ))}
                </div>
              </aside>
            </div>
          ) : (
            <div className="storage-map__empty">
              <FolderOpen size={24} aria-hidden="true" />
              <strong>{t("표시할 용량 항목이 없습니다")}</strong>
              <p>{t("현재 폴더에는 용량이 있는 파일이 없거나 접근할 수 없습니다.")}</p>
            </div>
          )}

          <section className="empty-directory-list" aria-labelledby="empty-directory-title">
            <div className="empty-directory-list__heading">
              <span>
                <FolderOpen size={17} aria-hidden="true" />
                <span>
                  <strong id="empty-directory-title">{t("빈 폴더")}</strong>
                  <small>{t("직접 포함된 항목이 하나도 없는 폴더만 표시합니다.")}</small>
                </span>
              </span>
              <strong>{t("{{count}}개", { count: formatCount(report.emptyDirectoryCount) })}</strong>
            </div>
            {report.emptyDirectories.length ? (
              <div className="empty-directory-list__rows">
                {report.emptyDirectories.slice(0, 8).map((directory) => (
                  <div key={directory.path}>
                    <span>
                      <strong>{directory.name}</strong>
                      <small title={directory.path}>{directory.path}</small>
                    </span>
                    <time dateTime={directory.modifiedAtUnixMs ? new Date(directory.modifiedAtUnixMs).toISOString() : undefined}>
                      {formatDate(directory.modifiedAtUnixMs)}
                    </time>
                  </div>
                ))}
              </div>
            ) : (
              <p className="empty-directory-list__none">{t("접근 가능한 범위에서 빈 폴더가 발견되지 않았습니다.")}</p>
            )}
            {report.emptyDirectoryCount > 8 ? (
              <p className="empty-directory-list__notice">
                {report.emptyDirectoriesTruncated
                  ? t("화면에는 8개만 표시합니다. 전체 {{total}}개 중 최대 {{kept}}개 경로를 보관했습니다.", {
                      total: formatCount(report.emptyDirectoryCount),
                      kept: formatCount(report.emptyDirectories.length),
                    })
                  : t("화면에는 8개만 표시합니다. 전체 {{total}}개 중 나머지 경로도 스캔 결과에 보관돼 있습니다.", {
                      total: formatCount(report.emptyDirectoryCount),
                    })}
              </p>
            ) : null}
          </section>

          <footer className="storage-map__footer">
            <span>
              <HardDrive size={14} aria-hidden="true" />
              {t("{{duration}} · 읽기 전용 분석", { duration: formatDuration(report.durationMs) })}
            </span>
            <span>
              {t("접근 제한 {{count}}개", { count: formatCount(report.unreadableEntries) })}
              {report.childrenTruncated
                ? ` · ${t("작은 직계 항목 {{count}}개 집계", { count: formatCount(report.omittedChildCount) })}`
                : ""}
              {report.trackingLimitReached ? ` · ${t("개별 항목 보관 안전 상한 도달")}` : ""}
            </span>
          </footer>
        </>
      ) : showAction ? (
        <button
          className="storage-map__start"
          type="button"
          disabled={blocked || scanning}
          onClick={() => {
            if (root) onStart(root);
            else onPickFolder();
          }}
        >
          <span aria-hidden="true">
            <Folder size={22} />
            <ChevronRight size={16} />
            <Folder size={18} />
          </span>
          <strong>{root ? t("큰 사각형부터 폴더 안쪽으로 이동합니다") : t("먼저 검사할 폴더를 선택하세요")}</strong>
          <small>{root ? t("지도를 다시 만들 수 있습니다.") : t("폴더를 고르면 용량 지도를 바로 만듭니다.")}</small>
        </button>
      ) : (
        <div className="storage-map__start is-static">
          <span aria-hidden="true">
            <Folder size={22} />
            <ChevronRight size={16} />
            <Folder size={18} />
          </span>
          <strong>{root ? t("큰 사각형부터 폴더 안쪽으로 이동합니다") : t("먼저 검사할 폴더를 선택하세요")}</strong>
          <small>{root ? t("위 안내의 버튼으로 폴더 용량 지도를 만드세요.") : t("폴더를 고른 뒤 용량 지도를 만들 수 있습니다.")}</small>
        </div>
      )}
    </section>
  );
}

function StorageMapMetric({ label, value }: { label: string; value: string }) {
  return (
    <span>
      <small>{label}</small>
      <strong>{value}</strong>
    </span>
  );
}

function prepareDisplayItems(
  report: DirectoryScanReport | null,
  formatOther: (count: string) => string,
): DisplayItem[] {
  if (!report) return [];

  const positive = report.children
    .filter((node) => node.logicalBytes > 0)
    .sort((left, right) => right.logicalBytes - left.logicalBytes);
  const needsAggregate =
    positive.length > MAX_TREEMAP_ITEMS || report.omittedLogicalBytes > 0;
  const directLimit = needsAggregate ? MAX_TREEMAP_ITEMS - 1 : MAX_TREEMAP_ITEMS;
  const direct: DisplayItem[] = positive.slice(0, directLimit).map((node) => ({
    id: node.path,
    name: node.name,
    path: node.path,
    logicalBytes: node.logicalBytes,
    itemCount: 1,
    node,
  }));
  const remainder = positive.slice(directLimit);
  const remainderBytes = remainder.reduce(
    (total, node) => total + node.logicalBytes,
    report.omittedLogicalBytes,
  );
  const remainderCount = remainder.length + report.omittedChildCount;

  if (remainderBytes > 0) {
    direct.push({
      id: `${report.root}::other`,
      name: formatOther(formatCount(remainderCount)),
      path: null,
      logicalBytes: remainderBytes,
      itemCount: remainderCount,
      node: null,
    });
  }

  return direct;
}

function layoutTreemap(items: DisplayItem[], width: number, height: number): LayoutItem[] {
  if (!items.length || width <= 0 || height <= 0) return [];
  const sorted = [...items].sort((left, right) => right.logicalBytes - left.logicalBytes);
  const result: LayoutItem[] = [];

  function place(slice: DisplayItem[], x: number, y: number, w: number, h: number) {
    if (!slice.length || w <= 0 || h <= 0) return;
    if (slice.length === 1) {
      result.push({ ...slice[0], x, y, width: w, height: h, tone: result.length % 6 });
      return;
    }

    const total = slice.reduce((sum, item) => sum + item.logicalBytes, 0);
    const target = total / 2;
    let firstTotal = 0;
    let splitIndex = 1;
    for (let index = 0; index < slice.length - 1; index += 1) {
      const next = firstTotal + slice[index].logicalBytes;
      if (index > 0 && Math.abs(target - firstTotal) <= Math.abs(target - next)) break;
      firstTotal = next;
      splitIndex = index + 1;
    }

    const ratio = total > 0 ? firstTotal / total : 0.5;
    if (w >= h) {
      const firstWidth = w * ratio;
      place(slice.slice(0, splitIndex), x, y, firstWidth, h);
      place(slice.slice(splitIndex), x + firstWidth, y, w - firstWidth, h);
    } else {
      const firstHeight = h * ratio;
      place(slice.slice(0, splitIndex), x, y, w, firstHeight);
      place(slice.slice(splitIndex), x, y + firstHeight, w, h - firstHeight);
    }
  }

  place(sorted, 0, 0, width, height);
  return result;
}

function useElementSize<T extends HTMLElement>(
  enabled: boolean,
): [RefObject<T | null>, { width: number; height: number }] {
  const ref = useRef<T>(null);
  const [size, setSize] = useState({ width: 0, height: 0 });

  useEffect(() => {
    if (!enabled) return;
    const element = ref.current;
    if (!element) return;

    const update = () => {
      const bounds = element.getBoundingClientRect();
      setSize({ width: bounds.width, height: bounds.height });
    };
    update();
    const observer = new ResizeObserver(update);
    observer.observe(element);
    return () => observer.disconnect();
  }, [enabled]);

  return [ref, size];
}
