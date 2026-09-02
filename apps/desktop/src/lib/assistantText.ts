export function formatAssistantPlainText(content: string): string {
  return content
    .normalize("NFC")
    .split(/\r?\n/)
    .filter((line) => !/^\s*`?#tags:\s*.*`?\s*$/i.test(line))
    .join("\n")
    .replace(/^\s*```[^\n]*$/gm, "")
    .replace(/^\s*#{1,6}\s+/gm, "")
    .replace(/\*\*([^*\n]+)\*\*/g, "$1")
    .replace(/__([^_\n]+)__/g, "$1")
    .replace(/`([^`\n]+)`/g, "$1")
    .replace(/\n{3,}/g, "\n\n")
    .trim();
}
