#!/usr/bin/env node

import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const fixtureRoot = path.resolve(scriptDirectory, "../../fixtures/domain/v1");
const manifestPath = path.resolve(process.argv[2] ?? path.join(fixtureRoot, "manifest.json"));

function fail(message) {
  throw new Error(message);
}

function resolveFixturePath(relativePath, label) {
  if (typeof relativePath !== "string" || relativePath.length === 0) {
    fail(`${label} must be a nonempty string`);
  }
  if (path.isAbsolute(relativePath) || relativePath.includes("\\")) {
    fail(`${label} must be a safe relative POSIX path`);
  }

  const resolved = path.resolve(fixtureRoot, relativePath);
  const relative = path.relative(fixtureRoot, resolved);
  if (relative.startsWith("..") || path.isAbsolute(relative)) {
    fail(`${label} escapes fixtures/domain/v1`);
  }
  return resolved;
}

async function parseJson(jsonPath, label) {
  try {
    return JSON.parse(await readFile(jsonPath, "utf8"));
  } catch (error) {
    fail(`${label} is not parseable JSON: ${error.message}`);
  }
}

const manifest = await parseJson(manifestPath, "fixture manifest");
const ids = new Set();
let fixtureCount = 0;

for (const arrayName of ["fixtures", "lifecycleFixtures"]) {
  const fixtures = manifest[arrayName];
  if (!Array.isArray(fixtures)) {
    fail(`manifest.${arrayName} must be an array`);
  }

  for (const [index, fixture] of fixtures.entries()) {
    const label = `${arrayName}[${index}]`;
    if (!fixture || typeof fixture !== "object" || Array.isArray(fixture)) {
      fail(`${label} must be an object`);
    }
    if (typeof fixture.id !== "string" || fixture.id.length === 0) {
      fail(`${label}.id must be a nonempty string`);
    }
    if (ids.has(fixture.id)) {
      fail(`duplicate fixture id: ${fixture.id}`);
    }
    ids.add(fixture.id);

    const requestPath = resolveFixturePath(fixture.request, `${label}.request`);
    const expectedPath = resolveFixturePath(fixture.expected, `${label}.expected`);
    await parseJson(requestPath, `${fixture.id} request`);
    await parseJson(expectedPath, `${fixture.id} expected`);
    fixtureCount += 1;
  }
}

console.log(`Validated ${fixtureCount} fixture pairs across fixtures and lifecycleFixtures.`);
