import assert from "node:assert/strict";
import test from "node:test";
import { isDockerManagementQuestion } from "../src/lib/dockerIntent.ts";

test("detects explicit Docker management questions", () => {
  assert.equal(isDockerManagementQuestion("도커 빌드 캐시가 얼마나 커?"), true);
  assert.equal(isDockerManagementQuestion("Docker container를 정리해줘"), true);
  assert.equal(isDockerManagementQuestion("buildx 용량을 확인해줘"), true);
  assert.equal(isDockerManagementQuestion("오래된 컨테이너를 정리할 수 있어?"), true);
});

test("does not treat ordinary image or photo questions as Docker", () => {
  assert.equal(isDockerManagementQuestion("중복 이미지를 찾아줘"), false);
  assert.equal(isDockerManagementQuestion("사진 폴더 용량을 알려줘"), false);
});
