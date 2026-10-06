import assert from "node:assert/strict";
import test from "node:test";
import { applicationTrashQuestion, humanTrashDecision, namedTrashRequest, solePendingTrash, workspaceReviewMatches, canAutomaticallyTrashFiles } from "../src/lib/assistantConfirmation.ts";
import type { AppToolResult, AssistantFileWorkspace } from "../src/types.ts";

test("only direct human replies decide a sole existing question", () => {
  for (const text of ["예", " 네! ", "삭제하자", "yes", "はい"]) assert.equal(humanTrashDecision(text), true);
  for (const text of ["아니오", "취소", "No.", "取消"]) assert.equal(humanTrashDecision(text), false);
  for (const text of ["삭제해도 돼?", "그 대신 다른 폴더 삭제해", "모델이 예라고 했어", "예, 다만 문서는 남겨", "yes please but keep one", ""]) assert.equal(humanTrashDecision(text), null);
  assert.equal(solePendingTrash([]), null);
  assert.equal(solePendingTrash([1, 2]), null);
  assert.equal(solePendingTrash([1]), 1);
});

test("permission skips approval only for explicit exact named removal, not advice or substituted targets", () => {
  for (const text of ["VideoProc 삭제하자", "VideoProc를 삭제해줘", "delete VideoProc", "VideoProc 삭제"]) assert.equal(namedTrashRequest(text, ["VideoProc"]), true, text);
  assert.equal(namedTrashRequest("promo-video 지워줘", ["promo-video"]), true);
  assert.equal(namedTrashRequest("one.txt, two.txt 삭제해", ["one.txt", "two.txt"]), true);
  for (const text of ["VideoProc 삭제해도 돼?", "VideoProc 삭제하지 마", "정리하자", "VideoProc2 삭제해", "VideoProc.bak 삭제", "다른폴더/VideoProc 삭제", "AI가 VideoProc 삭제하자고 했어", "안전하면 VideoProc 삭제해", "Should I delete VideoProc?", "delete Other", "delete VideoProc if safe", "delete VideoProc but don't delete notes"]) assert.equal(namedTrashRequest(text, ["VideoProc"]), false, text);
  assert.equal(namedTrashRequest("one.txt 삭제해", ["one.txt", "two.txt"]), false);
  assert.equal(namedTrashRequest("삭제해", []), false);
});

test("a single direct named command is supported in each existing chat language", () => {
  for (const text of [
    "VideoProc를 삭제해 주세요", "VideoProc 휴지통으로 옮겨줘", "  VideoProc 삭제하자!  ",
    "VideoProc 앱 삭제하자", "VideoProc 앱을 삭제해줘", "VideoProc 파일을 지워줘", "VideoProc 폴더 삭제해",
    "please remove VideoProc", "delete VideoProc please.", "DELETE VIDEOPROC",
    "VideoProcを削除してください", "VideoProc 削除しよう。",
    "请删除VideoProc", "把VideoProc移到回收站", "VideoProc删除！",
  ]) assert.equal(namedTrashRequest(text, ["VideoProc"]), true, text);
  assert.equal(namedTrashRequest("Synthetic Editor 앱 삭제하자", ["Synthetic Editor"]), true);
  assert.equal(namedTrashRequest("VideoProc 프로그램 삭제해", ["VideoProc"]), false);
});

