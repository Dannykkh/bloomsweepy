import { invoke } from "@tauri-apps/api/core";
import type { TrashOperationResult } from "../types";
import type { ApplicationInventory, ApplicationTrashPlan } from "./applicationTypes";

export function getApplicationInventory(): Promise<ApplicationInventory> {
  return invoke("get_application_inventory");
}

export function prepareApplicationTrash(inventoryId: string, applicationId: string): Promise<ApplicationTrashPlan> {
  return invoke("prepare_application_trash", { request: { inventoryId, applicationId } });
}

export function confirmApplicationTrash(planId: string): Promise<TrashOperationResult> {
  return invoke("confirm_application_trash", {
    request: { planId, bundleOnlyAcknowledged: true, noUninstallerAcknowledged: true },
  });
}

export function prepareApplicationDataTrash(inventoryId: string, applicationId: string, candidateIds: string[]): Promise<ApplicationTrashPlan> {
  return invoke("prepare_application_data_trash", { request: { inventoryId, applicationId, candidateIds } });
}

export function confirmApplicationDataTrash(planId: string): Promise<TrashOperationResult> {
  return invoke("confirm_application_data_trash", { request: { planId, relatedDataAcknowledged: true } });
}

export function dismissApplicationPlan(planId: string): Promise<void> {
  return invoke("dismiss_application_plan", { planId });
}

export function openApplicationUninstallSettings(): Promise<void> {
  return invoke("open_application_uninstall_settings");
}
