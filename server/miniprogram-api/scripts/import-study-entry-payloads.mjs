import fs from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { SupabaseRestClient } from '../src/supabase/rest-client.js';

const args = parseArgs(process.argv.slice(2));
const input = args.input ?? args.source ?? 'server/miniprogram-api/out/study-entry-payloads.json';
const batchSize = Number(args.batchSize ?? 500);

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..', '..');
const env = loadEnv(args.env ?? resolve(repoRoot, '.env.supabase.local'));
const url = args.supabaseUrl ?? env.SUPABASE_URL ?? process.env.SUPABASE_URL;
const serviceRoleKey =
  args.serviceRoleKey ??
  env.SUPABASE_SERVICE_ROLE_KEY ??
  process.env.SUPABASE_SERVICE_ROLE_KEY;

if (!fs.existsSync(input)) {
  fail(`Payload JSON not found: ${input}`);
}
if (!url || !serviceRoleKey) {
  fail('Missing SUPABASE_URL or SUPABASE_SERVICE_ROLE_KEY');
}

const rows = JSON.parse(fs.readFileSync(input, 'utf8'));
if (!Array.isArray(rows) || rows.length === 0) {
  fail('Input payload JSON must be a non-empty array');
}

const client = new SupabaseRestClient({ url, serviceRoleKey });
let inserted = 0;
for (let offset = 0; offset < rows.length; offset += batchSize) {
  const batch = rows.slice(offset, offset + batchSize);
  await client.upsert('study_entry_payloads', batch, { onConflict: 'entry_id' });
  inserted += batch.length;
}

console.log(
  JSON.stringify(
    {
      input,
      rows: inserted,
      table: 'study_entry_payloads',
    },
    null,
    2,
  ),
);

function loadEnv(filePath) {
  if (!fs.existsSync(filePath)) return {};
  const result = {};
  for (const line of fs.readFileSync(filePath, 'utf8').split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) continue;
    const index = trimmed.indexOf('=');
    if (index < 0) continue;
    result[trimmed.slice(0, index)] = trimmed.slice(index + 1);
  }
  return result;
}

function parseArgs(argv) {
  const result = {};
  for (const arg of argv) {
    if (!arg.startsWith('--')) continue;
    const [key, ...rest] = arg.slice(2).split('=');
    result[key] = rest.join('=') || true;
  }
  return result;
}

function fail(message) {
  console.error(message);
  process.exit(1);
}
