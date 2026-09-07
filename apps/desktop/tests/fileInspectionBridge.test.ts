import assert from "node:assert/strict";
import test from "node:test";
import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";
import { createInspectionGate, inspectFile, revealFile } from "../src/lib/fileInspectionBridge.ts";

test("inspection forwards only the path to the metadata-validating backend", async () => {
  const requests: unknown[] = [];
  const originalWindow = Object.getOwnPropertyDescriptor(globalThis, "window");
  Object.defineProperty(globalThis, "window", { configurable: true, value: {} });
  mockIPC((command, payload) => {
    requests.push({ command, payload });
    return "revealed";
  });
  try {
    assert.equal(await inspectFile("/Fixture/link.pdf", "file"), "revealed");
    assert.equal(await inspectFile("/Fixture/Documents", "directory"), "revealed");
    await revealFile("/Fixture/cloud-target-link.pdf");
    assert.deepEqual(requests, [
      { command: "inspect_local_path", payload: { path: "/Fixture/link.pdf" } },
      { command: "inspect_local_path", payload: { path: "/Fixture/Documents" } },
      { command: "reveal_local_path", payload: { path: "/Fixture/cloud-target-link.pdf" } },
    ]);
  } finally {
    clearMocks();
    if (originalWindow) Object.defineProperty(globalThis, "window", originalWindow);
    else Reflect.deleteProperty(globalThis, "window");
  }
});

test("inspection gate prevents simultaneous row and button dispatch", async () => {
  const gate = createInspectionGate();
  let calls = 0;
  let finish!: () => void;
  const pending = gate.run(async () => {
    calls += 1;
    await new Promise<void>((resolve) => { finish = resolve; });
    return "opened";
  });
  const duplicate = await gate.run(async () => { calls += 1; return "revealed"; });
  assert.equal(duplicate, null);
  assert.equal(calls, 1);
  finish();
  assert.deepEqual(await pending, { value: "opened" });
  assert.deepEqual(await gate.run(async () => "revealed"), { value: "revealed" });
});

test("inspection gate releases after a rejected open so explicit retry works", async () => {
  const gate = createInspectionGate();
  await assert.rejects(gate.run(async () => { throw new Error("missing file"); }), /missing file/);
  assert.deepEqual(await gate.run(async () => "opened"), { value: "opened" });
});
