import { invoke } from "@tauri-apps/api/core";
import type { FileCatalogEntryKind } from "../types";

export type FileInspectionOutcome = "opened" | "revealed";

export function inspectFile(
  path: string,
  _kind: FileCatalogEntryKind = "file",
): Promise<FileInspectionOutcome> {
  // Catalog kind is a display hint, never authority to launch a file. Rust
  // checks the live path, its parents and the actual filesystem metadata.
  return invoke<FileInspectionOutcome>("inspect_local_path", { path });
}

export function revealFile(path: string): Promise<void> {
  // The generic opener reveal API canonicalizes links. Keep this inside the
  // same no-follow backend boundary as Open, including for explicit Location.
  return invoke<void>("reveal_local_path", { path });
}

/** Immediate lock shared by row activation and the visible action buttons. */
export function createInspectionGate() {
  let busy = false;
  return {
    async run<T>(operation: () => Promise<T>): Promise<{ value: T } | null> {
      if (busy) return null;
      busy = true;
      try {
        return { value: await operation() };
      } finally {
        busy = false;
      }
    },
  };
}
