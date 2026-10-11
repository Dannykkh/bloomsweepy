import assert from "node:assert/strict";
import test from "node:test";
import {
  createFileChangeReviewPageLoader,
  fileChangeReviewDisposition,
  fileChangeReviewPagination,
  fileChangeReviewReason,
  fileChangeReviewRelativePath,
  type FileChangeReviewPage,
  type FileChangeReviewQuery,
} from "../src/lib/fileChangeReview.ts";

function page(revision = 1): FileChangeReviewPage {
  return { offset: 0, total: 0, entries: [], status: { config: { enabled: false, root: null, intervalMinutes: 15 },
    phase: "off", baselineEntries: 0, lastCheckedAtUnixMs: null, nextCheckAtUnixMs: null, historyCount: 0,
    reviewNeededCount: 0, skippedCount: 0, historyTruncated: false, error: null, revision } };
}

test("relative paths preserve identity and never strip a sibling or traversal path", () => {
  assert.equal(fileChangeReviewRelativePath("/Demo/Downloads/folder/긴 이름.txt", "/Demo/Downloads/"), "folder/긴 이름.txt");
  assert.equal(fileChangeReviewRelativePath("/Demo/Downloads", "/Demo/Downloads"), ".");
  assert.equal(fileChangeReviewRelativePath("/Demo/Downloads-other/file.txt", "/Demo/Downloads"), "/Demo/Downloads-other/file.txt");
  assert.equal(fileChangeReviewRelativePath("/Demo/downloads/file.txt", "/Demo/Downloads"), "/Demo/downloads/file.txt");
  assert.equal(fileChangeReviewRelativePath("/Demo/Downloads/../Private/file.txt", "/Demo/Downloads"), "/Demo/Downloads/../Private/file.txt");
  assert.equal(fileChangeReviewRelativePath("/Demo/file.txt", "/"), "Demo/file.txt");
  assert.equal(fileChangeReviewRelativePath("/Demo/file.txt", null), "/Demo/file.txt");
  assert.equal(fileChangeReviewRelativePath("/Demo/Downloads/back\\slash.txt", "/Demo/Downloads"), "back\\slash.txt");
  assert.equal(fileChangeReviewRelativePath("/Demo/Downloads/../Private/file.txt", "/Demo/Downloads/.."), "/Demo/Downloads/../Private/file.txt");
});

test("Windows drive and UNC roots use case insensitive boundaries without corrupting the filename", () => {
  assert.equal(fileChangeReviewRelativePath("C:\\Users\\Demo\\Downloads\\REPORT.PDF", "c:\\users\\demo\\downloads"), "REPORT.PDF");
  assert.equal(fileChangeReviewRelativePath("\\\\?\\C:\\Demo\\Folder\\한글.txt", "C:\\Demo\\Folder"), "한글.txt");
  assert.equal(fileChangeReviewRelativePath("\\\\?\\UNC\\server\\share\\Folder\\file.txt", "\\\\SERVER\\SHARE\\Folder"), "file.txt");
  assert.equal(fileChangeReviewRelativePath("C:\\Demo\\Folder-2\\file.txt", "C:\\Demo\\Folder"), "C:\\Demo\\Folder-2\\file.txt");
});

test("rule skipped and manually skipped records carry distinct labels without asserting safety", () => {
  assert.equal(fileChangeReviewDisposition({ reviewState: "skipped", decisionSource: "rule" }), "규칙으로 넘김");
  assert.equal(fileChangeReviewDisposition({ reviewState: "skipped", decisionSource: "manual" }), "직접 넘김");
  assert.equal(fileChangeReviewDisposition({ reviewState: "review_needed", decisionSource: "manual" }), "검토 필요");
  assert.equal(fileChangeReviewReason("executable"), "실행형 파일");
  assert.equal(fileChangeReviewReason("no_rule_match"), "검토 규칙에 해당하지 않음");
  assert.equal(fileChangeReviewReason("future_rule"), "알 수 없는 검토 사유");
});

test("a disposition shrinking the last page returns to the last valid twenty row page", () => {
  assert.deepEqual(fileChangeReviewPagination(40, 40, 0), {
    offset: 20, total: 40, start: 0, end: 0, previousOffset: 0, nextOffset: null,
  });
  assert.deepEqual(fileChangeReviewPagination(20, 41, 20), {
    offset: 20, total: 41, start: 21, end: 40, previousOffset: 0, nextOffset: 40,
  });
  assert.equal(fileChangeReviewPagination(0, 250, 500).end, 20);
  assert.equal(fileChangeReviewPagination(-1, 0, 0).offset, 0);
  assert.equal(fileChangeReviewPagination(Number.NaN, Number.NaN, 0).total, 0);
});

test("page reads never overlap; filter changes drop the old response and preserve the latest query", async () => {
  const calls: FileChangeReviewQuery[] = [];
  const pending: ((value: FileChangeReviewPage) => void)[] = [];
  const loader = createFileChangeReviewPageLoader(async (query) => {
    calls.push(query);
    return new Promise<FileChangeReviewPage>((resolve) => pending.push(resolve));
  });
  const first = loader.load({ offset: 0, filter: "review_needed" });
  await Promise.resolve(); await Promise.resolve();
  assert.equal(calls.length, 1);
  const latestQuery: FileChangeReviewQuery = { offset: 20, filter: "skipped" };
  const skipped = loader.load(latestQuery);
  latestQuery.offset = 99;
  assert.equal(calls.length, 1);
  pending[0](page(1));
  assert.equal(await first, null);
  await Promise.resolve(); await Promise.resolve();
  assert.deepEqual(calls[1], { offset: 20, filter: "skipped" });
  pending[1](page(2));
  assert.equal((await skipped)?.status.revision, 2);
});

test("a scope mutation invalidates a pending response and disposal prevents further page reads", async () => {
  let finish!: (value: FileChangeReviewPage) => void;
  let calls = 0;
  const loader = createFileChangeReviewPageLoader(async () => {
    calls += 1;
    return new Promise<FileChangeReviewPage>((resolve) => { finish = resolve; });
  });
  const pending = loader.load({ offset: 0, filter: "all" });
  await Promise.resolve(); await Promise.resolve();
  loader.invalidate();
  finish(page(1));
  assert.equal(await pending, null);
  loader.dispose();
  assert.equal(await loader.load({ offset: 0, filter: "all" }), null);
  assert.equal(calls, 1);
});

test("a failed read does not poison the serialized queue or replay a failed operation", async () => {
  let reads = 0;
  const loader = createFileChangeReviewPageLoader(async () => {
    reads += 1;
    if (reads === 1) throw new Error("synthetic transport failure");
    return page(2);
  });
  await assert.rejects(loader.load({ offset: 0, filter: "all" }), /synthetic transport failure/);
  assert.equal((await loader.load({ offset: 0, filter: "all" }))?.status.revision, 2);
  assert.equal(reads, 2);
});
