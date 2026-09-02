import { spawnSync } from "node:child_process";
import {
  chmodSync,
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
} from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const desktopRoot = resolve(scriptDirectory, "..");
const repositoryRoot = resolve(desktopRoot, "..", "..");
const tauriRoot = join(desktopRoot, "src-tauri");
const binaryDirectory = join(tauriRoot, "binaries");
const packageJson = JSON.parse(readFileSync(join(desktopRoot, "package.json"), "utf8"));
const tauriConfig = JSON.parse(readFileSync(join(tauriRoot, "tauri.conf.json"), "utf8"));

const requestedTarget = readTargetArgument(process.argv.slice(2));
const targetTriple = requestedTarget ?? process.env.TAURI_ENV_TARGET_TRIPLE ?? hostTriple();
if (!/^[A-Za-z0-9._-]+$/.test(targetTriple)) {
  fail("대상 플랫폼 이름이 안전한 형식이 아닙니다.");
}

if (packageJson.version !== tauriConfig.version) {
  fail("package.json과 tauri.conf.json의 앱 버전이 다릅니다.");
}

mkdirSync(binaryDirectory, { recursive: true });
const extension = targetTriple.includes("windows") ? ".exe" : "";
const destination = join(
  binaryDirectory,
  `bloomsweepy-mcp-${targetTriple}${extension}`,
);

if (targetTriple === "universal-apple-darwin") {
  const armBinary = buildSidecar("aarch64-apple-darwin", "");
  const intelBinary = buildSidecar("x86_64-apple-darwin", "");
  run("lipo", ["-create", armBinary, intelBinary, "-output", destination]);
} else {
  const source = buildSidecar(targetTriple, extension);
  copyFileSync(source, destination);
}

if (process.platform !== "win32") {
  chmodSync(destination, 0o755);
}

const versionOutput = capture(destination, ["--version"]);
const versionMatch = /^bloomsweepy-mcp\s+(\S+)\s*$/.exec(versionOutput.trim());
if (!versionMatch) {
  fail("준비한 MCP 도구의 버전을 확인하지 못했습니다.");
}
if (versionMatch[1] !== packageJson.version) {
  fail(
    `앱 ${packageJson.version}과 MCP 도구 ${versionMatch[1]}의 버전이 다릅니다.`,
  );
}

process.stdout.write(
  `Prepared bloomsweepy-mcp ${versionMatch[1]} for ${targetTriple}.\n`,
);

function readTargetArgument(argumentsList) {
  if (argumentsList.length === 0) return null;
  if (argumentsList.length !== 2 || argumentsList[0] !== "--target") {
    fail("사용법: node scripts/prepare-sidecar.mjs [--target <target-triple>]");
  }
  return argumentsList[1];
}

function hostTriple() {
  const output = capture(rustTool("rustc", process.env.RUSTC), ["-vV"]);
  const match = /^host:\s*(\S+)$/m.exec(output);
  if (!match) fail("Rust 호스트 플랫폼을 확인하지 못했습니다.");
  return match[1];
}

function buildSidecar(target, targetExtension) {
  run(rustTool("cargo", process.env.CARGO), [
    "build",
    "--release",
    "--locked",
    "-p",
    "bloomsweepy-mcp",
    "--target",
    target,
  ]);
  return join(
    repositoryRoot,
    "target",
    target,
    "release",
    `bloomsweepy-mcp${targetExtension}`,
  );
}

function rustTool(name, override) {
  if (override) return override;
  const userRoot = process.env.USERPROFILE ?? process.env.HOME;
  if (userRoot) {
    const candidate = join(
      userRoot,
      ".cargo",
      "bin",
      process.platform === "win32" ? `${name}.exe` : name,
    );
    if (existsSync(candidate)) return candidate;
  }
  return name;
}

function run(program, argumentsList) {
  const result = spawnSync(program, argumentsList, {
    cwd: repositoryRoot,
    env: process.env,
    shell: false,
    stdio: "inherit",
  });
  if (result.error) fail(`${program} 실행을 시작하지 못했습니다: ${result.error.message}`);
  if (result.status !== 0) fail(`${program} 실행이 실패했습니다.`);
}

function capture(program, argumentsList) {
  const result = spawnSync(program, argumentsList, {
    cwd: repositoryRoot,
    encoding: "utf8",
    env: process.env,
    shell: false,
    stdio: ["ignore", "pipe", "pipe"],
  });
  if (result.error) fail(`${program} 실행을 시작하지 못했습니다: ${result.error.message}`);
  if (result.status !== 0) fail(`${program} 실행이 실패했습니다.`);
  return result.stdout;
}

function fail(message) {
  process.stderr.write(`${message}\n`);
  process.exit(1);
}
