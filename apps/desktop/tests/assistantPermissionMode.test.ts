import assert from "node:assert/strict";
import test from "node:test";
import {
  ASSISTANT_PERMISSION_MODE_OPTIONS,
  canAutomaticallyTrashMode,
  canReviewInMode,
  isAssistantPermissionMode,
  normalizeAssistantPermissionMode,
} from "../src/lib/assistantPermissionMode.ts";

test("only explicit known permission modes are accepted", () => {
  for (const mode of ["read_only", "confirm", "full_access"] as const) {
    assert.equal(isAssistantPermissionMode(mode), true);
    assert.equal(normalizeAssistantPermissionMode(mode), mode);
  }
  assert.deepEqual(ASSISTANT_PERMISSION_MODE_OPTIONS.map(option => option.mode), ["read_only", "confirm", "full_access"]);
});

test("missing and malformed modes never display full access", () => {
  for (const value of [undefined, null, false, true, 1, "", "fullAccess", "FULL_ACCESS", " full_access ",
    "read-only", "__proto__", { mode: "full_access" }, ["full_access"]]) {
    assert.equal(isAssistantPermissionMode(value), false);
    assert.equal(normalizeAssistantPermissionMode(value), "confirm");
  }
});

test("read only forbids review and automatic trash even with a remembered legacy flag", () => {
  assert.equal(canReviewInMode("read_only"), false);
  assert.equal(canAutomaticallyTrashMode("read_only", false), false);
  assert.equal(canAutomaticallyTrashMode("read_only", true), false);
});

test("explicit confirm always requires confirmation even with a remembered legacy flag", () => {
  assert.equal(canReviewInMode("confirm"), true);
  assert.equal(canAutomaticallyTrashMode("confirm"), false);
  assert.equal(canAutomaticallyTrashMode("confirm", false), false);
  assert.equal(canAutomaticallyTrashMode("confirm", true), false);
});

test("only legacy hosts with no mode field retain the legacy skip-confirmation flag", () => {
  assert.equal(canReviewInMode(undefined), true);
  assert.equal(canAutomaticallyTrashMode(undefined), false);
  assert.equal(canAutomaticallyTrashMode(undefined, false), false);
  assert.equal(canAutomaticallyTrashMode(undefined, true), true);
  assert.equal(normalizeAssistantPermissionMode(undefined), "confirm");
});

test("explicit full access permits review and automatic trash independently of the legacy flag", () => {
  assert.equal(canReviewInMode("full_access"), true);
  assert.equal(canAutomaticallyTrashMode("full_access"), true);
  assert.equal(canAutomaticallyTrashMode("full_access", false), true);
  assert.equal(canAutomaticallyTrashMode("full_access", true), true);
});

test("an unknown runtime mode fails closed for review and automatic trash", () => {
  for (const mode of [null, "FULL_ACCESS", " full_access ", "unknown", 1, { mode: "full_access" }]) {
    assert.equal(canReviewInMode(mode), false);
    assert.equal(canAutomaticallyTrashMode(mode, false), false);
    assert.equal(canAutomaticallyTrashMode(mode, true), false);
  }
});
