import assert from "node:assert/strict";
import test from "node:test";
import { appTextCatalog, localizeAppText } from "../src/lib/localizeAppText.ts";

const languages = ["ko", "en", "ja", "zh-CN"] as const;
const macWarning = "저장되지 않은 값은 macOS 기본값으로 추측하지 않습니다. 변경 사항은 Finder 창을 다시 열거나 Finder를 다시 시작한 뒤 반영될 수 있습니다. 앱은 Finder를 자동으로 다시 시작하지 않습니다.";

test("every allowlisted app message has all four language variants", () => {
  const keys = Object.keys(appTextCatalog.ko);
  assert.ok(keys.length > 80);
  for (const language of languages) {
    assert.deepEqual(Object.keys(appTextCatalog[language]), keys);
    for (const key of keys) {
      const translated = localizeAppText(language, key);
      assert.equal(typeof translated, "string");
      assert.ok(translated.length > 0);
      if (language === "ko") assert.equal(translated, key);
      else assert.doesNotMatch(translated, /[가-힣]/, `${language}: ${key}`);
    }
  }
});

test("the same existing backend result follows language changes without another query", () => {
  assert.match(localizeAppText("en", macWarning), /^Unset values/);
  assert.match(localizeAppText("ja", macWarning), /^未設定値/);
  assert.match(localizeAppText("zh-CN", macWarning), /^不会/);
  assert.equal(localizeAppText("ko", macWarning), macWarning);
});

test("Windows snapshot warnings and app removal warnings preserve safety distinctions", () => {
  const windows = "설정 화면 연결은 실행 요청 전달만 의미합니다. Windows 버전별 페이지 가용성·실제 창 표시·설정 변경은 확인하지 않으며 앱은 값을 쓰지 않습니다.";
  assert.match(localizeAppText("en", windows), /only send an open request/);
  assert.match(localizeAppText("en", windows), /writes no settings/);
  const app = "전용 제거 프로그램이 있다면 먼저 사용하세요. 앱 본체 이동은 완전 제거가 아닙니다";
  assert.match(localizeAppText("en", app), /not a complete uninstall/);
});

test("cleanup evidence localizes numeric age without claiming deletion safety", () => {
  assert.equal(localizeAppText("en", "최근 37일 동안 변경되지 않음"), "Unchanged for the last 37 days");
  assert.equal(localizeAppText("ja", "최근 0일 동안 변경되지 않음"), "過去0日間変更なし");
  assert.equal(localizeAppText("zh-CN", "최근 37일 동안 변경되지 않음"), "最近 37 天未更改");
  assert.equal(localizeAppText("en", "최근 -1일 동안 변경되지 않음"), "최근 -1일 동안 변경되지 않음");
});

test("captured paths keep Unicode, newlines, and replacement metacharacters verbatim", () => {
  const path = "/Users/테스트/$1/$&/새 폴더\n보고서.txt";
  const text = `같은 내용의 파일 1개는 남깁니다: ${path}`;
  assert.equal(localizeAppText("en", text), `One identical file is kept: ${path}`);
  assert.equal(localizeAppText("ko", text), text);
});

test("closed-set setting titles are localized inside known warning templates", () => {
  assert.equal(localizeAppText("en", "파일 확장자 표시 값을 읽지 못했습니다"), "Could not read Show file extensions");
  assert.match(localizeAppText("ja", "숨김 파일 표시 값을 읽지 못했습니다"), /^隠しファイルを表示/);
  assert.equal(localizeAppText("en", "나만의 설정 값을 읽지 못했습니다"), "나만의 설정 값을 읽지 못했습니다");
});

test("app identifiers are opaque, tightly bounded template parameters", () => {
  const source = "앱 식별자 com.example.테스트와 정확히 일치하는 사용자 전용 경로";
  assert.equal(localizeAppText("en", source), source);
  assert.equal(
    localizeAppText("en", "앱 식별자 com.example.editor와 정확히 일치하는 사용자 전용 경로"),
    "User-only path matching app identifier com.example.editor exactly",
  );
});

test("MCP client names and terminal commands are preserved", () => {
  assert.equal(localizeAppText("en", "Claude Code에 BroomSweepy가 연결되어 있습니다"), "BroomSweepy is connected to Claude Code");
  const source = "Codex CLI는 실행되지만 로그인이 필요합니다. 터미널에서 codex login로 로그인한 뒤 다시 확인해 주세요";
  assert.match(localizeAppText("en", source), /Run codex login in a terminal/);
});

