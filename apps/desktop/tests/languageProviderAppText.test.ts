import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import test from "node:test";
import { createContext, isValidElement } from "react";
import ts from "typescript";
import * as formatting from "../src/lib/format.ts";
import * as appTextModule from "../src/lib/localizeAppText.ts";
import * as preferenceModule from "../src/i18n/preference.ts";
import type { LanguagePreference, ResolvedLanguage } from "../src/i18n/preference.ts";

const require = createRequire(import.meta.url);
const source = readFileSync(new URL("../src/i18n/index.tsx", import.meta.url), "utf8");
const compiled = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX,
    target: ts.ScriptTarget.ES2022, esModuleInterop: false },
}).outputText;
const japanese = JSON.parse(readFileSync(new URL("../src/i18n/ja.json", import.meta.url), "utf8"));
const chinese = JSON.parse(readFileSync(new URL("../src/i18n/zh-CN.json", import.meta.url), "utf8"));
const languages = ["ko", "en", "ja", "zh-CN", "ko"] as const;

interface ContextValue {
  language: ResolvedLanguage;
  preference: LanguagePreference;
  storageError: boolean;
  setPreference: (language: LanguagePreference) => void;
  t: (key: string, values?: Record<string, string | number>) => string;
  appText: (message: string) => string;
}
interface CachedHook { dependencies: unknown[]; value?: unknown }
const sameDependencies = (left: unknown[], right: unknown[]) =>
  left.length === right.length && left.every((value, index) => Object.is(value, right[index]));

// Run the real provider and real catalogs/helper, without mounting React or a
// native app. Only its hooks, storage and native language notification are fakes.
function createHarness(initial: LanguagePreference = "ko") {
  const slots: unknown[] = [];
  let cursor = 0;
  let pendingEffects: (() => void)[] = [];
  let currentValue: ContextValue;
  const nativeLanguages: string[] = [];
  const storageReads: string[] = [];
  const storageWrites: [string, string][] = [];
  const stored = new Map<string, string>([[preferenceModule.LANGUAGE_STORAGE_KEY, initial]]);
  const document = { documentElement: { lang: "uninitialized" } };
  function memo(compute: () => unknown, dependencies: unknown[]) {
    const index = cursor++;
    const cached = slots[index] as CachedHook | undefined;
    if (!cached || !sameDependencies(cached.dependencies, dependencies)) {
      slots[index] = { dependencies, value: compute() };
    }
    return (slots[index] as CachedHook).value;
  }
  const module = { exports: {} as {
    LanguageProvider?: (props: { children: unknown }) => unknown;
    useLanguage?: () => ContextValue;
  } };
  new Function("require", "module", "exports", "window", "document", compiled)((id: string) => {
    if (id === "react/jsx-runtime") return require(id);
    if (id === "react") return {
      createContext,
      useContext: () => currentValue,
      useState: (initialValue: unknown) => {
        const index = cursor++;
        if (!(index in slots)) slots[index] = typeof initialValue === "function" ? initialValue() : initialValue;
        return [slots[index], (next: unknown) => {
          slots[index] = typeof next === "function" ? next(slots[index]) : next;
        }];
      },
      useMemo: memo,
      useCallback: (callback: unknown, dependencies: unknown[]) => memo(() => callback, dependencies),
      useEffect: (effect: () => void, dependencies: unknown[]) => {
        const index = cursor++;
        const cached = slots[index] as CachedHook | undefined;
        if (!cached || !sameDependencies(cached.dependencies, dependencies)) pendingEffects.push(effect);
        slots[index] = { dependencies };
      },
    };
    if (id === "../lib/format") return formatting;
    if (id === "../lib/localizeAppText") return appTextModule;
    if (id === "./preference") return preferenceModule;
    if (id === "./ja.json") return { default: japanese };
    if (id === "./zh-CN.json") return { default: chinese };
    if (id === "../lib/bridge") return {
      setApplicationLanguage: (language: string) => {
        nativeLanguages.push(language);
        return Promise.resolve();
      },
    };
    assert.fail(`Unexpected dependency; native reads and subscriptions are forbidden: ${id}`);
  }, module, module.exports, {
    localStorage: {
      getItem: (key: string) => { storageReads.push(key); return stored.get(key) ?? null; },
      setItem: (key: string, value: string) => { storageWrites.push([key, value]); stored.set(key, value); },
    },
  }, document);
  assert.ok(module.exports.LanguageProvider && module.exports.useLanguage);
  function render() {
    cursor = 0; pendingEffects = [];
    const tree = module.exports.LanguageProvider!({ children: null });
    assert.ok(isValidElement<{ value: ContextValue }>(tree));
    currentValue = tree.props.value;
    for (const effect of pendingEffects) effect();
    assert.strictEqual(module.exports.useLanguage!(), currentValue);
    return currentValue;
  }
  let value = render();
  return {
    document, nativeLanguages, storageReads, storageWrites, stored,
    current: () => value,
    render: () => { value = render(); return value; },
    change: (language: LanguagePreference) => { value.setPreference(language); value = render(); return value; },
  };
}

