type FormattingLanguage = "ko" | "en" | "ja" | "zh-CN";

let formattingLanguage: FormattingLanguage = "en";

function locale(): string {
  if (formattingLanguage === "ko") return "ko-KR";
  if (formattingLanguage === "ja") return "ja-JP";
  if (formattingLanguage === "zh-CN") return "zh-CN";
  return "en-US";
}

export function setFormattingLanguage(language: FormattingLanguage): void {
  formattingLanguage = language;
}

export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";

  const units = ["B", "KB", "MB", "GB", "TB", "PB"];
  const exponent = Math.min(
    Math.floor(Math.log(bytes) / Math.log(1024)),
    units.length - 1,
  );
  const value = bytes / 1024 ** exponent;
  return `${new Intl.NumberFormat(locale(), { maximumFractionDigits: 1 }).format(value)} ${units[exponent]}`;
}

export function formatDockerBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";

  const units = ["B", "kB", "MB", "GB", "TB", "PB"];
  const exponent = Math.min(
    Math.floor(Math.log(bytes) / Math.log(1_000)),
    units.length - 1,
  );
  const value = bytes / 1_000 ** exponent;
  return `${new Intl.NumberFormat(locale(), { maximumFractionDigits: 2 }).format(value)} ${units[exponent]}`;
}

export function formatCount(value: number): string {
  return new Intl.NumberFormat(locale()).format(value);
}

export function formatDate(timestamp: number | null | undefined): string {
  if (!timestamp) {
    if (formattingLanguage === "ko") return "시각 정보 없음";
    if (formattingLanguage === "ja") return "時刻情報なし";
    if (formattingLanguage === "zh-CN") return "无时间信息";
    return "Time unavailable";
  }
  return new Intl.DateTimeFormat(locale(), {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(timestamp));
}

export function formatDateTimeAttribute(
  timestamp: number | null | undefined,
): string | undefined {
  if (!timestamp) return undefined;
  return new Date(timestamp).toISOString();
}

export function formatDuration(milliseconds: number): string {
  if (milliseconds < 1_000) return `${Math.round(milliseconds)}ms`;
  if (milliseconds < 60_000) {
    if (formattingLanguage === "ko") return `${(milliseconds / 1_000).toFixed(1)}초`;
    if (formattingLanguage === "ja") return `${(milliseconds / 1_000).toFixed(1)}秒`;
    if (formattingLanguage === "zh-CN") return `${(milliseconds / 1_000).toFixed(1)}秒`;
    return `${(milliseconds / 1_000).toFixed(1)}s`;
  }
  const minutes = Math.floor(milliseconds / 60_000);
  const seconds = Math.round((milliseconds % 60_000) / 1_000);
  if (formattingLanguage === "ko") return `${minutes}분 ${seconds}초`;
  if (formattingLanguage === "ja") return `${minutes}分 ${seconds}秒`;
  if (formattingLanguage === "zh-CN") return `${minutes}分 ${seconds}秒`;
  return `${minutes}m ${seconds}s`;
}

export function fileParent(path: string): string {
  const separator = path.includes("\\") ? "\\" : "/";
  const segments = path.split(separator);
  segments.pop();
  return segments.join(separator) || separator;
}
