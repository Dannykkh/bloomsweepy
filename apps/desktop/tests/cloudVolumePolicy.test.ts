import assert from "node:assert/strict";
import test from "node:test";
import {
  isCloudMountedVolume,
  visibleDashboardVolumes,
} from "../src/lib/cloudVolumePolicy.ts";

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
  const volume = (name: string, mountPoint: string) => ({
    name,
    mountPoint,
    fileSystem: "NTFS",
    totalBytes: 10,
    availableBytes: 5,
    removable: false,
    isSystem: mountPoint === "C:\\",
  });

  const visible = visibleDashboardVolumes([
    volume("MyData", "E:\\"),
    volume("khgudae@gmail.com - Google Drive", "I:\\"),
    volume("", "C:\\"),
    volume("Mgoon", "F:\\"),
    volume("", "D:\\"),
  ]);

  assert.deepEqual(visible.map((item) => item.mountPoint), ["C:\\", "D:\\", "E:\\", "F:\\"]);
});
