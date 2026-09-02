import type { VolumeInfo } from "../types";

export function findVolumeForPath(
  volumes: readonly VolumeInfo[],
  path: string | null,
): VolumeInfo | null {
  if (volumes.length === 0) return null;
  if (!path) return volumes.find((volume) => volume.isSystem) ?? volumes[0];

  const normalizedPath = normalizePath(path);
  return (
    [...volumes]
      .sort((left, right) => right.mountPoint.length - left.mountPoint.length)
      .find((volume) => isInsideMount(normalizedPath, normalizePath(volume.mountPoint)))
      ?? volumes.find((volume) => volume.isSystem)
      ?? volumes[0]
  );
}

function isInsideMount(path: string, mountPoint: string): boolean {
  if (mountPoint === "/") return path.startsWith("/");
  const boundary = mountPoint.endsWith("/") ? mountPoint : `${mountPoint}/`;
  return path === mountPoint || path.startsWith(boundary);
}

function normalizePath(path: string): string {
  let normalized = path;
  if (normalized.startsWith("\\\\?\\UNC\\")) {
    normalized = `\\\\${normalized.slice(8)}`;
  } else if (normalized.startsWith("\\\\?\\")) {
    normalized = normalized.slice(4);
  }
  normalized = normalized.replace(/\\/g, "/").toLocaleLowerCase("en-US");
  if (normalized.length > 1 && normalized.endsWith("/")) {
    normalized = normalized.replace(/\/+$/, "");
  }
  return normalized || "/";
}
