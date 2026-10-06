import assert from "node:assert/strict";
import test from "node:test";
import {
  ASSISTANT_MODEL_PREFERENCE_KEY,
  ASSISTANT_MODEL_ID_MAX_LENGTH,
  ASSISTANT_PREFERENCE_MAX_BYTES,
  ASSISTANT_REASONING_CHOICE_LIMIT,
  ASSISTANT_REASONING_EFFORTS,
  LEGACY_ASSISTANT_PROVIDER_KEY,
  LEGACY_OLLAMA_MODEL_KEY,
  assistantModelRequestValue,
  assistantModelSelection,
  assistantReasoningStatus,
  assistantReasoningEffortFor,
  emptyAssistantModelPreference,
  isAssistantProviderKind,
  normalizeAssistantModelId,
  normalizeAssistantReasoningEffort,
  parseAssistantModelPreference,
  readAssistantModelPreference,
  withAssistantModel,
  withAssistantProvider,
  withAssistantReasoningEffort,
  writeAssistantModelPreference,
} from "../src/lib/assistantModelPreference.ts";
import type { AssistantModelPreference } from "../src/lib/assistantModelPreference.ts";
import type { AssistantProviderStatus } from "../src/types.ts";

function storage(initial: Record<string, string> = {}) {
  const values = new Map(Object.entries(initial));
  return { values, getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => { values.set(key, value); } };
}

const provider: AssistantProviderStatus = {
  provider: "codex", label: "Codex", installed: true, authentication: "authenticated", available: true,
  busy: false, detail: "Synthetic provider", models: [{ id: "test-model", label: "Test model" }], state: "ready",
  executablePath: null, version: null, modelSelection: "optional", modelCatalogSource: "bundled",
};

test("model preferences retain distinct provider models without persisting conversation or permissions", () => {
  let preference = withAssistantModel(emptyAssistantModelPreference(), "codex", "gpt-6.1-sol");
  preference = withAssistantModel(withAssistantProvider(preference, "claudeCode"), "claudeCode", "sonnet");
  preference = withAssistantModel(withAssistantProvider(preference, "ollama"), "ollama", "qwen3:8b");
  assert.equal(preference.models.codex, "gpt-6.1-sol");
  assert.equal(preference.models.claudeCode, "sonnet");
  assert.equal(preference.models.ollama, "qwen3:8b");
  assert.equal(withAssistantProvider(preference, "codex").models.codex, "gpt-6.1-sol");
  const local = storage();
  assert.equal(writeAssistantModelPreference(local, preference), true);
  const restored = readAssistantModelPreference(local);
  assert.deepEqual(restored, { preference, storageError: false, hasSavedProvider: true });
  assert.deepEqual(Object.keys(JSON.parse(local.values.get(ASSISTANT_MODEL_PREFERENCE_KEY)!)), ["version", "provider", "models"]);
});

test("legacy provider and Ollama preferences migrate without exporting them to another provider", () => {
  const local = storage({ [LEGACY_ASSISTANT_PROVIDER_KEY]: "ollama", [LEGACY_OLLAMA_MODEL_KEY]: "qwen3:8b" });
  const old = readAssistantModelPreference(local);
  assert.equal(old.preference.provider, "ollama");
  assert.equal(old.preference.models.ollama, "qwen3:8b");
  assert.equal(old.preference.models.codex, undefined);
  assert.equal(old.hasSavedProvider, true);
  assert.equal(local.values.has(ASSISTANT_MODEL_PREFERENCE_KEY), false);
  assert.equal(writeAssistantModelPreference(local, withAssistantProvider(old.preference, "codex")), true);
  assert.equal(readAssistantModelPreference(local).preference.provider, "codex");
  assert.equal(local.values.get(LEGACY_ASSISTANT_PROVIDER_KEY), "ollama");
});

test("schema wins over stale legacy settings and drops unknown fields and provider keys", () => {
  const raw = JSON.stringify({ version: 1, provider: "claudeCode", models: { claudeCode: "sonnet", codex: "test-model", unknown: "private-data", ollama: "bad model" }, conversation: "not stored", approved: true });
  const parsed = parseAssistantModelPreference(raw)!;
  assert.deepEqual(parsed, { version: 1, provider: "claudeCode", models: { codex: "test-model", claudeCode: "sonnet" } });
  assert.deepEqual(readAssistantModelPreference(storage({ [ASSISTANT_MODEL_PREFERENCE_KEY]: raw, [LEGACY_ASSISTANT_PROVIDER_KEY]: "ollama" })).preference, parsed);
  const local = storage();
  assert.equal(writeAssistantModelPreference(local, JSON.parse(raw) as AssistantModelPreference), true);
  assert.equal(local.values.get(ASSISTANT_MODEL_PREFERENCE_KEY)?.includes("private-data"), false);
  assert.equal(local.values.get(ASSISTANT_MODEL_PREFERENCE_KEY)?.includes("not stored"), false);
});

