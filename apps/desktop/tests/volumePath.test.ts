import assert from "node:assert/strict";
import test from "node:test";
import {
  findContainingVolumeForPath,
  findVolumeForPath,
} from "../src/lib/volumePath.ts";

const volume = (name: string, mountPoint: string, isSystem = false) => ({
  name,
  mountPoint,
  fileSystem: "test",
  totalBytes: 1_000,
  availableBytes: 500,
  removable: false,
  readOnly: false,
  isDiskImage: false,
  isSystem,
});

test("matches Windows drive paths and extended path prefixes", () => {
  const volumes = [volume("system", "C:\\", true), volume("data", "D:\\")];
  assert.equal(findVolumeForPath(volumes, "d:\\git\\repo")?.name, "data");
  assert.equal(findVolumeForPath(volumes, "\\\\?\\D:\\git\\repo")?.name, "data");
});

test("uses mount boundaries for similarly named macOS volumes", () => {
  const volumes = [
    volume("Macintosh HD", "/", true),
    volume("Data", "/Volumes/Data"),
    volume("Data 2", "/Volumes/Data2"),
  ];
  assert.equal(findVolumeForPath(volumes, "/Volumes/Data2/project")?.name, "Data 2");
  assert.equal(findVolumeForPath(volumes, "/Volumes/Data/project")?.name, "Data");
});

test("can require a strict containing volume without the system fallback", () => {
  const visibleVolumes = [volume("system", "C:\\", true)];
  assert.equal(findContainingVolumeForPath(visibleVolumes, "G:\\Cloud\\file.txt"), null);
  assert.equal(findVolumeForPath(visibleVolumes, "G:\\Cloud\\file.txt")?.name, "system");
});
