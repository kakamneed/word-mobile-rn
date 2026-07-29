import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { dirname, resolve } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');

test('accepts a valid manifest pinned to an ancestor source commit', () => {
  const result = spawnSync(
    process.execPath,
    [
      'scripts/domain-export/validate-manifest.mjs',
      'artifacts/domain-wasm/manifest.json',
    ],
    { cwd: root, encoding: 'utf8' },
  );

  assert.equal(result.status, 0, result.stderr || result.stdout);
  assert.match(result.stdout, /Manifest valid for/);
});
