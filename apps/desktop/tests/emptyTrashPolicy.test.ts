import assert from "node:assert/strict";
import test from "node:test";
import { canConfirmEmptyTrash, emptyTrashErrorMessage, emptyTrashOutcomeMessages } from "../src/lib/emptyTrashPolicy.ts";

test("empty Trash requires a live plan, acknowledgment and an unused submission", () => {
  const plan = { id: "fixture", expiresAtUnixMs: 2000 };
  assert.equal(canConfirmEmptyTrash(null, true, false, 1000), false);
  assert.equal(canConfirmEmptyTrash(plan, false, false, 1000), false);
  assert.equal(canConfirmEmptyTrash(plan, true, true, 1000), false);
  assert.equal(canConfirmEmptyTrash(plan, true, false, 2000), false);
  assert.equal(canConfirmEmptyTrash(plan, true, false, 1000), true);
  assert.equal(canConfirmEmptyTrash({ ...plan, expiresAtUnixMs: NaN }, true, false, 1000), false);
});

test("invalid plans require a new review and uncertain outcomes require inspection", () => {
  assert.equal(emptyTrashErrorMessage("expired"), emptyTrashErrorMessage("invalidPlan"));
  assert.equal(emptyTrashErrorMessage("needsInspection"), emptyTrashOutcomeMessages.unconfirmed);
  assert.notEqual(emptyTrashErrorMessage("unavailable"), emptyTrashOutcomeMessages.requested);
});

test("outcomes never claim recovered bytes or rollback after OS cancellation", () => {
  assert.match(emptyTrashOutcomeMessages.requested, /요청했습니다/);
  assert.match(emptyTrashOutcomeMessages.requested, /보장하지 않습니다/);
  assert.match(emptyTrashOutcomeMessages.cancelled, /이미 삭제됐을 수/);
  assert.match(emptyTrashOutcomeMessages.unconfirmed, /계속될 수/);
});
