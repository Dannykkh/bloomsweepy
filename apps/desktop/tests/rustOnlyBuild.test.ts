import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(new URL(path, import.meta.url), "utf8");

test("Mac packaging has no dependency on the retired Swift project", () => {
  const workflow = read("../../../.github/workflows/macos-check.yml");
  assert.doesNotMatch(workflow, /xcodegen|project\.yml|\.swift\b|swift --version|xcodebuild\s+\\?\s*-project/i);
  assert.match(workflow, /xcrun --sdk macosx/);
  assert.match(workflow, /cargo test --workspace/);
  assert.match(workflow, /npx tauri build --bundles app/);
  assert.match(workflow, /--check-heap-budget/);
  assert.match(workflow, /hdiutil verify/);
});

test("Tauri owns its icons and entitlements without legacy Swift resources", () => {
  const config = JSON.parse(read("../src-tauri/tauri.conf.json"));
  const inputs: string[] = [...config.bundle.icon, config.bundle.macOS.entitlements];
  assert.ok(inputs.length > 1);
  for (const path of inputs) {
    assert.doesNotMatch(path, /(?:^|\/)BroomSweepy\//);
    assert.ok(existsSync(new URL(`../src-tauri/${path}`, import.meta.url)), `missing packaging input: ${path}`);
  }
});
