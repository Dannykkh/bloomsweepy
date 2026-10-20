import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import test from "node:test";
import { isValidElement, type ReactElement } from "react";
import ts from "typescript";

const require = createRequire(import.meta.url);
const source = readFileSync(new URL("../src/views/SettingsView.tsx", import.meta.url), "utf8");
const compiled = ts.transpileModule(source, {
  compilerOptions: {
    module: ts.ModuleKind.CommonJS,
    jsx: ts.JsxEmit.ReactJSX,
    target: ts.ScriptTarget.ES2022,
  },
}).outputText;

type Element = ReactElement<Record<string, unknown>>;
type Component = ((props: Record<string, unknown>) => null) & { displayName: string };
const categories = ["general", "scanning", "ai", "monitoring", "developer", "about"];
const panelNames = [
  "StartupSettingsPanel",
  "MenuBarSettingsPanel",
  "ControlStatusPanel",
  "AssistantModelSettingsPanel",
  "McpConnectionPanel",
  "FileChangeReviewPanel",
  "DockerManagementPanel",
];

function elements(node: unknown): Element[] {
  if (Array.isArray(node)) return node.flatMap(elements);
  if (!isValidElement<Record<string, unknown>>(node)) return [];
  return [node, ...elements(node.props.children)];
}

// Exercise the real view composition without loading native IPC panels or a browser.
function createHarness() {
  let selectedCategory: unknown;
  const changes: unknown[] = [];
  const languageChanges: unknown[] = [];
  const sideEffects: string[] = [];
  const panels = Object.fromEntries(panelNames.map((name) => {
    const component: Component = Object.assign(() => null, { displayName: name });
    return [name, component];
  })) as Record<string, Component>;
  const module = { exports: {} as { SettingsView?: (props: unknown) => unknown } };

  new Function("require", "module", "exports", compiled)((id: string) => {
    if (id === "react") {
      return {
        useState: (initial: unknown) => {
          if (selectedCategory === undefined) selectedCategory = initial;
          return [selectedCategory, (next: unknown) => { selectedCategory = next; }];
        },
      };
    }
    if (id === "react/jsx-runtime") return require(id);
    if (id === "lucide-react") {
      return Object.fromEntries(["Database", "HardDrive", "Languages", "ShieldCheck"]
        .map((name) => [name, () => null]));
    }
    if (id === "../i18n") {
      return {
        useLanguage: () => ({
          language: "ko",
          preference: "ko",
          setPreference: (value: unknown) => { languageChanges.push(value); },
          storageError: false,
          t: (value: string) => value,
        }),
      };
    }
    if (id === "./SettingsView.css") return {};
    const component = panels[id.split("/").at(-1) ?? ""];
    assert.ok(component, `Unexpected runtime dependency: ${id}`);
    return { [component.displayName]: component };
  }, module, module.exports);

  const config = {
    minLargeFileBytes: 100 * 1024 * 1024,
    minDuplicateFileBytes: 1024 * 1024,
    maxLargeFiles: 250,
    maxDuplicateGroups: 100,
    maxDuplicateCandidates: 250_000,
    maxIssues: 50,
  };
  const controlSettings = { marker: "unchanged", onPermission: () => sideEffects.push("permission") };
  const props = {
    config,
    controlSettings,
    dockerStatus: { marker: "status" },
    dockerLoading: true,
    dockerChanging: false,
    dockerError: "existing error",
    onConfigChange: (value: unknown) => { changes.push(value); },
    onDockerEnabledChange: async () => { sideEffects.push("docker"); },
    onOpenDocker: () => { sideEffects.push("openDocker"); },
  };
  assert.ok(module.exports.SettingsView);
  const render = () => elements(module.exports.SettingsView!(props));
  const category = (nodes: Element[], id: string) => {
    const panel = nodes.find((node) => node.props.id === `settings-category-${id}`);
    assert.ok(panel, `Missing category panel: ${id}`);
    return panel;
  };
  const navigation = (nodes: Element[]) => nodes.filter((node) =>
    node.type === "button" && String(node.props["aria-controls"]).startsWith("settings-category-"));
  return { render, category, navigation, config, props, panels, changes, languageChanges, sideEffects };
}

