export type LanguagePreference = "en" | "ko" | "ja" | "zh-CN";
export type ResolvedLanguage = LanguagePreference;

export const LANGUAGE_STORAGE_KEY = "bloomsweepy.ui-language.v1";

export function normalizeLanguagePreference(value: unknown): LanguagePreference {
  return value === "ko" || value === "ja" || value === "zh-CN" ? value : "en";
}
