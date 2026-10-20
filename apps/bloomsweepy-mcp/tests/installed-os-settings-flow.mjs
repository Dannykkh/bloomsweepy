// Opt-in installed-app MCP probe. Only discovery/details and two read-only OS
// inspections are sent. Never change consent, launch settings, or invoke file tools.
import { spawn } from "node:child_process";
import { isAbsolute, normalize } from "node:path";
import { isDeepStrictEqual } from "node:util";

const MAX_STDOUT_BYTES = 256 * 1024;
const MAX_RESULT_BYTES = 16 * 1024;
const TIMEOUT_MS = 20_000;
const expectedTools = ["app_action", "app_capabilities", "cancel_operation", "cleanup_candidates", "cleanup_plan_status", "create_cleanup_plan", "operation_status", "search_documents", "search_files", "start_storage_scan", "status", "system_overview"];
const finderIds = ["show_extensions", "show_hidden_files", "show_path_bar", "show_status_bar", "folders_first"];
const finderStates = new Set(["value", "unset", "managed", "typeMismatch", "error", "unsupported"]);
const privateKeys = new Set(["presentation", "registryPath", "registryKey", "registryView", "targets", "uri", "diagnostic", "diagnostics", "localDiagnostics", "warnings", "history", "domain", "key", "path"]);
const pending = new Map();
let child;
let childClosed;
let childCloseObserved = false;
let deadlineTimer;
let closing = false;
let fatalError;
let nextId = 0;
let buffer = "";
let stdoutBytes = 0;
let stderrBytes = 0;
let deadlineAt = 0;
let phase = "guard";

function failure(code) { return Object.assign(new Error(code), { safeCode: code }); }
function check(condition, code) { if (!condition) throw failure(code); }
function object(value) { return value !== null && typeof value === "object" && !Array.isArray(value); }
function exactKeys(value, keys, code) {
  check(object(value), code);
  check(Object.keys(value).sort().join("\n") === [...keys].sort().join("\n"), code);
}
function noLocalEvidence(value) {
  if (typeof value === "string") {
    check(!/(?:ms-settings:|HKEY_(?:CURRENT_USER|LOCAL_MACHINE)|\bHK(?:CU|LM)\\|\/(?:Users|private|Volumes)\/)/i.test(value), "local_identity_or_uri_exposed");
    return;
  }
  if (Array.isArray(value)) { for (const item of value) noLocalEvidence(item); return; }
  if (!object(value)) return;
  for (const [key, item] of Object.entries(value)) {
    check(!privateKeys.has(key), "local_evidence_exposed");
    noLocalEvidence(item);
  }
}
function failPending(reason) {
  for (const callback of pending.values()) { clearTimeout(callback.timer); callback.reject(reason); }
  pending.clear();
}
function fatal(code) {
  fatalError ??= failure(code);
  failPending(fatalError);
  if (child && !closing) child.kill();
}

function start(binary) {
  deadlineAt = Date.now() + TIMEOUT_MS;
  child = spawn(binary, ["mcp"], { shell: false, stdio: ["pipe", "pipe", "pipe"] });
  childClosed = new Promise((resolve) => child.once("close", () => { childCloseObserved = true; resolve(); }));
  deadlineTimer = setTimeout(() => fatal("total_timeout"), TIMEOUT_MS);
  child.on("error", () => fatal("sidecar_start_failed"));
  child.on("exit", () => { if (!closing) fatal("sidecar_exited_before_completion"); });
  child.stdin.on("error", () => { if (!closing) fatal("sidecar_input_closed"); });
  child.stderr.on("data", (chunk) => {
    // Consume private authentication/runtime diagnostics without retaining/logging them.
    stderrBytes += chunk.length;
    if (stderrBytes > MAX_STDOUT_BYTES) fatal("sidecar_diagnostic_budget_exceeded");
  });
  child.stdout.setEncoding("utf8");
  child.stdout.on("data", (chunk) => {
    stdoutBytes += Buffer.byteLength(chunk);
    if (stdoutBytes > MAX_STDOUT_BYTES) { fatal("stdout_budget_exceeded"); return; }
    buffer += chunk;
    while (buffer.includes("\n")) {
      const split = buffer.indexOf("\n");
      const line = buffer.slice(0, split); buffer = buffer.slice(split + 1);
      if (!line.trim()) continue;
      let response;
      try { response = JSON.parse(line); } catch { fatal("invalid_json_frame"); return; }
      if (!object(response) || response.jsonrpc !== "2.0") { fatal("invalid_rpc_frame"); return; }
      const callback = pending.get(response.id);
      if (!callback) {
        if (response.id !== undefined) { fatal("unexpected_response_id"); return; }
        continue;
      }
      pending.delete(response.id); clearTimeout(callback.timer);
      if (response.error) callback.reject(failure("rpc_error"));
      else if (!("result" in response)) callback.reject(failure("missing_rpc_result"));
      else callback.resolve(response.result);
    }
  });
}

