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

interface StorageTreemapPanelProps {
  root: string | null;
  report: DirectoryScanReport | null;
  progress: DirectoryScanProgress | null;
  state: ScanUiState;
  error: string | null;
  breadcrumbs: DirectoryBreadcrumb[];
  blocked: boolean;
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
  onStart,
  onCancel,
}: StorageTreemapPanelProps) {
  const scanning = state === "scanning";
  const mapItems = useMemo(() => prepareDisplayItems(report), [report]);
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
          <p className="eyebrow">STORAGE TREEMAP</p>
          <h2 id="storage-map-title">비례사각형 저장공간 맵</h2>
          <p>사각형 면적은 파일과 폴더의 논리 용량에 비례합니다.</p>
        </div>
        <div className="storage-map__actions">
          {scanning ? (
            <button className="secondary-button danger-outline" type="button" onClick={onCancel}>
              <X size={16} aria-hidden="true" />
              분석 취소
            </button>
          ) : (
            <button
              className="primary-button"
              type="button"
              disabled={!root || blocked}
              onClick={() => {
                if (root) onStart(root);
              }}
            >
              {report ? <RefreshCw size={16} aria-hidden="true" /> : <Search size={16} aria-hidden="true" />}
              {report ? "처음부터 다시 분석" : "저장공간 맵 만들기"}
            </button>
          )}
        </div>
      </header>

      {breadcrumbs.length ? (
        <nav className="storage-map__breadcrumbs" aria-label="저장공간 맵 경로">
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
            <strong>{progress?.message ?? "폴더 구조를 분석하고 있습니다"}</strong>
            <small>
              {formatCount(progress?.processedEntries ?? 0)}개 항목 · {formatBytes(progress?.processedBytes ?? 0)} 확인
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
          <div className="storage-map__summary" aria-label="현재 폴더 요약">
            <StorageMapMetric label="현재 범위" value={formatBytes(report.totalLogicalBytes)} />
            <StorageMapMetric label="직계 항목" value={`${formatCount(report.directChildCount)}개`} />
            <StorageMapMetric label="하위 파일" value={`${formatCount(report.totalFiles)}개`} />
            <StorageMapMetric label="빈 폴더" value={`${formatCount(report.emptyDirectoryCount)}개`} />
          </div>

          {mapItems.length ? (
            <div className="storage-map__workspace">
              <div className="storage-map__canvas" ref={canvasRef} aria-label="용량 비례 트리맵">
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
                      title={`${item.name} · ${formatBytes(item.logicalBytes)}${canOpen ? " · 하위 폴더 탐색" : ""}`}
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

              <aside className="storage-map__ranking" aria-label="현재 폴더 용량 순위">
                <div className="storage-map__ranking-heading">
                  <strong>용량 순위</strong>
                  <small>폴더를 선택하면 하위로 이동</small>
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
                            ? `${formatCount(node.fileCount)}개 파일 · ${formatCount(node.directoryCount)}개 폴더`
                            : "파일"}
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
              <strong>표시할 용량 항목이 없습니다</strong>
              <p>현재 폴더에는 용량이 있는 파일이 없거나 접근할 수 없습니다.</p>
            </div>
          )}

          <section className="empty-directory-list" aria-labelledby="empty-directory-title">
            <div className="empty-directory-list__heading">
              <span>
                <FolderOpen size={17} aria-hidden="true" />
                <span>
                  <strong id="empty-directory-title">빈 폴더</strong>
                  <small>직접 포함된 항목이 하나도 없는 폴더만 표시합니다.</small>
                </span>
              </span>
              <strong>{formatCount(report.emptyDirectoryCount)}개</strong>
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
              <p className="empty-directory-list__none">접근 가능한 범위에서 빈 폴더가 발견되지 않았습니다.</p>
            )}
            {report.emptyDirectoryCount > 8 ? (
              <p className="empty-directory-list__notice">
                화면에는 8개만 표시합니다. 전체 {formatCount(report.emptyDirectoryCount)}개 중
                {report.emptyDirectoriesTruncated
                  ? ` 최대 ${formatCount(report.emptyDirectories.length)}개 경로를 보관했습니다.`
                  : " 나머지 경로도 스캔 결과에 보관돼 있습니다."}
              </p>
            ) : null}
          </section>

          <footer className="storage-map__footer">
            <span>
              <HardDrive size={14} aria-hidden="true" />
              {formatDuration(report.durationMs)} · 읽기 전용 분석
            </span>
            <span>
              접근 제한 {formatCount(report.unreadableEntries)}개
              {report.childrenTruncated
                ? ` · 작은 직계 항목 ${formatCount(report.omittedChildCount)}개 집계`
                : ""}
              {report.trackingLimitReached ? " · 개별 항목 보관 안전 상한 도달" : ""}
            </span>
          </footer>
        </>
      ) : (
        <button
          className="storage-map__start"
          type="button"
          disabled={!root || blocked || scanning}
          onClick={() => {
            if (root) onStart(root);
          }}
        >
          <span aria-hidden="true">
            <Folder size={22} />
            <ChevronRight size={16} />
            <Folder size={18} />
          </span>
          <strong>큰 사각형부터 폴더 안쪽으로 이동합니다</strong>
          <small>용량 지도와 빈 폴더 목록을 만들려면 분석을 시작하세요.</small>
        </button>
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

function prepareDisplayItems(report: DirectoryScanReport | null): DisplayItem[] {
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
      name: `기타 ${formatCount(remainderCount)}개`,
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
