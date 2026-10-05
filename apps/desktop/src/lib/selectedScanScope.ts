import type { ScanConfig } from "../types";

type UpdateScope = (root: string | null, config: ScanConfig | null) => Promise<boolean>;

// Called only after a folder is selected in the app. This binds read-only
// requests to that folder, not arbitrary CLI paths or destructive operations.
export async function bindSelectedScanScope(
  currentRoot: string | null,
  selectedRoot: string,
  config: ScanConfig,
  enabled: boolean,
  bridgeAvailable: boolean,
  update: UpdateScope,
): Promise<boolean> {
  if (currentRoot === selectedRoot && enabled) return true;
  if (currentRoot !== selectedRoot && enabled && !await update(null, null)) {
    return false; // Never leave an old external scope active while switching folders.
  }
  if (bridgeAvailable) await update(selectedRoot, config);
  // An external bridge/save failure is surfaced by update(), but does not
  // prevent local inspection after the old scope has been safely revoked.
  return true;
}
