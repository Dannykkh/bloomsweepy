import { useEffect, type ReactNode } from "react";
import {
  Boxes,
  FileSearch,
  FolderOpen,
  HardDrive,
  LayoutDashboard,
  Map,
  Menu,
  MessageSquare,
  Search,
  Settings,
  Sparkles,
  X,
} from "lucide-react";
import { formatBytes, formatDate } from "../lib/format";
import type {
  ScanReport,
  ViewId,
  VolumeInfo,
} from "../types";

interface AppShellProps {
  activeView: ViewId;
  children: ReactNode;
  root: string | null;
  report: ScanReport | null;
  volume: VolumeInfo | null;
  mobileNavigationOpen: boolean;
  selectionBlocked: boolean;
  dockerEnabled: boolean;
  onMobileNavigationChange: (open: boolean) => void;
  onNavigate: (view: ViewId) => void;
  onPickFolder: () => void;
}

const navigationBeforeDocker: Array<{
  id: ViewId;
  label: string;
  description: string;
  icon: typeof Map;
}> = [
  {
    id: "dashboard",
    label: "대시보드",
    description: "디스크와 최근 변화",
    icon: LayoutDashboard,
  },
  {
    id: "overview",
    label: "용량 관리",
    description: "지도·큰 파일·중복",
    icon: Map,
  },
];

const dockerNavigation = {
  id: "docker" as ViewId,
  label: "Docker 용량",
  description: "이미지·캐시·컨테이너",
  icon: Boxes,
};

const navigationAfterDocker: Array<{
  id: ViewId;
  label: string;
  description: string;
  icon: typeof Map;
}> = [
  {
    id: "files",
    label: "파일 이름 찾기",
    description: "이름과 위치로 찾기",
    icon: Search,
  },
  {
    id: "documents",
    label: "문서 내용 찾기",
    description: "문장으로 찾기",
    icon: FileSearch,
  },
  {
    id: "assistant",
    label: "대화",
    description: "설치된 AI CLI",
    icon: MessageSquare,
  },
  {
    id: "settings",
    label: "설정",
    description: "스캔 기준과 안전",
    icon: Settings,
  },
];

const storageViews = new Set<ViewId>([
  "overview",
  "large-files",
  "duplicates",
  "cleanup",
]);

const folderViews = new Set<ViewId>([
  "overview",
  "large-files",
  "duplicates",
  "files",
  "documents",
]);

const titles: Record<ViewId, { eyebrow: string; title: string; description: string }> = {
  dashboard: {
    eyebrow: "오늘의 저장공간",
    title: "대시보드",
    description: "드라이브 상태와 최근 변화를 한 번에 확인합니다.",
  },
  overview: {
    eyebrow: "용량 관리",
    title: "폴더 용량 지도",
    description: "큰 사각형부터 따라가며 용량이 늘어난 위치를 찾습니다.",
  },
  docker: {
    eyebrow: "개발 도구",
    title: "Docker 용량",
    description: "Docker가 보고한 이미지·컨테이너·볼륨·빌드 캐시 사용량을 확인합니다.",
  },
  files: {
    eyebrow: "파일 찾기",
    title: "파일 이름 찾기",
    description: "파일을 열지 않고 이름이나 폴더 위치로 찾습니다.",
  },
  "large-files": {
    eyebrow: "용량 관리",
    title: "큰 파일",
    description: "크기와 위치를 나란히 비교합니다.",
  },
  documents: {
    eyebrow: "문서 찾기",
    title: "문서 내용 찾기",
    description: "선택한 폴더의 문서를 미리 읽어 내용으로 빠르게 찾습니다.",
  },
  cleanup: {
    eyebrow: "용량 관리",
    title: "정리 후보",
    description: "오래된 임시 파일과 삭제 후 남은 흔적을 근거별로 검토합니다.",
  },
  duplicates: {
    eyebrow: "용량 관리",
    title: "중복 파일",
    description: "파일 내용을 끝까지 비교해 실제로 같은 결과만 표시합니다.",
  },
  assistant: {
    eyebrow: "선택한 대상과 대화",
    title: "대화",
    description: "폴더 또는 Docker를 고르면 앱이 확인하고 로컬 AI CLI가 결과를 설명합니다.",
  },
  settings: {
    eyebrow: "앱 설정",
    title: "스캔 설정",
    description: "분석 범위와 결과 한도를 조정합니다.",
  },
};

export function AppShell({
  activeView,
  children,
  root,
  report,
  volume,
  mobileNavigationOpen,
  selectionBlocked,
  dockerEnabled,
  onMobileNavigationChange,
  onNavigate,
  onPickFolder,
}: AppShellProps) {
  const navigation = dockerEnabled
    ? [...navigationBeforeDocker, dockerNavigation, ...navigationAfterDocker]
    : [...navigationBeforeDocker, ...navigationAfterDocker];
  const usedPercent = volume?.totalBytes
    ? ((volume.totalBytes - volume.availableBytes) / volume.totalBytes) * 100
    : 0;
  const page = titles[activeView];
  const showFolderButton = folderViews.has(activeView);
  const folderLabel =
    activeView === "files"
      ? "찾을 위치"
      : activeView === "documents"
        ? "문서를 읽을 폴더"
        : "검사할 폴더";

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
            const active =
              item.id === "overview"
                ? storageViews.has(activeView)
                : activeView === item.id;
            return (
              <button
                type="button"
                className={`nav-item ${active ? "is-active" : ""}`}
                aria-label={`${item.label}: ${item.description}`}
                aria-current={active ? "page" : undefined}
                data-tooltip={item.label}
                title={`${item.label} - ${item.description}`}
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
            <p className="eyebrow">
              {storageViews.has(activeView) && report
                ? `마지막 검사 ${formatDate(report.completedAtUnixMs)}`
                : page.eyebrow}
            </p>
            <h1>{page.title}</h1>
            <p>{page.description}</p>
          </div>
          {showFolderButton ? (
            <button
              className="folder-button"
              type="button"
              disabled={selectionBlocked}
              onClick={onPickFolder}
            >
              <FolderOpen size={17} aria-hidden="true" />
              <span>
                <small>{folderLabel}</small>
                <strong title={root ?? undefined}>{root ?? "폴더 선택"}</strong>
              </span>
            </button>
          ) : null}
        </header>
        {children}
      </main>
    </div>
  );
}
