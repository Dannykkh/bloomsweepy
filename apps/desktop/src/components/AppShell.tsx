import { useEffect, type ReactNode } from "react";
import {
  Copy,
  FileSearch,
  FolderOpen,
  HardDrive,
  LayoutDashboard,
  ListChecks,
  Menu,
  Search,
  Settings,
  Sparkles,
  X,
} from "lucide-react";
import { formatBytes, formatDate } from "../lib/format";
import type {
  CleanupScanReport,
  DocumentIndexStatus,
  FileCatalogStatus,
  ScanReport,
  ViewId,
  VolumeInfo,
} from "../types";

interface AppShellProps {
  activeView: ViewId;
  children: ReactNode;
  root: string | null;
  report: ScanReport | null;
  cleanupReport: CleanupScanReport | null;
  documentIndex: DocumentIndexStatus | null;
  fileCatalog: FileCatalogStatus | null;
  volume: VolumeInfo | null;
  mobileNavigationOpen: boolean;
  selectionBlocked: boolean;
  onMobileNavigationChange: (open: boolean) => void;
  onNavigate: (view: ViewId) => void;
  onPickFolder: () => void;
}

const navigation: Array<{
  id: ViewId;
  label: string;
  description: string;
  icon: typeof LayoutDashboard;
}> = [
  {
    id: "overview",
    label: "대시보드",
    description: "현재 상태와 스캔",
    icon: LayoutDashboard,
  },
  {
    id: "files",
    label: "빠른 파일 찾기",
    description: "이름과 위치로 찾기",
    icon: Search,
  },
  {
    id: "documents",
    label: "문서 검색",
    description: "파일 내용과 문장",
    icon: FileSearch,
  },
  {
    id: "cleanup",
    label: "정리 후보",
    description: "임시 파일·프로그램 흔적",
    icon: ListChecks,
  },
  {
    id: "large-files",
    label: "공간 정리",
    description: "용량이 큰 파일",
    icon: HardDrive,
  },
  {
    id: "duplicates",
    label: "중복 파일",
    description: "전체 내용 검증",
    icon: Copy,
  },
  {
    id: "settings",
    label: "설정",
    description: "스캔 기준과 안전",
    icon: Settings,
  },
];

const titles: Record<ViewId, { title: string; description: string }> = {
  overview: {
    title: "저장공간 대시보드",
    description: "용량이 늘어난 위치를 찾고 삭제 전에 근거를 확인합니다.",
  },
  files: {
    title: "빠른 파일 찾기",
    description: "파일을 열지 않고 이름이나 폴더 위치로 찾습니다.",
  },
  "large-files": {
    title: "큰 파일",
    description: "크기와 위치를 나란히 비교합니다.",
  },
  documents: {
    title: "문서 검색",
    description: "선택한 폴더의 문서를 미리 읽어 내용으로 빠르게 찾습니다.",
  },
  cleanup: {
    title: "정리 후보",
    description: "오래된 임시 파일과 삭제 후 남은 흔적을 근거별로 검토합니다.",
  },
  duplicates: {
    title: "중복 파일",
    description: "파일 내용을 끝까지 비교해 실제로 같은 결과만 표시합니다.",
  },
  settings: {
    title: "스캔 설정",
    description: "분석 범위와 결과 한도를 조정합니다.",
  },
};

