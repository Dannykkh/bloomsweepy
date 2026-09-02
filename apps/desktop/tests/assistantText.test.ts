import assert from "node:assert/strict";
import test from "node:test";
import { formatAssistantPlainText } from "../src/lib/assistantText.ts";

test("removes provider markdown decoration without losing the message", () => {
  assert.equal(
    formatAssistantPlainText("## 결과\n\n- **빌드 캐시**: `21.1GB`"),
    "결과\n\n- 빌드 캐시: 21.1GB",
  );
});

test("removes provider metadata tags from the visible conversation", () => {
  assert.equal(
    formatAssistantPlainText("볼륨은 정리하지 않습니다.\n\n`#tags: docker, cleanup`"),
    "볼륨은 정리하지 않습니다.",
  );
});