function rpc(method, params = {}) {
  if (fatalError) return Promise.reject(fatalError);
  const id = ++nextId;
  return new Promise((resolve, reject) => {
    const remaining = deadlineAt - Date.now();
    if (remaining <= 0) { reject(failure("total_timeout")); return; }
    const timer = setTimeout(() => { pending.delete(id); reject(failure("request_timeout")); }, remaining);
    pending.set(id, { resolve, reject, timer });
    child.stdin.write(JSON.stringify({ jsonrpc: "2.0", id, method, params }) + "\n", (error) => {
      if (error) fatal("sidecar_input_failed");
    });
  });
}

async function tool(name, args = {}) {
  const result = await rpc("tools/call", { name, arguments: args });
  check(object(result) && !result.isError, "tool_rejected");
  check(Array.isArray(result.content), "missing_tool_content");
  const textValues = [];
  for (const content of result.content ?? []) {
    check(content.type === "text" && typeof content.text === "string", "unexpected_tool_content");
    try { textValues.push(JSON.parse(content.text)); } catch { throw failure("invalid_tool_json"); }
  }
  const value = result.structuredContent ?? textValues[0];
  check(object(value), "missing_tool_value");
  for (const exposed of [value, ...textValues]) {
    noLocalEvidence(exposed);
    check(Buffer.byteLength(JSON.stringify(exposed)) <= MAX_RESULT_BYTES, "tool_result_budget_exceeded");
    exactKeys(exposed, ["capability", "source", "status", "capturedAtUnixMs", "data", "truncated"], "unexpected_result_envelope");
    check(exposed.source === "broomsweepy" && exposed.truncated === false, "invalid_result_source_or_truncation");
    check(Number.isSafeInteger(exposed.capturedAtUnixMs) && exposed.capturedAtUnixMs > 0, "invalid_capture_timestamp");
    check(isDeepStrictEqual(exposed, value), "text_and_structured_projection_differ");
  }
  return value;
}
const app = (request) => tool("app_action", { request });

function blocked(result, capability) {
  if (result.status !== "permission_required") return;
  check(result.capability === capability && result.data?.performed === false, "invalid_permission_result");
  check(!Object.prototype.hasOwnProperty.call(result.data, "settings"), "inspection_data_exposed_without_permission");
  throw Object.assign(failure("external_inspection_not_allowed"), { blocked: true, capability });
}

function validateFinder(result) {
  check(result.capability === "macos.inspect", "wrong_macos_capability");
  check(result.status === "completed" && result.data?.supported === true, "macos_inspection_not_supported");
  const data = result.data;
  exactKeys(data, ["supported", "platform", "settings", "performed", "scope", "mutation", "storedPreferencesOnly"], "unexpected_macos_projection");
  check(data.performed === false && data.storedPreferencesOnly === true, "macos_readonly_semantics_missing");
  check(typeof data.platform === "string" && data.platform.length <= 160, "invalid_macos_platform");
  check(Array.isArray(data.settings) && data.settings.length === 5, "wrong_finder_row_count");
  const ids = new Set();
  for (const row of data.settings) {
    exactKeys(row, ["id", "title", "value", "state", "managed"], "unexpected_finder_row_fields");
    check(finderIds.includes(row.id) && !ids.has(row.id), "unexpected_finder_id"); ids.add(row.id);
    check(typeof row.title === "string" && row.title.length <= 240 && finderStates.has(row.state), "invalid_finder_state");
    check(typeof row.managed === "boolean", "invalid_finder_managed_state");
    if (row.state === "value") check(typeof row.value === "boolean" && !row.managed, "invalid_finder_boolean");
    else if (row.state === "managed") check(row.managed && (row.value === null || typeof row.value === "boolean"), "invalid_finder_managed_value");
    else check(row.value === null && !row.managed, "finder_absence_or_error_not_preserved");
  }
}

function validateUnsupportedWindows(result) {
  check(result.capability === "windows.inspect" && result.status === "unsupported", "windows_unsupported_status_missing");
  const data = result.data;
  exactKeys(data, ["supported", "platform", "osBuild", "osRevision", "edition", "settings", "performed", "storedValuesOnly", "effectiveStateKnown", "scope", "mutation"], "unexpected_windows_projection");
  check(data.supported === false && data.platform === "macos", "wrong_windows_host_support");
  check(Array.isArray(data.settings) && data.settings.length === 0, "windows_unsupported_rows_not_empty");
  check(data.performed === false && data.effectiveStateKnown === false && data.storedValuesOnly === true, "windows_readonly_semantics_missing");
  check(data.osBuild === null && data.osRevision === null && data.edition === null, "windows_metadata_guessed_on_mac");
}

