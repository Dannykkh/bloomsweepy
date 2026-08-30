const byteFormatter = new Intl.NumberFormat("ko-KR", {
  maximumFractionDigits: 1,
});

const countFormatter = new Intl.NumberFormat("ko-KR");

const dateFormatter = new Intl.DateTimeFormat("ko-KR", {
  month: "short",
  day: "numeric",
  hour: "2-digit",
  minute: "2-digit",
});

export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";

  const units = ["B", "KB", "MB", "GB", "TB", "PB"];
  const exponent = Math.min(
    Math.floor(Math.log(bytes) / Math.log(1024)),
    units.length - 1,
  );
  const value = bytes / 1024 ** exponent;
  return `${byteFormatter.format(value)} ${units[exponent]}`;
}

export function formatCount(value: number): string {
  return countFormatter.format(value);
}

export function formatDate(timestamp: number | null | undefined): string {
  if (!timestamp) return "시각 정보 없음";
  return dateFormatter.format(new Date(timestamp));
}

export function formatDateTimeAttribute(
  timestamp: number | null | undefined,
): string | undefined {
  if (!timestamp) return undefined;
  return new Date(timestamp).toISOString();
}

export function formatDuration(milliseconds: number): string {
  if (milliseconds < 1_000) return `${Math.round(milliseconds)}ms`;
  if (milliseconds < 60_000) return `${(milliseconds / 1_000).toFixed(1)}초`;
  return `${Math.floor(milliseconds / 60_000)}분 ${Math.round(
    (milliseconds % 60_000) / 1_000,
  )}초`;
}

export function fileParent(path: string): string {
  const separator = path.includes("\\") ? "\\" : "/";
  const segments = path.split(separator);
  segments.pop();
  return segments.join(separator) || separator;
}
