import assert from "node:assert/strict";
import test from "node:test";
import { assistantFailureMessage, assistantProviderStatusKey, isAssistantAuthenticationFailure } from "../src/lib/assistantProviderStatus.ts";
import type { AssistantProviderStatus } from "../src/types.ts";

const base: AssistantProviderStatus = {
  provider: "codex", label: "Codex", installed: true, authentication: "unknown",
  available: false, busy: false, detail: "", models: [], state: "checkFailed",
  executablePath: "/fixture/codex", version: null,
};

test("launch and compatibility errors never look like a login request", () => {
  for (const state of ["broken", "incompatible", "checkFailed", "serviceUnavailable"] as const) {
    const label = assistantProviderStatusKey({ ...base, state, authentication: "required" });
    assert.ok(!label.includes("로그인"));
    assert.ok(!label.includes("설치 안 됨"));
  }
});

test("missing, signed-out and ready states have distinct labels", () => {
  assert.match(assistantProviderStatusKey({ ...base, state: "notInstalled" }), /설치 안 됨/);
  assert.match(assistantProviderStatusKey({ ...base, state: "loginRequired" }), /로그인 필요/);
  assert.match(assistantProviderStatusKey({ ...base, state: "ready", authentication: "authenticated" }), /CLI 준비됨/);
  assert.match(assistantProviderStatusKey({ ...base, provider: "ollama", state: "noModels" }), /모델 없음/);
});

test("structured server auth failures preserve actionable text and update login state", () => {
  const failure = { kind: "authentication", message: "Claude Code에서 다시 로그인해 주세요" };
  assert.equal(assistantFailureMessage(failure), failure.message);
  assert.equal(isAssistantAuthenticationFailure(failure), true);
  assert.equal(isAssistantAuthenticationFailure({ kind: "other", message: "Network error" }), false);
  assert.equal(assistantFailureMessage("legacy error"), "legacy error");
  assert.equal(assistantFailureMessage({ unexpected: true }), null);
});
