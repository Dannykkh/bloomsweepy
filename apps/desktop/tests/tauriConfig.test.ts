import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const manifest = readFileSync(new URL("../src-tauri/Cargo.toml", import.meta.url), "utf8");
const common = JSON.parse(readFileSync(new URL("../src-tauri/tauri.conf.json", import.meta.url), "utf8"));
const macos = JSON.parse(readFileSync(new URL("../src-tauri/tauri.macos.conf.json", import.meta.url), "utf8"));

test("shared Tauri feature declaration matches the config used by Windows cargo checks", () => {
  // tauri-build validates the common dependency declaration, not Cargo's
  // target-resolved feature set. A macOS-only JSON flag cannot satisfy it on Windows.
  const dependencies = manifest.split("[dependencies]")[1]?.split(/\n\[/)[0];
  assert.ok(dependencies, "common dependency section exists");
  const tauri = dependencies.match(/^tauri\s*=\s*\{([^\n]+)\}/m)?.[1];
  assert.ok(tauri, "common Tauri dependency exists");
  const declaresPrivateApi = tauri.includes('"macos-private-api"');
  assert.equal(Boolean(common.app.macOSPrivateApi), declaresPrivateApi);
});

test("macOS keeps native glass while other platforms keep opaque window defaults", () => {
  const macApp = { ...common.app, ...macos.app };
  assert.equal(macApp.macOSPrivateApi, true);
  assert.equal(macApp.windows[0].transparent, true);
  assert.deepEqual(macApp.windows[0].windowEffects.effects, ["underWindowBackground"]);
  assert.notEqual(common.app.windows[0].transparent, true);
  assert.equal(common.app.windows[0].windowEffects, undefined);
});
