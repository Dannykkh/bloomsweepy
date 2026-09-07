import { useEffect, useRef, type ReactNode } from "react";
import {
  AppWindow,
  Boxes,
  Folder,
  FolderOpen,
  Gauge,
  HardDrive,
  LayoutDashboard,
  Menu,
  MessageSquare,
  Settings,
  Sparkles,
  X,
  type LucideIcon,
} from "lucide-react";
import { formatBytes, formatDate } from "../lib/format";
import { useLanguage, type MessageKey } from "../i18n";
import type { ScanReport, ViewId, VolumeInfo } from "../types";

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

type NavigationTone =
  | "dashboard"
  | "space"
  | "files"
  | "performance"
  | "applications"
  | "assistant"
  | "docker"
  | "settings";

interface NavigationItem {
  id: ViewId;
  label: MessageKey;
  description: MessageKey;
  icon: LucideIcon;
  tone: NavigationTone;
  activeViews: ReadonlySet<ViewId>;
}

const dashboardViews = new Set<ViewId>(["dashboard"]);
const storageViews = new Set<ViewId>([
  "overview",
  "large-files",
  "duplicates",
  "cleanup",
]);
const fileViews = new Set<ViewId>(["files", "documents"]);
const performanceViews = new Set<ViewId>(["performance"]);
const applicationViews = new Set<ViewId>(["applications"]);
const assistantViews = new Set<ViewId>(["assistant"]);
const dockerViews = new Set<ViewId>(["docker"]);
const settingsViews = new Set<ViewId>(["settings"]);

const navigationBeforeDocker: NavigationItem[] = [
  {
    id: "dashboard",
    label: "대시보드",
    description: "디스크와 최근 변화",
    icon: LayoutDashboard,
    tone: "dashboard",
    activeViews: dashboardViews,
  },
  {
    id: "performance",
    label: "성능",
    description: "CPU·메모리 상태",
    icon: Gauge,
    tone: "performance",
    activeViews: performanceViews,
  },
  {
    id: "applications",
    label: "앱 관리",
    description: "설치된 앱",
    icon: AppWindow,
    tone: "applications",
    activeViews: applicationViews,
  },
  {
    id: "overview",
    label: "공간 정리",
    description: "지도·큰 파일·중복",
    icon: HardDrive,
    tone: "space",
    activeViews: storageViews,
  },
  {
    id: "files",
    label: "파일 관리",
    description: "이름과 문서 내용으로 찾기",
    icon: Folder,
    tone: "files",
    activeViews: fileViews,
  },
  {
    id: "assistant",
    label: "AI 도우미",
    description: "설치된 AI CLI",
    icon: MessageSquare,
    tone: "assistant",
    activeViews: assistantViews,
  },
];

const dockerNavigation: NavigationItem = {
  id: "docker",
  label: "Docker 관리",
  description: "이미지·캐시·컨테이너",
  icon: Boxes,
  tone: "docker",
  activeViews: dockerViews,
};

const navigationAfterDocker: NavigationItem[] = [
  {
    id: "settings",
    label: "설정",
    description: "스캔 기준과 안전",
    icon: Settings,
    tone: "settings",
    activeViews: settingsViews,
  },
];

const folderViews = new Set<ViewId>([
  "overview",
  "large-files",
  "duplicates",
  "files",
  "documents",
]);

const titles: Record<
  ViewId,
  { eyebrow: MessageKey; title: MessageKey; description: MessageKey }
