import type { FileCatalogEntryKind } from "../types";

const directlyInspectableExtensions = new Set([
  "aac", "avi", "avif", "bmp", "csv", "doc", "docx", "epub", "flac",
  "gif", "heic", "heif", "hwp", "hwpx", "jpeg", "jpg", "key", "log",
  "m4a", "m4v", "markdown", "md", "mkv", "mobi", "mov", "mp3", "mp4",
  "mpeg", "mpg", "numbers", "odp", "ods", "odt", "ogg", "opus", "pages",
  "pdf", "png", "ppt", "pptx", "rtf", "tif", "tiff", "tsv", "txt", "wav",
  "webm", "webp", "xls", "xlsx",
]);

function finalFileExtension(path: string): string | null {
  const trimmed = path.replace(/[\\/]+$/, "");
  const finalSegment = trimmed.split(/[\\/]/).pop() ?? "";
  if (finalSegment.includes(":")) return null;
  const finalDot = finalSegment.lastIndexOf(".");
  if (finalDot <= 0 || finalDot === finalSegment.length - 1) return null;
  return finalSegment.slice(finalDot + 1).toLowerCase();
}

export type FileInspectionDecision = "open" | "reveal";

export function decideFileInspection(
  path: string,
  kind: FileCatalogEntryKind = "file",
): FileInspectionDecision {
  if (kind !== "file") return "reveal";
  const extension = finalFileExtension(path);
  return extension && directlyInspectableExtensions.has(extension) ? "open" : "reveal";
}
