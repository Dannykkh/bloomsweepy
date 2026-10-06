import assert from "node:assert/strict";
import test from "node:test";
import { assistantModelCatalogPresentation, reasoningEffortAtIndex, reasoningSliderIndex } from "../src/lib/assistantComposerOptions.ts";
import type { AssistantProviderStatus } from "../src/types.ts";

const catalogProvider: AssistantProviderStatus = {
  provider: "grok", label: "Grok", installed: false, authentication: "unknown",
  available: false, busy: false, detail: "", models: [], modelSelection: "optional",
  modelCatalogSource: "unavailable", state: "notInstalled", executablePath: null, version: null,
};

test("missing Grok and Antigravity show installation need, not an empty CLI default", () => {
  for (const provider of ["grok", "antigravity"] as const) {
    const shown = assistantModelCatalogPresentation({ ...catalogProvider, provider });
    assert.equal(shown.placeholder, "CLI 설치 필요");
    assert.match(shown.detail!, /설치한 뒤/);
    assert.ok(shown.documentationUrl?.startsWith("https://"));
    assert.deepEqual(catalogProvider.models, []);
  }
});

test("empty catalog separates login, launch, service, no-model and discovery failures", () => {
  const expected = {
    loginRequired: "로그인 후 목록 확인", broken: "CLI 확인 필요", incompatible: "CLI 확인 필요",
    checkFailed: "CLI 확인 필요", serviceUnavailable: "서비스 연결 필요", noModels: "설치된 모델 없음", ready: "모델 목록 확인 필요",
  } as const;
  for (const [state, placeholder] of Object.entries(expected)) {
    const shown = assistantModelCatalogPresentation({ ...catalogProvider, installed: true, state: state as AssistantProviderStatus["state"] });
    assert.equal(shown.placeholder, placeholder);
    assert.equal(shown.unavailable, true);
  }
});

test("genuine signed-out metadata remains selectable and unsupported ready hosts retain default contract", () => {
  const signedOut = assistantModelCatalogPresentation({ ...catalogProvider, provider: "claudeCode", installed: true,
    state: "loginRequired", models: [{ id: "opus", label: "Opus 5.5" }] });
  assert.equal(signedOut.unavailable, false);
  assert.equal(signedOut.placeholder, null);
  const unsupported = assistantModelCatalogPresentation({ ...catalogProvider, installed: true, state: "ready", modelSelection: "unsupported" });
  assert.equal(unsupported.unavailable, false);
  assert.equal(assistantModelCatalogPresentation({ ...catalogProvider, modelSelection: "unsupported" }).placeholder, "CLI 설치 필요");
});

test("official examples remain separate from actual model and reasoning catalogs", () => {
  const grok = assistantModelCatalogPresentation(catalogProvider);
  assert.deepEqual(grok.referenceModels.map(model => model.id), ["grok-build", "grok-4.7"]);
  assert.equal(grok.referenceUrl, "https://docs.x.ai/build/settings");
  assert.ok(grok.referenceModels.every(model => !("supportedReasoningEfforts" in model)));
  const agy = assistantModelCatalogPresentation({ ...catalogProvider, provider: "antigravity" });
  assert.equal(agy.referenceModels.length, 7);
  assert.equal(agy.referenceModels[0].id, "gemini-3.8-flash-high");
  assert.equal(agy.referenceModels[6].id, "gemini-3.1-pro-high");
  assert.equal(catalogProvider.models.length, 0);
  const actual = assistantModelCatalogPresentation({ ...catalogProvider, installed: true, available: true, state: "ready",
    models: [{ id: "custom-actual", label: "Actual CLI model" }] });
  assert.equal(actual.unavailable, false);
  assert.deepEqual(actual.referenceModels, []);
});

test("slider follows actual supported list order and skips unavailable levels", () => {
  const efforts = ["low", "high", "ultra"] as const;
  assert.equal(reasoningSliderIndex(efforts, "high", "low"), 1);
  assert.equal(reasoningEffortAtIndex(efforts, 1), "high");
  assert.equal(reasoningEffortAtIndex(efforts, 2), "ultra");
});

test("CLI default has a metadata display position but remains distinct from explicit none", () => {
  const efforts = ["none", "low", "medium"] as const;
  assert.equal(reasoningSliderIndex(efforts, "", "medium"), 2);
  assert.equal(reasoningSliderIndex(efforts, "none", "medium"), 0);
  assert.equal(reasoningEffortAtIndex(efforts, 0), "none");
  assert.equal(reasoningSliderIndex(efforts, "", null), null);
});

test("stale explicit effort never silently falls back to a valid model default", () => {
  assert.equal(reasoningSliderIndex(["low", "high"], "ultra", "low"), null);
  assert.equal(reasoningSliderIndex(["low", "high"], "unknown", "high"), null);
  assert.equal(reasoningSliderIndex(["low", "high"], "", "medium"), null);
});

test("one-step and empty catalogs do not fabricate additional slider choices", () => {
  assert.equal(reasoningSliderIndex(["high"], "", "high"), 0);
  assert.equal(reasoningEffortAtIndex(["high"], 0), "high");
  assert.equal(reasoningEffortAtIndex(["high"], 1), null);
  assert.equal(reasoningSliderIndex([], "", null), null);
  assert.equal(reasoningEffortAtIndex([], 0), null);
});

test("invalid slider indices never clamp or coerce into a supported value", () => {
  for (const index of [-1, 0.5, 3, NaN, Infinity, -Infinity]) {
    assert.equal(reasoningEffortAtIndex(["low", "medium", "high"], index), null);
  }
});
