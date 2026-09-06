import type { PerformanceProcessUsage, PerformanceSnapshot } from "../types";

export type PerformanceSort = "cpu" | "memory";

export function clampPercent(value: number): number {
  if (!Number.isFinite(value)) return 0;
  return Math.min(100, Math.max(0, value));
}

export function memoryUsagePercent(snapshot: PerformanceSnapshot | null): number {
  if (!snapshot || snapshot.memory.totalBytes <= 0) return 0;
  return clampPercent((snapshot.memory.usedBytes / snapshot.memory.totalBytes) * 100);
}

export function memoryCleanupReleasedBytes(value: number): boolean {
  return Number.isFinite(value) && value > 0;
}

export function sortPerformanceProcesses(
  processes: readonly PerformanceProcessUsage[],
  sort: PerformanceSort,
): PerformanceProcessUsage[] {
  return [...processes].sort((left, right) => {
    const difference = sort === "cpu"
      ? right.cpuMachinePercent - left.cpuMachinePercent
      : right.residentBytes - left.residentBytes;
    return difference || left.displayName.localeCompare(right.displayName);
  });
}

export function performanceProcessKey(process: PerformanceProcessUsage): string {
  const applicationIdentity = process.bundleIdentifier ?? process.displayName;
  return `${applicationIdentity}:${process.pid}`;
}

export function snapshotIsStale(
  snapshot: PerformanceSnapshot | null,
  nowUnixMs: number,
): boolean {
  if (!snapshot) return false;
  return nowUnixMs - snapshot.capturedAtUnixMs > snapshot.refreshAfterMs * 3;
}
