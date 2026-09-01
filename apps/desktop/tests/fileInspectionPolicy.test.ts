import assert from "node:assert/strict";
import test from "node:test";
import { decideFileInspection } from "../src/lib/fileInspectionPolicy.ts";

test("opens only explicitly allowed document and media files", () => {
  assert.equal(decideFileInspection("C:\\Files\\report.PDF"), "open");
  assert.equal(decideFileInspection("/Users/test/photo.heic"), "open");
  assert.equal(decideFileInspection("/Users/test/movie.mp4"), "open");
});

test("reveals executable and shell-dispatched Windows formats", () => {
  for (const extension of ["exe", "pif", "application", "msc", "chm", "scf"]) {
    assert.equal(decideFileInspection(`C:\\Files\\sample.${extension}`), "reveal");
  }
});

test("reveals ambiguous names, alternate streams, and double extensions", () => {
  assert.equal(decideFileInspection("C:\\Files\\README"), "reveal");
  assert.equal(decideFileInspection("C:\\Files\\report.txt:payload"), "reveal");
  assert.equal(decideFileInspection("C:\\Files\\report.pdf.exe"), "reveal");
});

test("never directly opens catalog directories, links, or other entries", () => {
  assert.equal(decideFileInspection("/Applications/Sample.app", "directory"), "reveal");
  assert.equal(decideFileInspection("/Users/test/Documents", "directory"), "reveal");
  assert.equal(decideFileInspection("/Users/test/report.pdf", "symlink"), "reveal");
  assert.equal(decideFileInspection("/Users/test/report.pdf", "other"), "reveal");
});
