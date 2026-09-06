import type { VolumeInfo } from "../types";

const cloudVolumeNames = [
  "googledrive",
  "googledrivefs",
  "drivefs",
  "onedrive",
  "dropbox",
  "iclouddrive",
  "pcloud",
];

export function isCloudMountedVolume(
  volume: Pick<VolumeInfo, "name" | "mountPoint">,
): boolean {
  const identity = `${volume.name} ${volume.mountPoint}`
    .normalize("NFKC")
    .toLocaleLowerCase("en-US")
    .replace(/…/g, "...");
  const compactIdentity = identity.replace(/[^a-z0-9]+/g, "");

  if (cloudVolumeNames.some((provider) => compactIdentity.includes(provider))) {
    return true;
  }

  // Google Drive for desktop can store an already-truncated Windows volume label,
  // for example "account@gmail.com - Google..." instead of "Google Drive".
  return identity.includes("@") && /\s-\sgoogle(?:\s+dr)?\.{3}(?:\s|$)/.test(identity);
}

export function visibleDashboardVolumes(
  volumes: readonly VolumeInfo[],
  platform?: string | null,
): VolumeInfo[] {
  return volumes
    .filter((volume) => !isCloudMountedVolume(volume))
    .filter((volume) => !volume.isDiskImage)
    .filter((volume) => platform !== "macos" || isUserFacingMacVolume(volume))
    .sort((left, right) => left.mountPoint.localeCompare(
      right.mountPoint,
      "en-US",
      { numeric: true, sensitivity: "base" },
    ));
}

function isUserFacingMacVolume(
  volume: Pick<VolumeInfo, "mountPoint">,
): boolean {
  const key = dashboardVolumeKey(volume);
  return key === "/" || key.startsWith("/Volumes/");
}

export function dashboardVolumeKey(
  volume: Pick<VolumeInfo, "mountPoint">,
): string {
  let normalized = volume.mountPoint;
  const isWindowsPath = normalized.startsWith("\\\\")
    || /^[a-z]:(?:[\\/]|$)/i.test(normalized);

  if (!isWindowsPath) {
    if (normalized.length > 1) normalized = normalized.replace(/\/+$/, "");
    return normalized || "/";
  }

  if (normalized.startsWith("\\\\?\\UNC\\")) {
    normalized = `\\\\${normalized.slice(8)}`;
  } else if (normalized.startsWith("\\\\?\\")) {
    normalized = normalized.slice(4);
  }

  normalized = normalized.replace(/\\/g, "/").toLocaleLowerCase("en-US");
  if (normalized.length > 1) normalized = normalized.replace(/\/+$/, "");
  return normalized || "/";
}

export function resolveDashboardVolume(
  volumes: readonly VolumeInfo[],
  selectedMountPoint: string | null,
): VolumeInfo | null {
  const visible = visibleDashboardVolumes(volumes);
  if (visible.length === 0) return null;

  if (selectedMountPoint) {
    const selectedKey = dashboardVolumeKey({ mountPoint: selectedMountPoint });
    const selected = visible.find(
      (volume) => dashboardVolumeKey(volume) === selectedKey,
    );
    if (selected) return selected;
  }

  return (
    visible.find((volume) => volume.isSystem)
    ?? visible.find((volume) => {
      const key = dashboardVolumeKey(volume);
      return key === "/" || key === "/System/Volumes/Data" || key === "c:";
    })
    ?? visible.find((volume) => !volume.removable && !volume.readOnly)
    ?? visible[0]
  );
}
