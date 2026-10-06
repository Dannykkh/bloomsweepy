// Opt-in installed MCP integration. Read-only inspection and review preparation;
// there is deliberately no permission-setting, approval or execution request.
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { setTimeout as wait } from "node:timers/promises";

const [binary, expectedRoot, reviewName] = process.argv.slice(2);
assert(binary?.startsWith("/"), "Usage: node installed-common-flow.mjs /absolute/sidecar [exact-approved-test-root] [test-name-to-review]");
assert(!reviewName || expectedRoot, "Review requires an exact fixture root guard");
const child = spawn(binary, ["mcp"], { shell: false, stdio: ["pipe", "pipe", "pipe"] });
const pending = new Map();
let nextId = 0;
let buffer = "";
child.stderr.on("data", () => {}); // Never dump authentication/runtime diagnostics.
child.stdout.setEncoding("utf8");
function failPending(reason) {
  for (const { reject, timer } of pending.values()) { clearTimeout(timer); reject(reason); }
  pending.clear();
}
child.on("error", () => failPending(new Error("MCP process failed to start")));
child.on("exit", () => failPending(new Error("MCP process exited before response")));
child.stdout.on("data", chunk => {
  buffer += chunk;
  if (Buffer.byteLength(buffer) > 256 * 1024) { failPending(new Error("MCP frame budget exceeded")); child.kill(); return; }
  while (buffer.includes("\n")) {
    const split = buffer.indexOf("\n");
    const line = buffer.slice(0, split); buffer = buffer.slice(split + 1);
    if (!line.trim()) continue;
    let response;
    try { response = JSON.parse(line); } catch { failPending(new Error("Invalid MCP JSON frame")); child.kill(); return; }
    const callback = pending.get(response.id);
    if (!callback) continue;
    pending.delete(response.id); clearTimeout(callback.timer);
    if (response.error) callback.reject(new Error(`MCP error code ${response.error.code}`));
    else callback.resolve(response.result);
  }
});
function rpc(method, params = {}) {
  const id = ++nextId;
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => { pending.delete(id); reject(new Error(`MCP timeout: ${method}`)); }, 20_000);
    pending.set(id, { resolve, reject, timer });
    child.stdin.write(JSON.stringify({ jsonrpc: "2.0", id, method, params }) + "\n");
  });
}
async function tool(name, args = {}) {
  const result = await rpc("tools/call", { name, arguments: args });
  assert(!result.isError, `Tool rejected: ${name}`);
  const value = result.structuredContent ?? JSON.parse(result.content.find(item => item.type === "text").text);
  assert(!("presentation" in value), "Local presentation leaked to MCP");
  return value;
}
const app = request => tool("app_action", { request });
const files = operation => app({ kind: "file_workspace", operation });
async function finish(started) {
  assert.equal(started.status, "running");
  const id = started.data.lastOperation?.operationId;
  assert.match(id, /^[a-f0-9]{32}$/);
  for (let poll = 0; poll < 4; poll++) {
    const actual = await tool("operation_status", { operationId: id });
    assert.equal(actual.operationId, id, "Operation response identity changed");
    if (actual.state === "completed") {
      const workspace = await files({ kind: "status" });
      assert.equal(workspace.data.lastOperation?.operationId, id, "Workspace returned another operation's result");
      assert.equal(workspace.data.lastOperation?.state, "completed");
      return workspace;
    }
    assert(["queued", "running"].includes(actual.state), "Inspection did not complete");
    await wait(500);
  }
  throw new Error("Bounded inspection wait exhausted; no extra work started");
}
try {
  const initialized = await rpc("initialize", { protocolVersion: "2025-03-26", capabilities: {}, clientInfo: { name: "BroomSweepy-installed-QA", version: "1" } });
  assert(initialized.serverInfo);
  child.stdin.write(JSON.stringify({ jsonrpc: "2.0", method: "notifications/initialized" }) + "\n");
  const listed = await rpc("tools/list");
  assert.equal(listed.tools.length, 12);
  const discovery = await tool("app_capabilities");
  assert.equal(discovery.status, "completed");
  assert.equal(discovery.truncated, false);
  assert.equal(discovery.data.catalog.capabilities.length, 24);
  const detail = await app({ kind: "capability_details", capabilityId: "files.workspace" });
  assert.equal(detail.status, "completed");
  assert.equal(detail.truncated, false);
  assert.equal(detail.data.capability.id, "files.workspace");
  assert.equal(detail.data.capability.operationExamples.length, 10);
  const bytes = Buffer.byteLength(JSON.stringify(discovery));
  const detailBytes = Buffer.byteLength(JSON.stringify(detail));
  assert(bytes <= 16 * 1024);
  assert(detailBytes <= 16 * 1024);
  console.log(JSON.stringify({ phase: "discovery", tools: 12, capabilities: 24, fileOperations: 10, bytes, detailBytes }));
  const status = await tool("status");
  assert(status.bridgeAvailable);
  const initial = await files({ kind: "status" });
  console.log(JSON.stringify({ phase: "workspace_status", status: initial.status, freshScan: initial.data.workspace?.freshScan,
    selectedCount: initial.data.workspace?.selectedCount, reviewPrepared: initial.data.reviewPrepared,
    lastOutcome: initial.data.lastOperation?.outcome, movedCount: initial.data.lastOperation?.movedCount }));
  if (expectedRoot) {
    assert(status.scanAccess.root === expectedRoot, "Refusing inspection outside the exact approved fixture");
    const measured = await finish(await files({ kind: "largest" }));
    assert.equal(measured.status, "completed");
    assert.equal(measured.data.workspace.freshScan, true);
    assert.equal(measured.data.workspace.sizeRanked, true);
    const data = JSON.stringify(measured.data);
    assert(!data.includes(expectedRoot) && !data.includes('"planId"'), "Local identity leaked in model data");
    const measuredPageBytes = measured.data.workspace.entries.reduce((sum, row) => sum + (row.logicalBytes ?? 0), 0);
    console.log(JSON.stringify({ phase: "largest", entries: measured.data.workspace.totalEntries, measuredPageBytes, status: measured.status }));
    if (reviewName) {
      const reviewed = await finish(await files({ kind: "review_named", name: reviewName }));
      assert.equal(reviewed.status, "review_required");
      assert.equal(reviewed.data.reviewPrepared, true);
      assert.equal(reviewed.data.deleted, false);
      assert(!JSON.stringify(reviewed).includes(expectedRoot));
      console.log(JSON.stringify({ phase: "review", status: reviewed.status, deleted: false, localPlanExposed: false }));
    }
  }
} finally {
  child.stdin.end(); child.kill();
  failPending(new Error("QA client closed"));
  if (child.pid && child.exitCode === null && child.signalCode === null) {
    await new Promise(resolve => {
      const timer = setTimeout(() => { child.kill("SIGKILL"); resolve(); }, 2_000);
      child.once("exit", () => { clearTimeout(timer); resolve(); });
    });
  }
}
