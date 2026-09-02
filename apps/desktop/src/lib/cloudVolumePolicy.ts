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

export function visibleDashboardVolumes(volumes: readonly VolumeInfo[]): VolumeInfo[] {
  return volumes
    .filter((volume) => !isCloudMountedVolume(volume))
    .sort((left, right) => left.mountPoint.localeCompare(
      right.mountPoint,
      "en-US",
      { numeric: true, sensitivity: "base" },
    ));
}