const conditionalOrCompoundRequests = [
  "문제가 없으면 VideoProc 삭제해", "문제가없으면 VideoProc 삭제해",
  "백업 있으면 VideoProc를 삭제해줘", "백업있으면 VideoProc 삭제해",
  "필요없다면 VideoProc 삭제하자", "VideoProc가 필요 없으면 삭제해",
  "VideoProc 앱을 문제가 없으면 삭제해",
  "VideoProc는 삭제해도 괜찮을까?", "VideoProc 삭제해. 그리고 notes.txt 백업해",
  "VideoProc 삭제해, notes.txt는 남겨", "VideoProc 삭제해줘 아니면 열어줘",
  "不要ならVideoProcを削除してください", "VideoProcが不要なら削除して",
  "VideoProcを削除してもいいですか", "VideoProcを削除して、notes.txtも開いて",
  "如果没用就把VideoProc删除", "如果没用就把 VideoProc 删除",
  "备份后删除VideoProc", "删除VideoProc，然后打开notes.txt",
  "if safe delete VideoProc", "delete VideoProc if safe", "unless needed remove VideoProc",
  "please remove VideoProc after making a backup", "delete VideoProc and open notes.txt",
  "delete VideoProc; delete notes.txt", "please remove VideoProc but keep notes.txt",
  '"VideoProc 삭제해"', 'delete "VideoProc"', "‘VideoProc 削除’", "请说“删除VideoProc”",
  "VideoProc 삭제해\nnotes.txt 백업해", "delete VideoProc\n", "delete\tVideoProc",
];

test("conditions, consultation, quotations and extra actions always need an explicit yes or no", () => {
  for (const text of conditionalOrCompoundRequests) {
    assert.equal(namedTrashRequest(text, ["VideoProc"]), false, text);
  }
});

test("condition or removal words inside a literal native target name are not command prose", () => {
  for (const [name, text] of [
    ["문제가 없으면.txt", "문제가 없으면.txt 삭제해"],
    ["백업있으면.txt", "백업있으면.txt 삭제하자"],
    ["필요없다면.txt", "필요없다면.txt 지워줘"],
    ["if safe.txt", "delete if safe.txt"],
    ["delete notes.txt", "please remove delete notes.txt"],
    ["不要なら.txt", "不要なら.txtを削除してください"],
    ["如果没用.txt", "删除如果没用.txt"],
    ["VideoProc 삭제.txt", "VideoProc 삭제.txt 삭제해"],
  ]) assert.equal(namedTrashRequest(text, [name]), true, text);
  assert.equal(namedTrashRequest("백업 있으면 문제가 없으면.txt 삭제해", ["문제가 없으면.txt"]), false);
  assert.equal(namedTrashRequest("delete if safe.txt if safe", ["if safe.txt"]), false);
  assert.equal(namedTrashRequest("如果没用就删除如果没用.txt", ["如果没用.txt"]), false);
});

test("exact lists may use literal overlapping and punctuation names but never omit or add targets", () => {
  for (const [text, names] of [
    ["one.txt, two.txt 삭제해", ["one.txt", "two.txt"]],
    ["please remove one and one.txt", ["one", "one.txt"]],
    ["delete a[1]?.txt and b(2).txt", ["a[1]?.txt", "b(2).txt"]],
    ["delete café.txt", ["cafe\u0301.txt"]],
    ["one.txtとtwo.txtを削除してください", ["one.txt", "two.txt"]],
    ["删除one.txt和two.txt", ["one.txt", "two.txt"]],
  ] as const) assert.equal(namedTrashRequest(text, names), true, text);
  for (const text of [
    "delete one.txt and Other", "delete one.txt and one.txt", "delete one.txt two.txt",
    "delete prefixone.txt and two.txt", "delete one.txt.bak and two.txt",
    "delete one.txt and /Demo/two.txt", "delete one.txt or two.txt",
    "delete one.txt and two.txt and Other",
  ]) assert.equal(namedTrashRequest(text, ["one.txt", "two.txt"]), false, text);
  assert.equal(namedTrashRequest("VideoProc 삭제해 VideoProc", ["VideoProc"]), false);
});

