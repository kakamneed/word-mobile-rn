import fs from 'node:fs';
import { dirname, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const packageRoot = resolve(scriptDir, '..');
const fixture = resolve(packageRoot, 'test-fixtures/study-entry-payloads.json');
const kajwebFixture = resolve(packageRoot, 'test-fixtures/kajweb-book.json');
const outputJson = resolve(packageRoot, 'out/check-study-entry-payloads.json');
const outputSql = resolve(packageRoot, 'out/check-study-entry-payloads.sql');
const kajwebOutputJson = resolve(packageRoot, 'out/check-kajweb-study-entry-payloads.json');
const kajwebOutputSql = resolve(packageRoot, 'out/check-kajweb-study-entry-payloads.sql');

const result = spawnSync(
  process.execPath,
  [
    resolve(scriptDir, 'export-study-entry-payloads.mjs'),
    `--source=${fixture}`,
    `--outputJson=${outputJson}`,
    `--outputSql=${outputSql}`,
  ],
  { encoding: 'utf8' },
);

if (result.status !== 0) {
  throw new Error(`Export failed: ${result.stderr || result.stdout}`);
}

const rows = JSON.parse(fs.readFileSync(outputJson, 'utf8'));
const sql = fs.readFileSync(outputSql, 'utf8');
if (rows.length !== 2 || rows[0].source_id !== 'fixture-alpha') {
  throw new Error('Exported payload JSON did not normalize fixture rows');
}
if (!sql.includes('insert into public.study_entry_payloads')) {
  throw new Error('Exported SQL missing study_entry_payloads insert');
}

const kajwebResult = spawnSync(
  process.execPath,
  [
    resolve(scriptDir, 'export-study-entry-payloads.mjs'),
    `--source=${kajwebFixture}`,
    `--outputJson=${kajwebOutputJson}`,
    `--outputSql=${kajwebOutputSql}`,
  ],
  { encoding: 'utf8' },
);

if (kajwebResult.status !== 0) {
  throw new Error(`Kajweb export failed: ${kajwebResult.stderr || kajwebResult.stdout}`);
}

const kajwebRows = JSON.parse(fs.readFileSync(kajwebOutputJson, 'utf8'));
if (
  kajwebRows.length !== 1 ||
  kajwebRows[0].source_id !== 'CET4_3_1' ||
  kajwebRows[0].word !== 'cancel' ||
  kajwebRows[0].meanings_json[0] !== '取消' ||
  kajwebRows[0].example_sentence !== 'Our flight was cancelled.'
) {
  throw new Error('Kajweb payload JSON did not normalize expected fields');
}

console.log(
  JSON.stringify(
    {
      rows: rows.length,
      kajwebRows: kajwebRows.length,
      outputJson,
      outputSql,
      firstSourceId: rows[0].source_id,
      firstKajwebSourceId: kajwebRows[0].source_id,
    },
    null,
    2,
  ),
);