test("bounded model ids reject options, control characters and unknown input", () => {
  for (const id of ["", "gpt-6.1-sol", "sonnet", "qwen3:8b", "org/model-v1.2", "a".repeat(ASSISTANT_MODEL_ID_MAX_LENGTH)]) assert.equal(normalizeAssistantModelId(id), id);
  for (const id of [null, 1, [], "--help", "-m", "model\nname", "model name", "a,b", "a;exec", "$(id)", "a".repeat(ASSISTANT_MODEL_ID_MAX_LENGTH + 1)]) assert.equal(normalizeAssistantModelId(id), null);
  for (const kind of [null, "", "unknown", "Codex", "__proto__"]) assert.equal(isAssistantProviderKind(kind), false);
  const original = emptyAssistantModelPreference();
  assert.equal(withAssistantModel(original, "codex", "--help"), original);
});

test("malformed and oversized storage cannot replace a valid legacy preference", () => {
  for (const raw of ["{", "[]", "null", JSON.stringify({ version: 2, provider: "codex", models: {} }), JSON.stringify({ version: 1, provider: "other", models: {} }), JSON.stringify({ version: 1, provider: "codex", models: [] }), "x".repeat(ASSISTANT_PREFERENCE_MAX_BYTES + 1)]) {
    assert.equal(parseAssistantModelPreference(raw), null);
    assert.equal(readAssistantModelPreference(storage({ [ASSISTANT_MODEL_PREFERENCE_KEY]: raw, [LEGACY_ASSISTANT_PROVIDER_KEY]: "claudeCode" })).preference.provider, "claudeCode");
  }
});

test("unavailable or failing storage reports failure without mutating the current selection", () => {
  assert.equal(readAssistantModelPreference({ getItem: () => { throw new Error("Storage blocked"); } }).storageError, true);
  const preference = withAssistantModel(emptyAssistantModelPreference(), "codex", "test-model");
  const snapshot = JSON.stringify(preference);
  assert.equal(writeAssistantModelPreference({ setItem: () => { throw new Error("Quota exceeded"); } }, preference), false);
  assert.equal(JSON.stringify(preference), snapshot);
  assert.equal(readAssistantModelPreference(storage()).hasSavedProvider, false);
});

test("blank optional model means native CLI default; selected models are passed without catalogue fallback", () => {
  assert.equal(assistantModelRequestValue(provider, ""), null);
  assert.equal(assistantModelRequestValue(provider, "test-model"), "test-model");
  assert.equal(assistantModelRequestValue(provider, "model-no-longer-listed"), "model-no-longer-listed");
  assert.equal(assistantModelRequestValue({ ...provider, provider: "claudeCode" }, "sonnet"), "sonnet");
  assert.equal(assistantModelRequestValue({ ...provider, provider: "ollama", modelSelection: "required" }, "qwen3:8b"), "qwen3:8b");
  assert.equal(assistantModelRequestValue({ ...provider, modelSelection: "unsupported" }, "test-model"), null);
  assert.equal(assistantModelRequestValue(provider, "--help"), null);
});

test("legacy host remains model-required only for Ollama", () => {
  const legacy = { ...provider, modelSelection: undefined };
  assert.equal(assistantModelSelection(legacy), "unsupported");
  assert.equal(assistantModelSelection({ ...legacy, provider: "ollama" }), "required");
  assert.equal(assistantModelSelection(null), "unsupported");
  assert.equal(assistantModelRequestValue(legacy, "test-model"), null);
});

test("legacy model preference migration leaves the chosen model and default effort untouched", () => {
  const old = { version: 1, provider: "codex", models: { codex: "prior-model", ollama: "qwen3:8b" } };
  const restored = readAssistantModelPreference(storage({ [ASSISTANT_MODEL_PREFERENCE_KEY]: JSON.stringify(old) }));
  assert.deepEqual(restored.preference, old);
  assert.equal(assistantReasoningEffortFor(restored.preference, "codex", "prior-model"), "");
  const next = withAssistantReasoningEffort(restored.preference, "codex", "prior-model", "medium");
  assert.deepEqual(next.models, old.models, "setting effort must not silently upgrade the model");
});

