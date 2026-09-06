import assert from "node:assert/strict";
import test from "node:test";
import {
  clampPercent,
  memoryUsagePercent,
  memoryCleanupReleasedBytes,
  performanceProcessKey,
  snapshotIsStale,
  sortPerformanceProcesses,
} from "../src/lib/performancePresentation.ts";
import type { PerformanceProcessUsage, PerformanceSnapshot } from "../src/types.ts";

function process(
  displayName: string,
  cpuMachinePercent: number,
  residentBytes: number,
): PerformanceProcessUsage {
  return {
    targetId: null,
    displayName,
    bundleIdentifier: null,
    pid: residentBytes,
    kind: "userProcess",
    cpuCorePercent: cpuMachinePercent,
    cpuMachinePercent,
    residentBytes,
    processCount: 1,
    canRequestTermination: false,
    terminationEligibility: "unsupported",
  };
}

function snapshot(): PerformanceSnapshot {
  return {
    snapshotId: "snapshot",
    capturedAtUnixMs: 1_000,
    sampleWindowMs: 2_000,
    refreshAfterMs: 2_000,
    platform: "macos",
    logicalCpuCount: 8,
    cpuUsagePercent: 25,
    memory: {
      totalBytes: 1_000,
      availableBytes: 250,
      usedBytes: 750,
      totalSwapBytes: 100,
      usedSwapBytes: 10,
    },
    processes: [],
    processesTruncated: false,
    capabilities: {
      processMetricsAvailable: true,
      gracefulTerminationAvailable: true,
      appMemoryCleanupAvailable: true,
      processScope: "guiApplications",
    },
  };
}

test("percent presentation rejects non-finite values and clamps the range", () => {
  assert.equal(clampPercent(Number.NaN), 0);
  assert.equal(clampPercent(-1), 0);
  assert.equal(clampPercent(120), 100);
});

test("memory usage uses the runtime used and total values", () => {
  assert.equal(memoryUsagePercent(snapshot()), 75);
  assert.equal(memoryUsagePercent(null), 0);
});

test("memory cleanup only presents a positive allocator report as released bytes", () => {
  assert.equal(memoryCleanupReleasedBytes(4_096), true);
  assert.equal(memoryCleanupReleasedBytes(0), false);
  assert.equal(memoryCleanupReleasedBytes(-1), false);
  assert.equal(memoryCleanupReleasedBytes(Number.NaN), false);
});

test("process sort switches between actual CPU and resident memory", () => {
  const rows = [process("CPU", 80, 10), process("RAM", 5, 100)];
  assert.equal(sortPerformanceProcesses(rows, "cpu")[0]?.displayName, "CPU");
  assert.equal(sortPerformanceProcesses(rows, "memory")[0]?.displayName, "RAM");
  assert.equal(rows[0]?.displayName, "CPU", "sorting must not mutate the snapshot");
});

test("snapshot freshness allows three refresh windows", () => {
  const value = snapshot();
  assert.equal(snapshotIsStale(value, 6_999), false);
  assert.equal(snapshotIsStale(value, 7_001), true);
});

test("process row keys stay stable across snapshots but change for another app", () => {
  const first = process("Editor", 10, 42);
  first.pid = 900;
  first.bundleIdentifier = "com.example.editor";
  first.targetId = "snapshot-token-one";
  const next = { ...first, targetId: "snapshot-token-two", cpuMachinePercent: 20 };
  const reusedPid = { ...next, bundleIdentifier: "com.example.other" };

  assert.equal(performanceProcessKey(first), performanceProcessKey(next));
  assert.notEqual(performanceProcessKey(first), performanceProcessKey(reusedPid));
});
