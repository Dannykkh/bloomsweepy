// Real component + mock native bridge. Never opens BroomSweepy or writes OS preferences.
// Use an installed Playwright module; no implicit package/browser download.
// Browser execution requires separate authorization; node --check only parses this file.
import assert from "node:assert/strict";
import { mkdir, mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const moduleId = process.env.BROOM_PLAYWRIGHT_MODULE;
const { chromium } = await import(moduleId ? pathToFileURL(resolve(moduleId)).href : "playwright");
const base = process.env.BROOM_FIXTURE_URL ?? "http://127.0.0.1:1431";
const output = resolve(process.env.BROOM_QA_OUTPUT ?? fileURLToPath(new URL("../../../docs/qa/screenshots", import.meta.url)));
await mkdir(output, { recursive: true });
const browserArgs = process.env.BROOM_BROWSER_ARGS_JSON ? JSON.parse(process.env.BROOM_BROWSER_ARGS_JSON) : [];
assert.ok(Array.isArray(browserArgs) && browserArgs.every(value => typeof value === "string"), "BROOM_BROWSER_ARGS_JSON must be a JSON string array");
assert.ok(!(process.env.BROOM_BROWSER_EXECUTABLE && process.env.BROOM_BROWSER_CHANNEL), "Choose a browser executable OR a channel, not both");
const profileEnabled = process.env.BROOM_QA_PROFILE === "1";
const profileOutput = profileEnabled
  ? process.env.BROOM_QA_PROFILE_OUTPUT ? resolve(process.env.BROOM_QA_PROFILE_OUTPUT) : await mkdtemp(resolve(tmpdir(), "broomsweepy-mac-settings-ui-profiles-"))
  : null;
const caseFilters = (process.env.BROOM_QA_CASE ?? "").split(",").map(value => value.trim()).filter(Boolean);
const scenarioTimeout = Number(process.env.BROOM_QA_CASE_TIMEOUT_MS ?? 60000);
assert.ok(Number.isFinite(scenarioTimeout) && scenarioTimeout >= 1000 && scenarioTimeout <= 300000, "Scenario timeout must be 1000–300000 ms");
const browser = await chromium.launch({
  headless: process.env.BROOM_BROWSER_HEADLESS !== "0",
  args: browserArgs,
  ...(process.env.BROOM_BROWSER_EXECUTABLE ? { executablePath: process.env.BROOM_BROWSER_EXECUTABLE } : {}),
  ...(process.env.BROOM_BROWSER_CHANNEL ? { channel: process.env.BROOM_BROWSER_CHANNEL } : {}),
});
console.log(JSON.stringify({ browserVersion: browser.version(), headless: process.env.BROOM_BROWSER_HEADLESS !== "0", browserArgs, profileEnabled, profileOutput, caseFilters, scenarioTimeout }));
let passed = 0;
const problems = [];

async function bounded(promise, label, timeout = 2500) {
  let timer;
  try {
    return await Promise.race([promise, new Promise((_, reject) => {
      timer = setTimeout(() => reject(new Error(`${label} exceeded ${timeout} ms (Node watchdog)`)), timeout);
    })]);
  } finally { clearTimeout(timer); }
}

const unavailable = error => ({ unavailable: error.message });

async function diagnostics(page) {
  // page.evaluate has no Playwright action timeout. A busy renderer must not hang failure reporting.
  const state = await bounded(page.evaluate(() => ({
    visibility: document.visibilityState,
    readyState: document.readyState,
    fixture: window.__macSettingsFixture ?? null,
    reactLog: document.querySelector('[data-testid="fixture-log"]')?.textContent ?? null,
    body: document.body?.innerText.slice(0, 1800) ?? null,
  })), "failure DOM diagnostics").catch(unavailable);
  console.log(JSON.stringify({ diagnostics: state }));
}

async function stopProfile(session, name) {
  if (!session) return;
  try {
    const { profile } = await bounded(session.send("Profiler.stop"), "CPU profiler stop", 5000);
    const counts = new Map();
    for (const id of profile.samples ?? []) counts.set(id, (counts.get(id) ?? 0) + 1);
    const nodes = new Map(profile.nodes.map(node => [node.id, node]));
    const total = profile.samples?.length ?? 0;
    const hotFunctions = [...counts].sort((a, b) => b[1] - a[1]).slice(0, 12).map(([id, samples]) => ({
      samples, percent: total ? Math.round(samples / total * 1000) / 10 : 0,
      ...nodes.get(id)?.callFrame,
    }));
    assert.ok(profileOutput, "CPU profile output requires the opt-in profiler");
    await mkdir(profileOutput, { recursive: true });
    const file = resolve(profileOutput, `${name.replace(/[^a-zA-Z0-9-]+/g, "-").slice(0, 90)}-${Date.now()}.cpuprofile`);
    await writeFile(file, JSON.stringify(profile));
    console.log(JSON.stringify({ cpuProfile: file, totalSamples: total, hotFunctions }));
  } catch (error) { console.log(JSON.stringify({ cpuProfile: unavailable(error) })); }
  finally { await bounded(session.detach(), "CPU profiler detach").catch(() => {}); }
}

async function scenario(name, kind, run, options = {}) {
  if (caseFilters.length && !caseFilters.some(filter => name.includes(filter) || kind.includes(filter))) return;
  const context = await bounded(browser.newContext({ viewport: { width: 1280, height: 820 }, reducedMotion: "reduce", ...options }), "new browser context", 15000);
  const page = await bounded(context.newPage(), "new browser page", 15000);
  page.setDefaultTimeout(30000);
  let profiler = null;
  const errors = [];
  page.on("pageerror", error => errors.push(error.message));
  page.on("console", message => { if (message.type() === "error") errors.push(message.text()); });
  page.on("response", response => { if (response.status() >= 400 && !response.url().endsWith("/favicon.ico")) errors.push(`HTTP ${response.status()} ${response.url()}`); });
  try {
    if (profileEnabled) {
      profiler = await bounded(context.newCDPSession(page), "CPU profiler attach", 5000);
      await bounded(profiler.send("Profiler.enable"), "CPU profiler enable");
      await bounded(profiler.send("Profiler.setSamplingInterval", { interval: 1000 }), "CPU profiler interval");
      await bounded(profiler.send("Profiler.start"), "CPU profiler start");
    }
    await bounded((async () => {
      await page.bringToFront();
      await page.goto(`${base}/mac-settings-fixture.html?case=${kind}`, { waitUntil: "domcontentloaded", timeout: 15000 });
      await run(page);
      assert.deepEqual(errors, [], "no console/page/network errors");
    })(), `scenario ${name}`, scenarioTimeout);
    passed += 1;
    console.log(`PASS ${name}`);
  } catch (error) {
    problems.push({ name, error: error.message });
    console.log(`FAIL ${name}: ${error.message}`);
    console.log(JSON.stringify({ errors }));
    await diagnostics(page);
  } finally {
    await stopProfile(profiler, name);
    try { await bounded(context.close(), "browser context cleanup", 10000); }
    catch (error) { problems.push({ name: `${name} cleanup`, error: error.message }); console.log(`FAIL ${name} cleanup: ${error.message}`); }
  }
}

const ready = page => page.getByText("설정 상태를 새로 확인했습니다.", { exact: true }).waitFor({ timeout: 30000 });
const apply = page => page.getByRole("button", { name: "파일 확장자 표시 · 적용", exact: true });
const choice = (page, enabled = true, title = "파일 확장자 표시") => page.getByRole("button", { name: `${title} · ${enabled ? "켜기" : "끄기"}`, exact: true });
const choiceGroup = (page, title = "파일 확장자 표시") => page.getByRole("group", { name: `${title} · 변경할 값`, exact: true });
const assertChoice = async (page, value) => {
  assert.equal(await choiceGroup(page).count(), 1);
  for (const enabled of [false, true]) {
    assert.equal(await choice(page, enabled).getAttribute("aria-pressed"), String(value === enabled));
  }
};
const log = page => page.getByTestId("fixture-log");
const logHas = async (page, pattern) => {
  await page.waitForFunction(pattern => new RegExp(pattern).test(document.querySelector('[data-testid="fixture-log"]')?.textContent ?? ""), pattern.source, { polling: 100 });
};

try {
  await scenario("MAC-07 stage/apply/restore, same-render duplicate protection and absent restoration", "slow", async page => {
    await ready(page);
    assert.equal(await apply(page).isDisabled(), true);
    await assertChoice(page, null); // Unset means neither choice is pressed, not false.
    await choice(page).click();
    await assertChoice(page, true);
    await page.getByText("아직 적용 안 됨", { exact: true }).waitFor();
    assert.match(await log(page).innerText(), /applies: 0/);
    await apply(page).evaluate(button => { button.click(); button.click(); });
    await logHas(page, /applies: 1/);
    assert.equal(await choice(page).isDisabled(), true, "busy blocks controls");
    assert.equal(await choice(page, false).isDisabled(), true, "busy blocks both choices");
    await page.getByText("설정값을 저장하고 다시 확인했습니다. Finder 화면에는 다시 열 때 반영될 수 있습니다.", { exact: true }).waitFor();
    assert.match(await log(page).innerText(), /applies: 1/);
    await assertChoice(page, true);
    const restore = page.getByRole("button", { name: "파일 확장자 표시 · 복원", exact: true });
    await restore.evaluate(button => { button.click(); button.click(); });
    await page.getByText("원래 사용자 설정으로 복원하고 다시 확인했습니다.", { exact: true }).waitFor();
    assert.match(await log(page).innerText(), /restores: 1/);
    await assertChoice(page, null);
    assert.equal(await restore.count(), 0);
    assert.match(await log(page).innerText(), /actual OS calls: 0/);
  });
  await scenario("MAC-08 managed/type/error reads cannot stage writes", "mixed", async page => {
    await ready(page);
    for (const name of ["숨김 파일 표시", "상태 막대 표시", "이름순 정렬 시 폴더 우선"]) {
      assert.equal(await choiceGroup(page, name).count(), 1);
      for (const enabled of [false, true]) assert.equal(await choice(page, enabled, name).isDisabled(), true);
    }
    assert.equal(await page.getByText("현재 값 · 조직에서 관리", { exact: true }).count(), 1);
    assert.equal(await page.getByText("현재 값 · 설정 형식 불일치", { exact: true }).count(), 1);
    assert.equal(await page.getByText("현재 값 · 조회 실패", { exact: true }).count(), 1);
    assert.match(await log(page).innerText(), /applies: 0/);
  });
  await scenario("MAC-08 unsupported platform has no mutation controls", "unsupported", async page => {
    await ready(page);
    await page.getByRole("heading", { name: "이 운영체제에서는 Finder 설정을 지원하지 않습니다.", exact: true }).waitFor();
    assert.equal(await page.getByRole("group", { name: / · 변경할 값$/ }).count(), 0);
    assert.equal(await page.getByRole("button", { name: / · 적용$/ }).count(), 0);
    assert.match(await log(page).innerText(), /applies: 0/);
  });
  await scenario("MAC-08 read failure is actionable, no guessed values", "load-error", async page => {
    await page.getByRole("alert").waitFor();
    assert.equal(await page.getByRole("group", { name: / · 변경할 값$/ }).count(), 0);
    assert.equal(await page.getByRole("button", { name: / · 적용$/ }).count(), 0);
    assert.equal(await page.getByRole("button", { name: "상태 새로 확인", exact: true }).isEnabled(), true);
  });
  for (const kind of ["write-error", "external-change"]) {
    await scenario(`MAC-07/08 ${kind} locks stale state until explicit refresh`, kind, async page => {
      await ready(page);
      await choice(page).click();
      await apply(page).click();
      await page.getByRole("alert").waitFor();
      assert.equal(await choice(page).isDisabled(), true);
      assert.match(await log(page).innerText(), /applies: 1/);
      await page.getByRole("button", { name: "상태 새로 확인", exact: true }).click();
      await ready(page);
      assert.equal(await choice(page).isEnabled(), true);
      assert.match(await log(page).innerText(), /applies: 1/);
    });
  }
  await scenario("MAC-08 restore failure never claims success or auto-retries", "restore-error", async page => {
    await ready(page);
    await page.getByRole("button", { name: "경로 막대 표시 · 복원", exact: true }).click();
    await page.getByRole("alert").waitFor();
    assert.match(await log(page).innerText(), /restores: 1/);
    assert.equal(await page.getByRole("button", { name: "경로 막대 표시 · 복원", exact: true }).isDisabled(), true);
    assert.equal(await page.getByText("원래 사용자 설정으로 복원하고 다시 확인했습니다.", { exact: true }).count(), 0);
  });
  await scenario("MAC-07 late response cannot reopen an unmounted management view", "slow", async page => {
    await ready(page);
    await choice(page).click();
    await apply(page).click();
    await page.getByRole("button", { name: "시험 화면 나가기", exact: true }).click();
    await logHas(page, /Completed; no OS changes/);
    assert.equal(await page.getByTestId("fixture-route").count(), 1);
    assert.equal(await page.locator(".mac-management-view").count(), 0);
    assert.match(await log(page).innerText(), /applies: 1/);
    await page.getByRole("button", { name: "맥 설정 시험으로 돌아가기", exact: true }).click();
    await ready(page);
    await assertChoice(page, true);
    assert.equal(await page.getByRole("button", { name: "파일 확장자 표시 · 복원", exact: true }).count(), 1);
  });
  for (const width of [1280, 390]) {
    await scenario(`MAC-09 ${width}px layout/font/focus/reduced-motion exploration`, "success", async page => {
      await ready(page);
      assert.equal(await page.getByRole("heading", { name: "맥 설정", level: 1, exact: true }).count(), 1);
      assert.equal(await page.locator("h1").count(), 1, "one Shell-equivalent title, no duplicate title");
      assert.equal(await page.getByRole("heading", { name: "파일 표시", exact: true }).count(), 1);
      assert.equal(await page.getByRole("heading", { name: "경로와 정보", exact: true }).count(), 1);
      assert.deepEqual(await page.getByRole("group", { name: / · 변경할 값$/ }).evaluateAll(groups => groups.map(group => group.getAttribute("aria-label"))), [
        "파일 확장자 표시 · 변경할 값", "숨김 파일 표시 · 변경할 값", "이름순 정렬 시 폴더 우선 · 변경할 값", "경로 막대 표시 · 변경할 값", "상태 막대 표시 · 변경할 값",
      ]);
      assert.equal(await page.locator(".mac-management-connections").count(), 0, "no menu links disguised as setting actions");
      await page.evaluate(() => document.fonts.ready);
      const measured = await page.evaluate(() => ({
        width: innerWidth, scrollWidth: document.documentElement.scrollWidth,
        loaded: [...document.fonts].some(face => /Pretendard/.test(face.family) && face.status === "loaded"),
        reduced: matchMedia("(prefers-reduced-motion: reduce)").matches,
        minFont: Math.min(...[...document.querySelectorAll(".mac-management-view p,.mac-management-view button,.mac-management-view summary,.mac-management-view h3,.mac-management-view h4,.mac-management-view span")].filter(el => el.getClientRects().length).map(el => parseFloat(getComputedStyle(el).fontSize))),
        smallTargets: [...document.querySelectorAll(".mac-management-view button,.mac-management-view summary")].filter(el => el.getClientRects().length && (el.getBoundingClientRect().height < 44 || el.getBoundingClientRect().width < 44)).length,
      }));
      assert.ok(measured.scrollWidth <= measured.width, JSON.stringify(measured));
      assert.equal(measured.loaded, true);
      assert.equal(measured.reduced, true);
      assert.ok(measured.minFont >= 14);
      assert.equal(measured.smallTargets, 0);
      await page.keyboard.press("Tab");
      assert.equal(await page.evaluate(() => document.activeElement?.tagName), "BUTTON");
      await page.getByRole("button", { name: "설정 변경 기록", exact: true }).focus();
      await page.keyboard.press("Enter");
      assert.equal(await page.evaluate(() => document.activeElement?.id), "mac-history-title", "history navigation moves keyboard focus to its heading");
      await page.screenshot({ path: resolve(output, `2026-10-21-mac-settings-${width === 390 ? "narrow" : "desktop"}.jpg`), fullPage: true, type: "jpeg", quality: 80 });
    }, { viewport: { width, height: width === 390 ? 844 : 820 }, colorScheme: "dark" });
  }
  console.log(JSON.stringify({ passed, failed: problems.length, problems, actualOsWrites: 0 }));
  if (problems.length) process.exitCode = 1;
} finally {
  try { await bounded(browser.close(), "browser cleanup", 10000); }
  catch (error) {
    console.error(`FAIL browser cleanup: ${error.message}; exiting the failed harness, not waiting indefinitely for its automation browser`);
    process.exit(1);
  }
}