test("reasoning choices are independent for each provider and model and survive reload", () => {
  let preference = withAssistantModel(emptyAssistantModelPreference(), "codex", "first-model");
  preference = withAssistantReasoningEffort(preference, "codex", "first-model", "high");
  preference = withAssistantReasoningEffort(preference, "codex", "second-model", "low");
  preference = withAssistantReasoningEffort(preference, "claudeCode", "first-model", "medium");
  assert.equal(assistantReasoningEffortFor(preference, "codex", "first-model"), "high");
  assert.equal(assistantReasoningEffortFor(preference, "codex", "second-model"), "low");
  assert.equal(assistantReasoningEffortFor(preference, "claudeCode", "first-model"), "medium");
  assert.equal(assistantReasoningEffortFor(preference, "ollama", "first-model"), "");
  const local = storage();
  assert.equal(writeAssistantModelPreference(local, preference), true);
  assert.deepEqual(readAssistantModelPreference(local).preference, preference);
  const cleared = withAssistantReasoningEffort(preference, "codex", "first-model", "");
  assert.equal(assistantReasoningEffortFor(cleared, "codex", "first-model"), "");
  assert.equal(assistantReasoningEffortFor(cleared, "codex", "second-model"), "low");
  assert.equal(assistantReasoningEffortFor(preference, "codex", "first-model"), "high", "updates never mutate old snapshots");
});

test("effort whitelist rejects options, unknown values, control characters and default-model keys", () => {
  for (const effort of ASSISTANT_REASONING_EFFORTS) assert.equal(normalizeAssistantReasoningEffort(effort), effort);
  assert.equal(normalizeAssistantReasoningEffort(""), "");
  const preference = emptyAssistantModelPreference();
  for (const effort of [null, true, 3, [], "HIGH", "high ", "high\n", "--help", "turbo", "a".repeat(500)]) assert.equal(normalizeAssistantReasoningEffort(effort), null);
  assert.equal(withAssistantReasoningEffort(preference, "codex", "model", "invalid"), preference);
  assert.equal(withAssistantReasoningEffort(preference, "codex", "", "high"), preference);
  assert.equal(withAssistantReasoningEffort(preference, "codex", "__proto__", "high"), preference);
  assert.equal(assistantReasoningEffortFor(preference, "codex", "constructor"), "");
});

test("reasoning storage is bounded and retains the current model and newest choice", () => {
  let preference = withAssistantModel(emptyAssistantModelPreference(), "codex", "active-model");
  preference = withAssistantReasoningEffort(preference, "codex", "active-model", "high");
  for (let index = 0; index < 30; index++) preference = withAssistantReasoningEffort(preference, "codex", `model-${index}`, "medium");
  assert.equal(Object.keys(preference.reasoningEfforts?.codex ?? {}).length, ASSISTANT_REASONING_CHOICE_LIMIT);
  assert.equal(assistantReasoningEffortFor(preference, "codex", "active-model"), "high");
  assert.equal(assistantReasoningEffortFor(preference, "codex", "model-29"), "medium");
  assert.equal(preference.models.codex, "active-model");
  assert.equal(writeAssistantModelPreference(storage(), preference), true);
});

test("bounded storage parsing prioritizes the active model instead of silently dropping its effort", () => {
  const parsed = parseAssistantModelPreference(JSON.stringify({ version: 1, provider: "claudeCode", models: { claudeCode: "active-model" },
    reasoningEfforts: { codex: Object.fromEntries(Array.from({ length: 20 }, (_, index) => [`model-${index}`, "medium"])),
      claudeCode: { "active-model": "high" } } }))!;
  assert.equal(assistantReasoningEffortFor(parsed, "claudeCode", "active-model"), "high");
  assert.equal(Object.values(parsed.reasoningEfforts ?? {}).reduce((count, choices) => count + Object.keys(choices).length, 0), ASSISTANT_REASONING_CHOICE_LIMIT);
});