test("the actual provider retranslates one frozen backend snapshot through KO→EN→JA→ZH→KO without a new query", () => {
  const warning = "저장되지 않은 값은 macOS 기본값으로 추측하지 않습니다. 변경 사항은 Finder 창을 다시 열거나 Finder를 다시 시작한 뒤 반영될 수 있습니다. 앱은 Finder를 자동으로 다시 시작하지 않습니다.";
  const protection = "다른 사용자가 소유한 관련 데이터는 보호됩니다";
  const snapshot = Object.freeze({ warnings: Object.freeze([warning]), protectionReason: protection });
  const expected = {
    ko: { warning: /^저장되지 않은 값은/, protection },
    en: { warning: /^Unset values are not inferred/, protection: "Related data owned by another user is protected" },
    ja: { warning: /^未設定値から/, protection: "別のユーザーが所有する関連データは保護されています" },
    "zh-CN": { warning: /^不会根据 macOS/, protection: "其他用户拥有的相关数据受保护" },
  };
  const duration = { ko: "1.5초", en: "1.5s", ja: "1.5秒", "zh-CN": "1.5秒" };
  const harness = createHarness();
  for (const [index, language] of languages.entries()) {
    const value = index === 0 ? harness.current() : harness.change(language);
    assert.equal(value.language, language);
    assert.match(value.appText(snapshot.warnings[0]), expected[language].warning);
    assert.equal(value.appText(snapshot.protectionReason), expected[language].protection);
    assert.equal(harness.document.documentElement.lang, language);
    assert.equal(formatting.formatDuration(1_500), duration[language]);
  }
  assert.deepEqual(harness.nativeLanguages, languages, "Only native language notifications may run");
  assert.deepEqual(harness.storageReads, [preferenceModule.LANGUAGE_STORAGE_KEY], "Preference is read only at mount");
  assert.deepEqual(harness.storageWrites, languages.slice(1).map(language => [preferenceModule.LANGUAGE_STORAGE_KEY, language]));
  assert.deepEqual(snapshot, { warnings: [warning], protectionReason: protection });
  harness.render();
  assert.deepEqual(harness.nativeLanguages, languages, "An ordinary rerender must not resend native language changes");
});

test("actual catalog notices recorded in an earlier language use the provider's reverse lookup", () => {
  const harness = createHarness();
  const key = "설정 상태를 새로 확인했습니다.";
  const previousNotices = [key, "Preference status refreshed.", japanese[key], chinese[key]];
  assert.equal(new Set(previousNotices).size, 4);
  for (const language of languages) {
    const value = harness.change(language);
    const expected = value.t(key);
    for (const notice of previousNotices) assert.equal(value.appText(notice), expected);
  }
  assert.deepEqual(harness.nativeLanguages, languages);
});

test("provider appText preserves unknown diagnostics, Korean paths and non-catalog user prose in every language", () => {
  const harness = createHarness();
  const opaque = Object.freeze([
    "/Users/사용자 캐시/파일 확장자 표시.txt",
    "C:\\사용자 자료\\보고서.txt",
    "사용자가 입력한 고유한 질문입니다: 내 프로젝트는 어떻게 정리할까요?",
    "모델이 자유롭게 작성한 별도 응답입니다. 원문을 보존합니다.",
    "미등록 진단 원문: /Users/테스트/$1/$&/새 폴더\n보고서.txt",
    "__proto__", "constructor", "", " 사용자 캐시 ",
  ]);
  for (const language of languages) {
    const value = harness.change(language);
    for (const text of opaque) assert.equal(value.appText(text), text);
  }
  assert.deepEqual(harness.nativeLanguages, languages);
});
