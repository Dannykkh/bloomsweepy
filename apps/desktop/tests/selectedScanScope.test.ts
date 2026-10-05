import assert from "node:assert/strict";
import test from "node:test";
import { bindSelectedScanScope } from "../src/lib/selectedScanScope.ts";
import { DEFAULT_SCAN_CONFIG } from "../src/types.ts";
import type { ScanConfig } from "../src/types.ts";

function recorder(results: boolean[] = []) {
  const calls: [string | null, ScanConfig | null][] = [];
  return { calls, update: async (root: string | null, config: ScanConfig | null) => {
    calls.push([root, config]);
    return results.shift() ?? true;
  } };
}

test("selecting a folder binds its exact scope and current config without a permission toggle", async () => {
  const io = recorder();
  assert.equal(await bindSelectedScanScope(null, "/fixture/a", DEFAULT_SCAN_CONFIG, false, true, io.update), true);
  assert.deepEqual(io.calls, [["/fixture/a", DEFAULT_SCAN_CONFIG]]);
});

test("changing folders revokes the old scope before binding the new scope", async () => {
  const io = recorder();
  assert.equal(await bindSelectedScanScope("/fixture/a", "/fixture/b", DEFAULT_SCAN_CONFIG, true, true, io.update), true);
  assert.deepEqual(io.calls, [[null, null], ["/fixture/b", DEFAULT_SCAN_CONFIG]]);
});

test("failed old-scope revocation blocks a folder switch and never grants the new scope", async () => {
  const io = recorder([false]);
  assert.equal(await bindSelectedScanScope("/fixture/a", "/fixture/b", DEFAULT_SCAN_CONFIG, true, true, io.update), false);
  assert.deepEqual(io.calls, [[null, null]]);
});

test("failed new binding does not block local inspection after old scope was revoked", async () => {
  const io = recorder([true, false]);
  assert.equal(await bindSelectedScanScope("/fixture/a", "/fixture/b", DEFAULT_SCAN_CONFIG, true, true, io.update), true);
  assert.deepEqual(io.calls, [[null, null], ["/fixture/b", DEFAULT_SCAN_CONFIG]]);
});

test("local folder selection works when no external bridge exists", async () => {
  const io = recorder();
  assert.equal(await bindSelectedScanScope(null, "/fixture/a", DEFAULT_SCAN_CONFIG, false, false, io.update), true);
  assert.deepEqual(io.calls, []);
});

test("selecting the same folder retries a missing external binding", async () => {
  const io = recorder();
  assert.equal(await bindSelectedScanScope("/fixture/a", "/fixture/a", DEFAULT_SCAN_CONFIG, false, true, io.update), true);
  assert.deepEqual(io.calls, [["/fixture/a", DEFAULT_SCAN_CONFIG]]);
});

test("selecting the same bound folder makes no redundant persisted grant", async () => {
  const io = recorder();
  assert.equal(await bindSelectedScanScope("/fixture/a", "/fixture/a", DEFAULT_SCAN_CONFIG, true, true, io.update), true);
  assert.deepEqual(io.calls, []);
});