test("hostile or malformed reasoning storage cannot corrupt the saved provider or model", () => {
  const base = { version: 1, provider: "codex", models: { codex: "prior-model" } };
  const parsed = parseAssistantModelPreference(JSON.stringify({ ...base, reasoningEfforts: {
    codex: { "prior-model": "high", "bad-effort": "execute", "bad model": "high", "": "low", "__proto__": "high" },
    claudeCode: [], unknownProvider: { "prior-model": "ultra" },
  } }))!;
  assert.deepEqual(parsed.models, base.models);
  assert.deepEqual(parsed.reasoningEfforts, { codex: { "prior-model": "high" } });
  for (const invalid of [[], null, "high"]) assert.deepEqual(parseAssistantModelPreference(JSON.stringify({ ...base, reasoningEfforts: invalid })), base);
});

const reasoningProvider: AssistantProviderStatus = { ...provider,
  models: [{ id: "first-model", label: "First", supportedReasoningEfforts: ["low", "medium", "high", "high", "ultra", "invalid"], defaultReasoningEffort: "low" },
    { id: "second-model", label: "Second", supportedReasoningEfforts: ["low", "medium", "high", "max"], defaultReasoningEffort: "medium" }],
};

test("reasoning options and displayed defaults come from the explicitly selected model", () => {
  const first = assistantReasoningStatus(reasoningProvider, "first-model", "ultra");
  assert.deepEqual(first.supportedEfforts, ["low", "medium", "high", "ultra"]);
  assert.equal(first.defaultEffort, "low");
  assert.equal(first.requestValue, "ultra");
  assert.equal(first.stale, false);
  const second = assistantReasoningStatus(reasoningProvider, "second-model", "ultra");
  assert.equal(second.defaultEffort, "medium");
  assert.equal(second.supportedEfforts.includes("ultra"), false);
  assert.equal(second.stale, true);
  assert.equal(second.requestValue, null);
  assert.equal(assistantReasoningStatus(reasoningProvider, "first-model", "").requestValue, null, "catalog default is a label, not an override");
});

test("CLI default and unsupported providers never inherit an explicit effort", () => {
  assert.equal(assistantReasoningStatus(reasoningProvider, "", "").mode, "modelDefault");
  assert.equal(assistantReasoningStatus(reasoningProvider, "", "high").requestValue, null);
  assert.equal(assistantReasoningStatus({ ...reasoningProvider, provider: "claudeCode" }, "first-model", "high").mode, "unsupported");
  assert.equal(assistantReasoningStatus({ ...reasoningProvider, provider: "ollama" }, "first-model", "high").requestValue, null);
});

test("catalog failure or lost support preserves the choice and blocks sending until deliberate reset", () => {
  const preference = withAssistantReasoningEffort(withAssistantModel(emptyAssistantModelPreference(), "codex", "first-model"), "codex", "first-model", "ultra");
  const value = assistantReasoningEffortFor(preference, "codex", "first-model");
  for (const unavailable of [{ ...reasoningProvider, modelCatalogSource: "unavailable" as const, models: [] },
    { ...reasoningProvider, models: [{ id: "first-model", label: "First" }] }]) {
    const status = assistantReasoningStatus(unavailable, "first-model", value);
    assert.equal(status.stale, true);
    assert.equal(status.requestValue, null);
    assert.equal(assistantReasoningEffortFor(preference, "codex", "first-model"), "ultra");
    assert.equal(assistantReasoningStatus(unavailable, "first-model", "").stale, false);
  }
  const reset = withAssistantReasoningEffort(preference, "codex", "first-model", "");
  assert.equal(reset.models.codex, "first-model");
  assert.equal(assistantReasoningEffortFor(reset, "codex", "first-model"), "");
});

test("old-host missing reasoning metadata stays on defaults and cannot silently use a saved effort", () => {
  const legacy = { ...provider, modelSelection: undefined };
  const defaults = assistantReasoningStatus(legacy, "test-model", "");
  assert.equal(defaults.mode, "unsupported");
  assert.equal(defaults.stale, false);
  assert.equal(defaults.requestValue, null);
  assert.equal(assistantReasoningStatus(legacy, "test-model", "high").stale, true);
});

test("reasoning write failure does not mutate or discard the current local choice", () => {
  const preference = withAssistantReasoningEffort(withAssistantModel(emptyAssistantModelPreference(), "codex", "first-model"), "codex", "first-model", "medium");
  assert.equal(writeAssistantModelPreference({ setItem: () => { throw new Error("Storage blocked"); } }, preference), false);
  assert.equal(assistantReasoningEffortFor(preference, "codex", "first-model"), "medium");
  assert.equal(preference.models.codex, "first-model");
});
