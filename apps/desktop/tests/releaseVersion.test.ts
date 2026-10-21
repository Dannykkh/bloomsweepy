import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (relative: string) => readFileSync(new URL(relative, import.meta.url), "utf8");
const desktop = JSON.parse(read("../package.json"));
const tauri = JSON.parse(read("../src-tauri/tauri.conf.json"));
const lock = JSON.parse(read("../package-lock.json"));

test("app, workspace packages and lockfiles share one release version", () => {
  const version = desktop.version;
  assert.match(version, /^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/);
  assert.equal(tauri.version, version);
  assert.equal(lock.version, version);
  assert.equal(lock.packages[""].version, version);
  const packages = [
    ["bloomsweepy-desktop", "../src-tauri/Cargo.toml"],
    ["bloomsweepy-mcp", "../../bloomsweepy-mcp/Cargo.toml"],
    ["bloomsweepy-core", "../../../crates/bloomsweepy-core/Cargo.toml"],
    ["bloomsweepy-control", "../../../crates/bloomsweepy-control/Cargo.toml"],
  ];
  const cargoLock = read("../../../Cargo.lock");
  for (const [name, manifest] of packages) {
    assert.equal(read(manifest).match(/^version\s*=\s*"([^"]+)"/m)?.[1], version, manifest);
    const entry = cargoLock.split("[[package]]").find((block) => block.includes(`name = "${name}"\n`));
    assert.ok(entry, `missing ${name} in Cargo.lock`);
    assert.equal(entry.match(/^version\s*=\s*"([^"]+)"/m)?.[1], version, name);
  }
});

test("native title, embedded title and MCP initialization advertise the release version", () => {
  assert.equal(tauri.app.windows[0].title, `BroomSweepy ${desktop.version}`);
  assert.equal(read("../index.html").match(/<title>([^<]+)<\/title>/)?.[1], `BroomSweepy ${desktop.version}`);
  const handler = read("../../bloomsweepy-mcp/src/mcp.rs").match(/#\[tool_handler\(([\s\S]*?)\)\]/)?.[1];
  assert.ok(handler, "MCP tool_handler metadata must exist");
  assert.equal(handler.match(/version\s*=\s*"([^"]+)"/)?.[1], desktop.version);
});