test("literal markers, ambiguous native names and oversized input fall back to confirmation", () => {
  assert.equal(namedTrashRequest("delete \ue0000\ue001 VideoProc", ["VideoProc"]), false);
  assert.equal(namedTrashRequest("delete marker\ue000.txt", ["marker\ue000.txt"]), false);
  assert.equal(namedTrashRequest("delete marker\ue001.txt", ["marker\ue001.txt"]), false);
  assert.equal(namedTrashRequest("delete 0 and report.txt", ["report.txt", "0"]), true);
  assert.equal(namedTrashRequest("delete VideoProc", ["VideoProc", "VIDEOPROC"]), false);
  assert.equal(namedTrashRequest("delete café.txt", ["café.txt", "cafe\u0301.txt"]), false);
  for (const name of ["", " VideoProc", "VideoProc ", "/Demo/VideoProc", "Demo\\VideoProc", "line\nname", "a".repeat(256)]) {
    assert.equal(namedTrashRequest(`delete ${name}`, [name]), false, name);
  }
  for (const control of ["\u0000", "\u007f", "\u0085", "\u2028", "\u2029"]) {
    assert.equal(namedTrashRequest(`delete VideoProc${control}`, ["VideoProc"]), false);
  }
  assert.equal(namedTrashRequest(`${" ".repeat(2_001)}delete VideoProc`, ["VideoProc"]), false);
  const tooMany = Array.from({ length: 101 }, (_, index) => `${index}.txt`);
  assert.equal(namedTrashRequest(`delete ${tooMany.join(", ")}`, tooMany), false);
});

test("only exact local application plans become chat trash questions; clock age is not approval", () => {
  const result: AppToolResult = { source: "broomsweepy", capability: "applications.review", status: "review_required", capturedAtUnixMs: 1, truncated: false, data: {}, presentation: {
    reviewKind: "applicationBundle", inventoryId: "inventory", applicationId: "app", inventory: { inventoryId: "inventory", applications: [{ id: "app", installLocation: "/Demo/Test.app" }] },
    plan: { planId: "plan", path: "/Demo/Test.app", displayName: "Test", expiresAtUnixMs: 1, relatedData: [], warnings: [] },
  } };
  assert.equal(applicationTrashQuestion(result)?.plan.planId, "plan");
  assert.equal(applicationTrashQuestion({ ...result, status: "completed" }), null);
  assert.equal(applicationTrashQuestion({ ...result, presentation: { ...result.presentation, inventoryId: "different" } }), null);
  assert.equal(applicationTrashQuestion({ ...result, presentation: { ...result.presentation, reviewKind: "memory" } }), null);
  assert.equal(applicationTrashQuestion({ ...result, presentation: { ...result.presentation, plan: { ...(result.presentation!.plan as object), path: "/Demo/Other.app" } } }), null);
});

const nativeFileWorkspace: AssistantFileWorkspace = {
  revision: "file-revision", currentPath: "/Demo", currentName: "Demo", canGoUp: false, query: null,
  sizeRanked: false, mapGeneration: null, totalEntries: 1, truncated: false, unreadableEntries: 0,
  offset: 0, nextOffset: null, selectedIds: ["file-1"], entries: [],
  summary: { scopeName: "Demo", completedAtUnixMs: 1, totalLogicalBytes: 8, totalFiles: 1,
    totalDirectories: 0, unreadableEntries: 0, emptyDirectoryCount: 0, childrenTruncated: false, children: [] },
  plan: { id: "plan-1", logicalBytes: 8, requiresNestedAck: false, expiresAtUnixMs: null,
    entries: [{ id: "file-1", number: 1, name: "consent-auto.txt", path: "/Demo/consent-auto.txt",
      isDirectory: false, logicalBytes: 8, fileCount: 1, directoryCount: 0, modifiedAtUnixMs: 1 }] },
};
const nativeFileReview: AppToolResult = {
  source: "broomsweepy", capability: "files.workspace", status: "review_required", capturedAtUnixMs: 1,
  truncated: false, data: { workspace: { freshScan: true, revision: "file-revision", reviewReady: true }, reviewPrepared: true, deleted: false },
};