> = {
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
  applications: {
    eyebrow: "앱 관리",
    title: "설치된 앱",
    description: "앱 제거와 관련 데이터 정리를 나누어 검토합니다.",
  },
  duplicates: {
    eyebrow: "용량 관리",
    title: "중복 파일",
    description: "파일 내용을 끝까지 비교해 실제로 같은 결과만 표시합니다.",
  },
  performance: {
    eyebrow: "실시간 상태",
    title: "성능",
    description: "실제 CPU와 메모리 사용량, 많이 사용하는 앱을 확인합니다.",
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
  const { t } = useLanguage();
  const mobileNavigationButtonRef = useRef<HTMLButtonElement>(null);
  const navigation = dockerEnabled
    ? [...navigationBeforeDocker, dockerNavigation, ...navigationAfterDocker]
    : [...navigationBeforeDocker, ...navigationAfterDocker];
  const usedPercent = volume?.totalBytes
    ? ((volume.totalBytes - volume.availableBytes) / volume.totalBytes) * 100
    : 0;
  const page = titles[activeView];
  const showFolderButton = folderViews.has(activeView);
  const folderLabel: MessageKey =
    activeView === "files"
      ? "찾을 위치"
      : activeView === "documents"
        ? "문서를 읽을 폴더"
        : "검사할 폴더";

  useEffect(() => {
    if (!mobileNavigationOpen) return;

    function closeOnEscape(event: KeyboardEvent) {
      if (event.key === "Escape") {
        onMobileNavigationChange(false);
        requestAnimationFrame(() => mobileNavigationButtonRef.current?.focus());
      }
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
        {t("본문으로 건너뛰기")}
      </a>
      <button
        ref={mobileNavigationButtonRef}
        className="mobile-nav-button icon-button"
        type="button"
        aria-label={mobileNavigationOpen ? t("내비게이션 닫기") : t("내비게이션 열기")}
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
        aria-label={t("BroomSweepy 내비게이션")}
      >
        <div className="brand-lockup">
          <span className="brand-lockup__mark" aria-hidden="true">
            <Sparkles size={19} />
          </span>
          <span className="brand-lockup__copy">
            <strong translate="no">BroomSweepy</strong>
          </span>
        </div>

        <nav className="primary-navigation" aria-label={t("주요 화면")}>
          {navigation.map((item) => {
            const Icon = item.icon;
            const active = item.activeViews.has(activeView);
            const current = activeView === item.id;
            const stateClasses = [
              `nav-item--${item.tone}`,
              active ? "is-active" : "",
              current ? "is-current" : "",
            ]
              .filter(Boolean)
              .join(" ");
            return (
              <button
                type="button"
                className={`nav-item ${stateClasses}`}
                aria-label={`${t(item.label)}: ${t(item.description)}`}
                aria-current={current ? "page" : undefined}
                data-tone={item.tone}
                data-tooltip={t(item.label)}
                title={`${t(item.label)} - ${t(item.description)}`}
                key={item.id}
                onClick={() => navigate(item.id)}
              >
                <span className="nav-item__icon" aria-hidden="true">
                  <Icon size={18} strokeWidth={2} />
                </span>
                <span className="nav-item__copy">
                  <strong>{t(item.label)}</strong>
                  <small>{t(item.description)}</small>
                </span>
              </button>
            );
          })}
        </nav>

        <div className="sidebar-volume" aria-label={t("기본 디스크 상태")}>
          <div className="sidebar-volume__line">
            <HardDrive size={14} aria-hidden="true" />
            <span>
              {volume
                ? t("{{size}} 여유", { size: formatBytes(volume.availableBytes) })
                : t("디스크 확인 중")}
            </span>
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
          aria-label={t("내비게이션 닫기")}
          onClick={() => onMobileNavigationChange(false)}
        />
      ) : null}

      <main
        className="main-content"
        id="main-content"
        inert={mobileNavigationOpen ? true : undefined}
        tabIndex={-1}
      >
        {activeView !== "dashboard" ? (
          <header className={[
            "utility-header utility-header--compact",
            storageViews.has(activeView) ? "utility-header--storage" : "",
            activeView === "assistant" ? "utility-header--assistant" : "",
          ].filter(Boolean).join(" ")}>
            <div className="utility-header__identity">
              <p className="eyebrow utility-header__context">
                {storageViews.has(activeView) && report
                  ? t("마지막 검사 {{date}}", {
                      date: formatDate(report.completedAtUnixMs),
                    })
                  : t(page.eyebrow)}
              </p>
              <h1>{t(page.title)}</h1>
              <p className="utility-header__description">{t(page.description)}</p>
            </div>
            {showFolderButton ? (
              <button
                className="folder-button utility-header__folder"
                type="button"
                aria-label={`${t(folderLabel)}: ${root ?? t("폴더 선택")}`}
                disabled={selectionBlocked}
                onClick={onPickFolder}
              >
                <FolderOpen size={17} aria-hidden="true" />
                <span>
                  <small>{t(folderLabel)}</small>
                  <strong title={root ?? undefined}>{root ?? t("폴더 선택")}</strong>
                </span>
              </button>
            ) : null}
          </header>
        ) : null}
        {children}
      </main>
    </div>
  );
}
