import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import ts from "typescript";

const sourcePath = new URL("../src/i18n/index.tsx", import.meta.url);
const sourceText = readFileSync(sourcePath, "utf8");
const sourceFile = ts.createSourceFile(
  sourcePath.pathname,
  sourceText,
  ts.ScriptTarget.Latest,
  true,
  ts.ScriptKind.TSX,
);

function unwrapExpression(expression: ts.Expression): ts.Expression {
  if (ts.isAsExpression(expression) || ts.isSatisfiesExpression(expression)) {
    return unwrapExpression(expression.expression);
  }
  if (ts.isParenthesizedExpression(expression)) {
    return unwrapExpression(expression.expression);
  }
  return expression;
}

function readEnglishCatalog(): Record<string, string> {
  for (const statement of sourceFile.statements) {
    if (!ts.isVariableStatement(statement)) continue;
    for (const declaration of statement.declarationList.declarations) {
      if (!ts.isIdentifier(declaration.name) || declaration.name.text !== "englishMessages") {
        continue;
      }
      assert.ok(declaration.initializer, "englishMessages must have an initializer");
      const expression = unwrapExpression(declaration.initializer);
      assert.ok(ts.isObjectLiteralExpression(expression), "englishMessages must be an object literal");

      return Object.fromEntries(
        expression.properties.map((property) => {
          assert.ok(ts.isPropertyAssignment(property), "catalog entries must be properties");
          assert.ok(
            ts.isStringLiteral(property.name) && ts.isStringLiteral(property.initializer),
            "catalog keys and values must be string literals",
          );
          return [property.name.text, property.initializer.text];
        }),
      );
    }
  }
  assert.fail("englishMessages was not found");
}

function readJsonCatalog(name: "ja" | "zh-CN"): Record<string, string> {
  return JSON.parse(
    readFileSync(new URL(`../src/i18n/${name}.json`, import.meta.url), "utf8"),
  ) as Record<string, string>;
}

function placeholders(value: string): string[] {
  return [...value.matchAll(/\{\{(\w+)\}\}/g)].map((match) => match[1]).sort();
}

const english = readEnglishCatalog();

for (const locale of ["ja", "zh-CN"] as const) {
  test(`${locale} catalog covers every English message with matching placeholders`, () => {
    const catalog = readJsonCatalog(locale);
    assert.ok(Object.keys(english).length > 800, "the source catalog should contain the full UI");
    assert.deepEqual(Object.keys(catalog).sort(), Object.keys(english).sort());

    for (const [key, englishValue] of Object.entries(english)) {
      assert.equal(typeof catalog[key], "string", `missing ${locale} value for ${key}`);
      assert.notEqual(catalog[key].trim(), "", `empty ${locale} value for ${key}`);
      assert.deepEqual(
        placeholders(catalog[key]),
        placeholders(englishValue),
        `placeholder mismatch for ${locale}: ${key}`,
      );
    }
  });
}