export function AppShell({
  activeView,
  children,
  root,
  report,
  cleanupReport,
  documentIndex,
  fileCatalog,
  volume,
  mobileNavigationOpen,
  selectionBlocked,
  onMobileNavigationChange,
  onNavigate,
  onPickFolder,
}: AppShellProps) {
  const usedPercent = volume?.totalBytes
    ? ((volume.totalBytes - volume.availableBytes) / volume.totalBytes) * 100
    : 0;
  const page = titles[activeView];

  useEffect(() => {
    if (!mobileNavigationOpen) return;

    function closeOnEscape(event: KeyboardEvent) {
      if (event.key === "Escape") onMobileNavigationChange(false);
    }

    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [mobileNavigationOpen, onMobileNavigationChange]);

  function navigate(view: ViewId) {
    onNavigate(view);
    onMobileNavigationChange(false);
  }

  return (
    <div className="app-shell">
      <a className="skip-link" href="#main-content">
        본문으로 건너뛰기
      </a>
      <button
        className="mobile-nav-button icon-button"
        type="button"
        aria-label={mobileNavigationOpen ? "내비게이션 닫기" : "내비게이션 열기"}
        aria-expanded={mobileNavigationOpen}
        aria-controls="primary-sidebar"
        onClick={() => onMobileNavigationChange(!mobileNavigationOpen)}
      >
        {mobileNavigationOpen ? (
          <X size={20} aria-hidden="true" />
        ) : (
          <Menu size={20} aria-hidden="true" />
        )}
      </button>

      <aside
        className={`sidebar ${mobileNavigationOpen ? "is-open" : ""}`}
        id="primary-sidebar"
        aria-label="BroomSweepy 내비게이션"
      >
        <div className="brand-lockup">
          <span className="brand-lockup__mark" aria-hidden="true">
            <Sparkles size={19} />
          </span>
          <span className="brand-lockup__copy">
            <strong>BroomSweepy</strong>
            <small>Storage instrument</small>
          </span>
        </div>

        <nav className="primary-navigation" aria-label="주요 화면">
          {navigation.map((item) => {
            const Icon = item.icon;
            const active = activeView === item.id;
            const badge =
              item.id === "files"
                ? fileCatalog?.indexedEntries
                : item.id === "documents"
                ? documentIndex?.indexedDocuments
                : item.id === "cleanup"
                ? cleanupReport
                  ? cleanupReport.candidates.length + cleanupReport.registryResidues.candidates.length
                  : null
                : item.id === "large-files"
                ? report?.largeFiles.length
                : item.id === "duplicates"
                  ? report?.duplicateGroups.length
                  : null;

            return (
              <button
                type="button"
                className={`nav-item ${active ? "is-active" : ""}`}
                aria-label={`${item.label}: ${item.description}`}
                aria-current={active ? "page" : undefined}
                key={item.id}
                onClick={() => navigate(item.id)}
              >
                <span className="nav-item__icon" aria-hidden="true">
                  <Icon size={17} />
                </span>
                <span className="nav-item__copy">
                  <strong>{item.label}</strong>
                  <small>{item.description}</small>
                </span>
                {badge !== null && badge !== undefined ? (
                  <span className="nav-item__badge">{badge}</span>
                ) : null}
              </button>
            );
          })}
        </nav>

        <div className="sidebar-volume" aria-label="기본 디스크 상태">
          <div className="sidebar-volume__line">
            <HardDrive size={14} aria-hidden="true" />
            <span>{volume ? `${formatBytes(volume.availableBytes)} 여유` : "디스크 확인 중"}</span>
            {volume ? <strong>{Math.round(usedPercent)}%</strong> : null}
          </div>
          <div className="usage-track" aria-hidden="true">
            <span style={{ transform: `scaleX(${usedPercent / 100})` }} />
          </div>
        </div>
      </aside>

      {mobileNavigationOpen ? (
        <button
          className="navigation-scrim"
          type="button"
          aria-label="내비게이션 닫기"
          onClick={() => onMobileNavigationChange(false)}
        />
      ) : null}

      <main className="main-content" id="main-content" tabIndex={-1}>
        <header className="utility-header">
          <div>
            <p className="eyebrow">{report ? `마지막 스캔 ${formatDate(report.completedAtUnixMs)}` : "분석 준비"}</p>
            <h1>{page.title}</h1>
            <p>{page.description}</p>
          </div>
          <button
            className="folder-button"
            type="button"
            disabled={selectionBlocked}
            onClick={onPickFolder}
          >
            <FolderOpen size={17} aria-hidden="true" />
            <span>
              <small>{activeView === "files" ? "찾을 위치" : "스캔 범위"}</small>
              <strong title={root ?? undefined}>{root ?? "폴더 선택"}</strong>
            </span>
          </button>
        </header>
        {children}
      </main>
    </div>
  );
}