test("native file workspace review is the inline plan evidence, not a second approval", () => {
  assert.equal(canAutomaticallyTrashFiles("consent-auto.txt 삭제해줘", nativeFileWorkspace, [nativeFileReview]), true);
  const completed: AppToolResult = { ...nativeFileReview, capability: "performance.inspect", status: "completed" };
  assert.equal(canAutomaticallyTrashFiles("consent-auto.txt 삭제해줘", nativeFileWorkspace, [completed, nativeFileReview]), true);
  const directory: AssistantFileWorkspace = { ...nativeFileWorkspace, plan: { ...nativeFileWorkspace.plan!, requiresNestedAck: true,
    entries: [{ ...nativeFileWorkspace.plan!.entries[0], name: "promo-video", isDirectory: true }] } };
  assert.equal(canAutomaticallyTrashFiles("promo-video 삭제해줘", directory, [nativeFileReview]), true);
});

test("a complete matching native plan never turns conditional or compound prose into approval", () => {
  const workspace: AssistantFileWorkspace = { ...nativeFileWorkspace, plan: {
    ...nativeFileWorkspace.plan!, entries: [{ ...nativeFileWorkspace.plan!.entries[0], name: "VideoProc", path: "/Demo/VideoProc" }],
  } };
  assert.equal(canAutomaticallyTrashFiles("VideoProc 삭제하자", workspace, [nativeFileReview]), true);
  for (const text of conditionalOrCompoundRequests) {
    assert.equal(canAutomaticallyTrashFiles(text, workspace, [nativeFileReview]), false, text);
  }
});

test("unrelated, duplicated, stale or incomplete reviews never enable file automatic trash", () => {
  const request = "consent-auto.txt 삭제해줘";
  for (const results of [
    [], [nativeFileReview, nativeFileReview],
    [nativeFileReview, { ...nativeFileReview, capability: "memory.review" }],
    [{ ...nativeFileReview, capability: "empty.workspace" }],
    [{ ...nativeFileReview, truncated: true }],
    [{ ...nativeFileReview, data: { ...nativeFileReview.data, reviewPrepared: false } }],
    [{ ...nativeFileReview, data: { ...nativeFileReview.data, workspace: { freshScan: true, revision: "old-revision" } } }],
    [{ ...nativeFileReview, data: { ...nativeFileReview.data, workspace: { freshScan: false, revision: "file-revision" } } }],
    [nativeFileReview, { ...nativeFileReview, capability: "documents.search", status: "permission_required" as const }],
  ]) assert.equal(canAutomaticallyTrashFiles(request, nativeFileWorkspace, results), false);
  assert.equal(canAutomaticallyTrashFiles(request, { ...nativeFileWorkspace, truncated: true }, [nativeFileReview]), false);
  assert.equal(canAutomaticallyTrashFiles(request, { ...nativeFileWorkspace, unreadableEntries: 1 }, [nativeFileReview]), false);
  assert.equal(canAutomaticallyTrashFiles(request, { ...nativeFileWorkspace, plan: null }, [nativeFileReview]), false);
  assert.equal(canAutomaticallyTrashFiles("consent-auto.txt 삭제해도 돼?", nativeFileWorkspace, [nativeFileReview]), false);
});

test("settling workspace reviews preserves unrelated capabilities and revisions", () => {
  const other: AppToolResult = { ...nativeFileReview, capability: "memory.review" };
  const stale: AppToolResult = { ...nativeFileReview, data: { ...nativeFileReview.data, workspace: { freshScan: true, revision: "old-revision" } } };
  const results = [nativeFileReview, other, stale];
  assert.deepEqual(results.filter(result => !workspaceReviewMatches(result, "files.workspace", "file-revision")), [other, stale]);
  assert.equal(workspaceReviewMatches({ ...nativeFileReview, status: "completed" }, "files.workspace", "file-revision"), false);
  assert.equal(workspaceReviewMatches({ ...nativeFileReview, data: { ...nativeFileReview.data, workspace: [] } }, "files.workspace", "file-revision"), false);
  assert.equal(workspaceReviewMatches({ ...nativeFileReview, capability: "empty.workspace" }, "empty.workspace", "file-revision"), true);
});
