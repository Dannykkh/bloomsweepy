import assert from "node:assert/strict";
import test from "node:test";
import {
  dashboardVolumeKey,
  isCloudMountedVolume,
  resolveDashboardVolume,
  visibleDashboardVolumes,
} from "../src/lib/cloudVolumePolicy.ts";

const volume = (
  name: string,
  mountPoint: string,
  options: Partial<{
    isSystem: boolean;
    removable: boolean;
    readOnly: boolean;
    isDiskImage: boolean;
  }> = {},
) => ({
  name,
  mountPoint,
  fileSystem: "test",
  totalBytes: 1_000,
  availableBytes: 500,
  removable: options.removable ?? false,
  readOnly: options.readOnly ?? false,
  isDiskImage: options.isDiskImage ?? false,
  isSystem: options.isSystem ?? false,
});

test("hides every Google Drive volume, including Windows-truncated labels", () => {
  const volumes = [
    { name: "happyguy81@gmail.com - Google...", mountPoint: "G:\\" },
    { name: "khgudae@gmail.com - Google Drive", mountPoint: "I:\\" },
    { name: "MyData", mountPoint: "E:\\" },
  ];

  assert.deepEqual(
    volumes.filter((volume) => !isCloudMountedVolume(volume)),
    [{ name: "MyData", mountPoint: "E:\\" }],
  );
});

test("hides common cloud mounts without hiding physical drive labels", () => {
  for (const name of ["OneDrive", "Dropbox", "iCloud Drive", "pCloud", "GoogleDriveFS"]) {
    assert.equal(isCloudMountedVolume({ name, mountPoint: "Z:\\" }), true);
  }

  assert.equal(isCloudMountedVolume({ name: "Google archive", mountPoint: "D:\\" }), false);
  assert.equal(isCloudMountedVolume({ name: "Mgoon", mountPoint: "F:\\" }), false);
});

test("sorts visible Windows drives by drive letter after filtering", () => {
  const visible = visibleDashboardVolumes([
    volume("MyData", "E:\\"),
    volume("khgudae@gmail.com - Google Drive", "I:\\"),
    volume("", "C:\\", { isSystem: true }),
    volume("Mgoon", "F:\\"),
    volume("", "D:\\"),
  ]);

  assert.deepEqual(visible.map((item) => item.mountPoint), ["C:\\", "D:\\", "E:\\", "F:\\"]);
});

test("keeps an explicit physical-drive selection across Windows path variants", () => {
  const volumes = [
    volume("Windows", "C:\\", { isSystem: true }),
    volume("Projects", "D:\\"),
  ];

  assert.equal(resolveDashboardVolume(volumes, "d:\\")?.name, "Projects");
  assert.equal(resolveDashboardVolume(volumes, "\\\\?\\D:\\")?.name, "Projects");
  assert.equal(dashboardVolumeKey({ mountPoint: "\\\\?\\D:\\" }), "d:");
});

test("keeps case-sensitive POSIX mount identities distinct", () => {
  assert.equal(
    dashboardVolumeKey({ mountPoint: "/Volumes/Data/" }),
    "/Volumes/Data",
  );
  assert.notEqual(
    dashboardVolumeKey({ mountPoint: "/Volumes/Data" }),
    dashboardVolumeKey({ mountPoint: "/Volumes/data" }),
  );
});

test("falls back to the system drive when the selected external drive disappears", () => {
  const volumes = [
    volume("Macintosh HD", "/", { isSystem: true }),
    volume("Archive", "/Volumes/Archive", { removable: true }),
  ];

  assert.equal(
    resolveDashboardVolume(volumes, "/Volumes/Missing")?.name,
    "Macintosh HD",
  );
});

test("uses a conventional system mount or fixed writable drive when flags are missing", () => {
  assert.equal(
    resolveDashboardVolume([
      volume("External", "/Volumes/External", { removable: true }),
      volume("Root", "/"),
    ], null)?.name,
    "Root",
  );
  assert.equal(
    resolveDashboardVolume([
      volume("Read only", "D:\\", { readOnly: true }),
      volume("Data", "E:\\"),
    ], null)?.name,
    "Data",
  );
});

test("never resolves a hidden cloud mount as the dashboard drive", () => {
  assert.equal(
    resolveDashboardVolume([
      volume("Google Drive", "G:\\", { isSystem: true }),
      volume("Local", "C:\\"),
    ], "G:\\")?.name,
    "Local",
  );
  assert.equal(resolveDashboardVolume([], null), null);
});

test("shows user-facing macOS drives without internal system mounts", () => {
  const visible = visibleDashboardVolumes([
    volume("Macintosh HD", "/", { isSystem: true }),
    volume("Data", "/System/Volumes/Data"),
    volume("VM", "/System/Volumes/VM"),
    volume("Simulator", "/Library/Developer/CoreSimulator/Volumes/iOS"),
    volume("Archive", "/Volumes/Archive", { removable: true }),
  ], "macos");

  assert.deepEqual(
    visible.map((item) => item.mountPoint),
    ["/", "/Volumes/Archive"],
  );
});

test("hides mounted macOS disk images without hiding read-only physical media", () => {
  const visible = visibleDashboardVolumes([
    volume("Macintosh HD", "/", { isSystem: true }),
    volume("ChatGPT Installer", "/Volumes/ChatGPT Installer", {
      removable: true,
      readOnly: true,
      isDiskImage: true,
    }),
    volume("Workspace Image", "/Volumes/Workspace Image", {
      removable: true,
      isDiskImage: true,
    }),
    volume("Archive", "/Volumes/Archive", {
      removable: true,
      readOnly: true,
    }),
  ], "macos");

  assert.deepEqual(
    visible.map((item) => item.mountPoint),
    ["/", "/Volumes/Archive"],
  );
});
