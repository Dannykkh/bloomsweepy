export interface ApplicationInventoryEntry {
  id: string;
  displayName: string;
  displayVersion: string | null;
  publisher: string | null;
  installLocation: string | null;
  estimatedBytes: number | null;
  removalMode: "trashBundle" | "systemSettings" | "protected";
  protectionReason: string | null;
}

export interface ApplicationInventory {
  platform: "macos" | "windows" | "unsupported";
  inventoryId: string;
  applications: ApplicationInventoryEntry[];
  issues: string[];
}

export interface ApplicationDataCandidate {
  id: string;
  path: string;
  kind: "cache" | "preferences";
  evidence: string;
  estimatedBytes: number | null;
}

export interface ApplicationTrashPlan {
  planId: string;
  displayName: string;
  path: string;
  expiresAtUnixMs: number;
  relatedData: ApplicationDataCandidate[];
  warnings: string[];
}