test("settings opens General with six labeled category controls linked to mounted panels", () => {
  const harness = createHarness();
  const nodes = harness.render();
  const navigation = harness.navigation(nodes);
  assert.deepEqual(navigation.map((node) => node.props.children), [
    "일반", "검사·검색", "AI·권한", "파일 감시", "개발 도구", "정보",
  ]);
  assert.equal(nodes.find((node) => node.type === "nav")?.props["aria-label"], "앱 설정 분류");

  for (const [index, id] of categories.entries()) {
    const panel = harness.category(nodes, id);
    assert.equal(navigation[index].props.type, "button");
    assert.equal(navigation[index].props["aria-controls"], panel.props.id);
    assert.equal(navigation[index].props["aria-current"], index === 0 ? "page" : undefined);
    assert.equal(panel.props.hidden, index !== 0);
    assert.ok(elements(panel).some((node) =>
      node.type === "h2" && node.props.id === panel.props["aria-labelledby"]));
  }
});

test("category changes keep each existing panel in the same composition without changing settings", () => {
  const harness = createHarness();
  let nodes = harness.render();
  const originalPanels = panelNames.map((name) => nodes.find((node) => node.type === harness.panels[name])!);

  for (const id of [...categories, "general"]) {
    const button = harness.navigation(nodes).find((node) =>
      node.props["aria-controls"] === `settings-category-${id}`)!;
    (button.props.onClick as () => void)();
    nodes = harness.render();
    assert.deepEqual(categories.filter((key) => !harness.category(nodes, key).props.hidden), [id]);
    assert.equal(harness.navigation(nodes).filter((node) => node.props["aria-current"] === "page").length, 1);

    for (const [index, name] of panelNames.entries()) {
      const matching = nodes.filter((node) => node.type === harness.panels[name]);
      assert.equal(matching.length, 1, `${name} must not be conditionally removed or duplicated`);
      assert.equal(matching[0].type, originalPanels[index].type);
    }
  }

  assert.deepEqual(harness.changes, []);
  assert.deepEqual(harness.languageChanges, []);
  assert.deepEqual(harness.sideEffects, []);
});

test("existing settings panels belong to the intended category", () => {
  const harness = createHarness();
  const nodes = harness.render();
  const expected: Record<string, string[]> = {
    general: ["StartupSettingsPanel", "MenuBarSettingsPanel"],
    scanning: [],
    ai: ["ControlStatusPanel", "AssistantModelSettingsPanel", "McpConnectionPanel"],
    monitoring: ["FileChangeReviewPanel"],
    developer: ["DockerManagementPanel"],
    about: [],
  };
  for (const id of categories) {
    const childNodes = elements(harness.category(nodes, id));
    assert.deepEqual(childNodes.filter((node) => Object.values(harness.panels).includes(node.type as Component))
      .map((node) => (node.type as Component).displayName), expected[id]);
  }
  assert.equal(elements(harness.category(nodes, "about")).filter((node) => node.type === "li").length, 7);
});

test("scan thresholds still update immediately while preserving every other config field", () => {
  const harness = createHarness();
  const nodes = harness.render();
  const selects = elements(harness.category(nodes, "scanning")).filter((node) => node.type === "select");
  const fields = ["minLargeFileBytes", "minDuplicateFileBytes", "maxLargeFiles", "maxDuplicateGroups"] as const;
  const values = [50 * 1024 * 1024, 10 * 1024 * 1024, 500, 250];
  assert.equal(selects.length, fields.length);

  for (const [index, field] of fields.entries()) {
    assert.equal(selects[index].props.value, harness.config[field]);
    (selects[index].props.onChange as (event: unknown) => void)({ currentTarget: { value: String(values[index]) } });
    assert.deepEqual(harness.changes[index], { ...harness.config, [field]: values[index] });
  }
  assert.equal(harness.config.maxLargeFiles, 250, "the parent-owned config must not be mutated");
});

test("language and native permission/Docker callbacks preserve their existing contracts", () => {
  const harness = createHarness();
  const nodes = harness.render();
  const language = elements(harness.category(nodes, "general")).find((node) => node.type === "select")!;
  assert.equal(language.props.value, "ko");
  assert.equal(language.props["aria-describedby"], "language-setting-description");
  (language.props.onChange as (event: unknown) => void)({ currentTarget: { value: "ja" } });
  assert.deepEqual(harness.languageChanges, ["ja"]);

  const control = nodes.find((node) => node.type === harness.panels.ControlStatusPanel)!;
  assert.deepEqual(control.props, harness.props.controlSettings);
  const docker = nodes.find((node) => node.type === harness.panels.DockerManagementPanel)!;
  assert.equal(docker.props.status, harness.props.dockerStatus);
  assert.equal(docker.props.loading, true);
  assert.equal(docker.props.changing, false);
  assert.equal(docker.props.error, "existing error");
  assert.equal(docker.props.onEnabledChange, harness.props.onDockerEnabledChange);
  assert.equal(docker.props.onOpenDocker, harness.props.onOpenDocker);
  assert.deepEqual(harness.sideEffects, []);
});
