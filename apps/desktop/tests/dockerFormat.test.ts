import assert from "node:assert/strict";
import test from "node:test";
import { formatDockerBytes } from "../src/lib/format.ts";

test("uses Docker decimal units instead of file-system binary units", () => {
  assert.equal(formatDockerBytes(71_780_000_000), "71.78 GB");
  assert.equal(formatDockerBytes(8_102_000), "8.1 MB");
});
