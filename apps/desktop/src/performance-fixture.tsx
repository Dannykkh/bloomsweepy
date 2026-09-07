import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { StrictMode, useState } from "react";
import { createRoot } from "react-dom/client";
import "./App.css";
import { AppShell } from "./components/AppShell";
import { LanguageProvider } from "./i18n";
import { LANGUAGE_STORAGE_KEY } from "./i18n/preference";
import type { PerformanceSnapshot, ViewId } from "./types";
import { PerformanceView } from "./views/PerformanceView";

const fixtureParams = new URLSearchParams(window.location.search);
const fixtureLanguage = fixtureParams.get("language") ?? "ko";
window.localStorage.setItem(LANGUAGE_STORAGE_KEY, fixtureLanguage);
mockWindows("main");

let fixtureSampleIndex = 0;

function fixtureSnapshot(): PerformanceSnapshot {
  const motionSample = fixtureSampleIndex++ % 2;
  const moving = fixtureParams.has("motion");
  const usedMemoryGb = moving ? (motionSample === 0 ? 12 : 18.72) : 15.8;
  return {
    snapshotId: `fixture-${Date.now()}`,
    capturedAtUnixMs: Date.now(),
    sampleWindowMs: 2_012,
    refreshAfterMs: 2_000,
    platform: "macos",
    logicalCpuCount: 10,
    cpuUsagePercent: moving ? (motionSample === 0 ? 18 : 76) : 37.4,
    memory: {
      totalBytes: 24 * 1024 ** 3,
      availableBytes: (24 - usedMemoryGb) * 1024 ** 3,
      usedBytes: usedMemoryGb * 1024 ** 3,
      totalSwapBytes: 4 * 1024 ** 3,
      usedSwapBytes: 620 * 1024 ** 2,
    },
    processes: [
      {
        targetId: "figma-target",
        displayName: "Figma",
        bundleIdentifier: "com.figma.Desktop",
        pid: 9124,
        kind: "guiApp",
        cpuCorePercent: 142.1,
        cpuMachinePercent: 14.2,
        residentBytes: 2.86 * 1024 ** 3,
        processCount: 9,
        canRequestTermination: true,
        terminationEligibility: "eligible",
      },
      {
        targetId: "chrome-target",
        displayName: "Google Chrome",
        bundleIdentifier: "com.google.Chrome",
        pid: 8132,
        kind: "guiApp",
        cpuCorePercent: 88,
        cpuMachinePercent: 8.8,
        residentBytes: 3.42 * 1024 ** 3,
        processCount: 18,
        canRequestTermination: true,
        terminationEligibility: "eligible",
      },
      {
        targetId: "music-target",
        displayName: "Spotify",
        bundleIdentifier: "com.spotify.client",
        pid: 7330,
        kind: "guiApp",
        cpuCorePercent: 26,
        cpuMachinePercent: 2.6,
        residentBytes: 780 * 1024 ** 2,
        processCount: 5,
        canRequestTermination: true,
        terminationEligibility: "eligible",
      },
      {
        targetId: null,
        displayName: "Finder",
        bundleIdentifier: "com.apple.finder",
        pid: 530,
        kind: "guiApp",
        cpuCorePercent: 8,
        cpuMachinePercent: 0.8,
        residentBytes: 312 * 1024 ** 2,
        processCount: 1,
        canRequestTermination: false,
        terminationEligibility: "protectedSystemApp",
      },
      {
        targetId: null,
        displayName: "BroomSweepy",
        bundleIdentifier: "com.broomsweepy.desktop",
        pid: 12004,
        kind: "guiApp",
        cpuCorePercent: 4,
        cpuMachinePercent: 0.4,
        residentBytes: 176 * 1024 ** 2,
        processCount: 1,
        canRequestTermination: false,
        terminationEligibility: "selfApp",
      },
    ],
    processesTruncated: false,
    capabilities: {
      processMetricsAvailable: true,
      gracefulTerminationAvailable: true,
      appMemoryCleanupAvailable: true,
      processScope: "guiApplications",
    },
  };
}

mockIPC((command, payload) => {
  if (command === "get_performance_snapshot") return fixtureSnapshot();
  if (command === "clean_app_memory") {
    return {
      outcome: "completed",
      allocatorReleasedBytes: fixtureParams.has("zero-released") ? 0 : 38 * 1024 ** 2,
      appResidentBeforeBytes: 214 * 1024 ** 2,
      appResidentAfterBytes: 176 * 1024 ** 2,
      systemAvailableBeforeBytes: 8.2 * 1024 ** 3,
      systemAvailableAfterBytes: 8.24 * 1024 ** 3,
      requestedAtUnixMs: Date.now(),
      completedAtUnixMs: Date.now() + 50,
    };
  }
  if (command === "prepare_graceful_process_termination") {
    const values = payload as Record<string, unknown> | undefined;
    const targetId = String(values?.targetId ?? "app");
    const displayName = targetId.startsWith("figma")
      ? "Figma"
      : targetId.startsWith("chrome")
        ? "Google Chrome"
        : "Spotify";
    return {
      outcome: "ready",
      preview: {
        previewId: "fixture-preview",
        displayName,
        pid: 9124,
        capturedAtUnixMs: Date.now(),
        expiresAtUnixMs: Date.now() + 30_000,
      },
    };
  }
  if (command === "execute_graceful_process_termination") {
    return {
      outcome: "requestSent",
      displayName: "Figma",
      requestedAtUnixMs: Date.now(),
    };
  }
  if (command === "set_application_language") return null;
  return null;
}, { shouldMockEvents: true });

function FixtureApp() {
  const [activeView, setActiveView] = useState<ViewId>("performance");
  return (
    <LanguageProvider>
      <AppShell
        activeView={activeView}
        root={null}
        report={null}
        volume={{
          name: "Macintosh HD",
          mountPoint: "/",
          fileSystem: "apfs",
          totalBytes: 494 * 1024 ** 3,
          availableBytes: 142 * 1024 ** 3,
          removable: false,
          readOnly: false,
          isDiskImage: false,
          isSystem: true,
        }}
        mobileNavigationOpen={false}
        selectionBlocked={false}
        dockerEnabled={false}
        onMobileNavigationChange={() => undefined}
        onNavigate={setActiveView}
        onPickFolder={() => undefined}
      >
        {activeView === "performance" ? <PerformanceView /> : (
          <div className="view-stack"><p>Performance fixture</p></div>
        )}
      </AppShell>
    </LanguageProvider>
  );
}

createRoot(document.getElementById("root")!).render(
  <StrictMode><FixtureApp /></StrictMode>,
);

if (new URLSearchParams(window.location.search).has("dialog")) {
  window.setTimeout(() => {
    document.querySelector<HTMLButtonElement>(
      ".performance-process-row__action button",
    )?.click();
  }, 80);
}

if (new URLSearchParams(window.location.search).has("memory-cleaned")) {
  window.setTimeout(() => {
    document.querySelector<HTMLButtonElement>(
      ".performance-memory-cleaner__button",
    )?.click();
  }, 80);
}