async function run() {
  const [binary, ...extra] = process.argv.slice(2);
  check(process.platform === "darwin" && process.arch === "arm64", "requires_darwin_arm64_host");
  check(typeof binary === "string" && isAbsolute(binary) && normalize(binary) === binary
    && binary !== "/" && !binary.endsWith("/") && !/[\u0000-\u001f\u007f]/.test(binary) && extra.length === 0, "absolute_sidecar_argument_required");
  phase = "initialize";
  start(binary);
  const initialized = await rpc("initialize", { protocolVersion: "2025-03-26", capabilities: {}, clientInfo: { name: "BroomSweepy-installed-OS-readonly-QA", version: "1" } });
  check(object(initialized?.serverInfo), "initialize_failed");
  child.stdin.write(JSON.stringify({ jsonrpc: "2.0", method: "notifications/initialized" }) + "\n");
  phase = "tools_list";
  const listed = await rpc("tools/list");
  check(Array.isArray(listed?.tools) && listed.tools.map((item) => item.name).sort().join("\n") === [...expectedTools].sort().join("\n"), "unexpected_tool_inventory");
  phase = "discovery";
  const discovery = await tool("app_capabilities");
  check(discovery.capability === "capabilities" && discovery.status === "completed", "discovery_incomplete");
  const catalog = discovery.data?.catalog;
  check(object(catalog) && catalog.discoveryFormatVersion === 2 && catalog.detailsRequired === true, "wrong_discovery_format");
  check(Array.isArray(catalog.capabilities) && catalog.capabilities.length === 27, "wrong_capability_count");
  check(new Set(catalog.capabilities.map((entry) => entry.id)).size === 27 && catalog.capabilities.every((entry) => entry.finalExecution === false), "invalid_capability_inventory");
  check(typeof discovery.data.externalInspectionAllowed === "boolean", "unknown_inspection_permission");
  const permissionAtDiscovery = discovery.data.externalInspectionAllowed;
  const detailBytes = [];
  for (const [capabilityId, kind] of [["macos.inspect", "macos_inspect"], ["windows.inspect", "windows_inspect"]]) {
    phase = "capability_details";
    const entry = catalog.capabilities.find((item) => item.id === capabilityId);
    check(entry?.requiresInspectionAccess === true && JSON.stringify(entry.requestExample) === JSON.stringify({ kind }), "inspection_entry_missing");
    const detail = await app({ kind: "capability_details", capabilityId });
    check(detail.capability === "capabilities.details" && detail.status === "completed" && detail.data?.detailFormatVersion === 1, "detail_lookup_failed");
    check(detail.data.capability?.id === capabilityId && detail.data.capability.finalExecution === false && detail.data.capability.requiresInspectionAccess === true, "detail_contract_mismatch");
    check(JSON.stringify(detail.data.capability.requestExample) === JSON.stringify({ kind }), "detail_request_changed");
    detailBytes.push(Buffer.byteLength(JSON.stringify(detail)));
  }
  console.log(JSON.stringify({ phase: "discovery", status: "validated", tools: expectedTools.length, capabilities: 27, discoveryFormatVersion: 2, bytes: Buffer.byteLength(JSON.stringify(discovery)), detailBytes }));
  phase = "macos_inspect";
  const macos = await app({ kind: "macos_inspect" });
  if (!permissionAtDiscovery) check(macos.status === "permission_required", "inspection_permission_boundary_bypassed");
  blocked(macos, "macos.inspect");
  validateFinder(macos);
  phase = "windows_inspect";
  const windows = await app({ kind: "windows_inspect" });
  blocked(windows, "windows.inspect");
  validateUnsupportedWindows(windows);
  if (fatalError) throw fatalError;
  return { phase: "os_settings", status: "readonly_contract_validated", macos: { status: macos.status, rows: 5, stateAndNullEncodingValidated: true }, windows: { status: windows.status, rows: 0, performed: false, effectiveStateKnown: false }, localEvidenceExposed: false, permissionsChanged: false, settingsLaunchRequested: false, osChangesRequested: false, finderValuesLogged: false, windowsRuntimeVerified: false };
}

async function close() {
  if (closing) return;
  closing = true;
  clearTimeout(deadlineTimer);
  failPending(failure("qa_client_closed"));
  if (!child) return;
  let timer;
  try {
    child.stdin.end();
    if (!childCloseObserved) {
      if (child.pid && child.exitCode === null && child.signalCode === null) child.kill();
      const closed = await Promise.race([childClosed.then(() => true), new Promise((resolve) => {
        timer = setTimeout(() => resolve(false), 2_000);
      })]);
      if (!closed) {
        fatalError ??= failure("sidecar_close_timeout");
        child.kill("SIGKILL");
      }
    }
  } finally {
    clearTimeout(timer);
    if (buffer.trim()) fatalError ??= failure("incomplete_json_frame");
    child.stdin.destroy(); child.stdout.destroy(); child.stderr.destroy();
    child.unref();
  }
}

try {
  const summary = await run();
  await close();
  if (fatalError) throw fatalError;
  console.log(JSON.stringify(summary));
} catch (error) {
  await close();
  process.exitCode = error?.blocked ? 2 : 1;
  console.error(JSON.stringify({ phase, status: error?.blocked ? "blocked" : "failed", reason: error?.safeCode ?? "qa_harness_error", ...(error?.blocked ? { capability: error.capability, inspectionCompleted: false, permissionChangeRequested: false } : {}), finderValuesLogged: false }));
}
