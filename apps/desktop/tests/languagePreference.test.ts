import assert from "node:assert/strict";
import test from "node:test";
import {
  LANGUAGE_STORAGE_KEY,
  normalizeLanguagePreference,
} from "../src/i18n/preference.ts";

test("English is the default for missing and unsupported preferences", () => {
  assert.equal(normalizeLanguagePreference(null), "en");
  assert.equal(normalizeLanguagePreference(undefined), "en");
  assert.equal(normalizeLanguagePreference("system"), "en");
  assert.equal(normalizeLanguagePreference("zh-TW"), "en");
});

test("the four supported language preferences remain stable", () => {
  assert.equal(normalizeLanguagePreference("en"), "en");
  assert.equal(normalizeLanguagePreference("ko"), "ko");
  assert.equal(normalizeLanguagePreference("ja"), "ja");
  assert.equal(normalizeLanguagePreference("zh-CN"), "zh-CN");
  assert.equal(LANGUAGE_STORAGE_KEY, "bloomsweepy.ui-language.v1");
});