test("persisted Docker outcomes keep count and irreversible-action semantics", () => {
  assert.equal(localizeAppText("en", "선택한 Docker 정리 3개를 완료했습니다"), "Completed 3 selected Docker cleanup steps");
  assert.equal(localizeAppText("en", "Docker 정리 1/3개를 완료했습니다. 완료된 단계는 되돌릴 수 없습니다"), "Completed 1/3 Docker cleanup steps. Completed steps cannot be undone");
});

test("known extraction issues are translated, unknown diagnostic payloads are not rewritten", () => {
  assert.equal(localizeAppText("en", "검색할 수 있는 텍스트가 없습니다"), "No searchable text is available");
  assert.equal(localizeAppText("en", "문서 XML을 읽지 못했습니다: 문서이름_원문.xml"), "Could not read document XML: 문서이름_원문.xml");
});

test("native Mac permission and restore errors keep their exact safety meaning", () => {
  assert.equal(localizeAppText("en", "이 컴퓨터에만 적용된 별도 설정 값이 있어 변경하지 않습니다"), "A separate computer-specific preference exists; no change will be made");
  assert.equal(localizeAppText("en", "현재 설정이 다시 변경되어 복원 기록을 완료하지 않았습니다"), "The preference changed again; its restore record was not marked complete");
  assert.equal(localizeAppText("en", "Mac 설정 복원 작업이 중단되었습니다: SQLite 原文 오류"), "Mac preference restoration was interrupted: SQLite 原文 오류");
});

test("catalog progress, schema limits, and cancellation are app-owned messages", () => {
  assert.equal(localizeAppText("en", "바뀐 파일만 검색 목록에 반영하고 있습니다…"), "Updating the search catalog with changed files only…");
  assert.equal(localizeAppText("en", "이 버전에서 읽을 수 없는 파일 목록입니다 (형식 7). 파일 목록을 지우고 다시 만들어 주세요"), "This version cannot read file catalog format 7. Clear and rebuild the catalog");
  assert.equal(localizeAppText("en", "문서 읽기를 취소했습니다"), "Document reading was cancelled");
});

test("nested backend or OS diagnostics are opaque and never recursively translated", () => {
  assert.equal(localizeAppText("en", "앱 목록 조회가 중단됐습니다: 앱 목록이 변경됐습니다. 새 목록에서 다시 선택하세요"), "App-list inspection was interrupted: 앱 목록이 변경됐습니다. 새 목록에서 다시 선택하세요");
  assert.equal(localizeAppText("en", "파일 목록을 열지 못했습니다: database is locked ($1)"), "The file catalog could not be opened: database is locked ($1)");
});

test("only enumerated ScanError Access wrappers localize known app safety reasons", () => {
  const reason = "클라우드 또는 온라인 전용 항목이 포함되어 검사를 완료하지 못했습니다. 더 작은 로컬 폴더를 선택해 주세요";
  assert.equal(localizeAppText("en", `failed to access scan path: ${reason}`), "Could not access the scan target: The inspection could not finish because it contains cloud or online-only items. Choose a smaller local folder");
  const unknown = "failed to access scan path: 알 수 없는 OS 오류 /Users/사용자/자료";
  assert.equal(localizeAppText("en", unknown), unknown);
});

test("unknown text, filenames, paths, URLs, model answers, and whitespace remain unchanged", () => {
  const values = [
    "프로젝트_정리계획.md", "/Users/사용자 캐시/파일 확장자 표시.txt", "C:\\사용자 캐시\\설정.txt",
    "https://example.com/사용자 캐시", "사용자가 입력한 새로운 문장", "모델이 자유롭게 작성한 답변입니다.",
    "질문: 최근 7일 동안 변경되지 않음", "최근 7일 동안 변경되지 않음 이라는 파일", " 사용자 캐시 ",
    "__proto__", "constructor", "toString", "", "\n", "existing English text",
  ];
  for (const language of languages) {
    for (const value of values) assert.equal(localizeAppText(language, value), value);
  }
});

test("overlong untrusted strings bypass template matching unchanged", () => {
  const value = `확인 중: ${"사용자자료".repeat(4_000)}`;
  assert.equal(localizeAppText("en", value), value);
});

test("the helper stays independent of React, bridge calls, and a running desktop app", async () => {
  const { readFile } = await import("node:fs/promises");
  const source = await readFile(new URL("../src/lib/localizeAppText.ts", import.meta.url), "utf8");
  assert.doesNotMatch(source, /import\s+(?!type\b)[\s\S]*?from\s+["']/);
  assert.doesNotMatch(source, /\b(?:invoke|fetch|localStorage|window|document)\s*\(/);
});
