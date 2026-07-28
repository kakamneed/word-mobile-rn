#!/usr/bin/env node

import fs from 'node:fs';
import process from 'node:process';
import { isDeepStrictEqual } from 'node:util';

function parseJson(path) {
  try {
    return JSON.parse(fs.readFileSync(path, 'utf8').replace(/^\uFEFF/, ''));
  } catch (error) {
    throw new Error(`Unable to parse JSON ${path}: ${error.message}`);
  }
}

function compare(expectedPath, actualPath) {
  const expected = parseJson(expectedPath);
  const actual = parseJson(actualPath);
  if (!isDeepStrictEqual(expected, actual)) {
    throw new Error(
      `Semantic JSON mismatch: ${expectedPath} != ${actualPath}. Object key order is ignored; array order and values are significant.`,
    );
  }
}

function runSelfTest() {
  const expected = JSON.parse('{"alpha":1,"nested":{"beta":2},"items":["x","y"]}');
  const reorderedObject = JSON.parse('{"items":["x","y"],"nested":{"beta":2},"alpha":1}');
  const reorderedArray = JSON.parse('{"alpha":1,"nested":{"beta":2},"items":["y","x"]}');
  const changedValue = JSON.parse('{"alpha":2,"nested":{"beta":2},"items":["x","y"]}');

  if (!isDeepStrictEqual(expected, reorderedObject)) {
    throw new Error('Self-test failed: object key order must be ignored.');
  }
  if (isDeepStrictEqual(expected, reorderedArray)) {
    throw new Error('Self-test failed: array order must be preserved.');
  }
  if (isDeepStrictEqual(expected, changedValue)) {
    throw new Error('Self-test failed: scalar values must be preserved.');
  }
  console.log('Semantic JSON comparator self-test passed.');
}

const args = process.argv.slice(2);
if (args.length === 1 && args[0] === '--self-test') {
  runSelfTest();
  process.exit(0);
}

const expectedIndex = args.indexOf('--expected');
const actualIndex = args.indexOf('--actual');
if (
  expectedIndex < 0 ||
  actualIndex < 0 ||
  !args[expectedIndex + 1] ||
  !args[actualIndex + 1]
) {
  console.error(
    'Usage: compare-semantic-json.mjs --expected <expected.json> --actual <actual.json> | --self-test',
  );
  process.exit(2);
}

try {
  compare(args[expectedIndex + 1], args[actualIndex + 1]);
  console.log('Semantic JSON comparison passed.');
} catch (error) {
  console.error(error.message);
  process.exit(1);
}
