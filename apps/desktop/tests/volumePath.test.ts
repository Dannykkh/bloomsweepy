import assert from "node:assert/strict";
import test from "node:test";
import { findVolumeForPath } from "../src/lib/volumePath.ts";

const volume = (name: string, mountPoint: string, isSystem = false) => ({
  name,
  mountPoint,
  fileSystem: "test",
  totalBytes: 1_000,
  availableBytes: 500,
  removable: false,
  readOnly: false,
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
