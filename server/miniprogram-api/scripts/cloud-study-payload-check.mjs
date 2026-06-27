import fs from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { SupabaseRestClient } from '../src/supabase/rest-client.js';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(scriptDir, '..', '..', '..');
const env = loadEnv(resolve(repoRoot, '.env.supabase.local'));
const url = process.env.SUPABASE_URL ?? env.SUPABASE_URL;
const serviceRoleKey =
  process.env.SUPABASE_SERVICE_ROLE_KEY ?? env.SUPABASE_SERVICE_ROLE_KEY;

if (!url || !serviceRoleKey) {
  throw new Error('Missing SUPABASE_URL or SUPABASE_SERVICE_ROLE_KEY');
}

const client = new SupabaseRestClient({ url, serviceRoleKey });
const rows = await client.select(
  'study_entry_payloads',
  '?select=entry_id,source_id,word,wordbook_id&order=rank_in_book.asc&limit=5',
);
const sharedRootRows = await client.select(
  'study_entry_payloads',
  '?source_id=like.root_affix_shared_%25&is_active=eq.true&select=source_id,word&limit=100',
);
const medicalRootRows = await client.select(
  'study_entry_payloads',
  '?source_id=like.root_affix_medical_%25&is_active=eq.true&select=source_id,word&limit=100',
);

if (!Array.isArray(rows) || rows.length === 0) {
  throw new Error(
    'study_entry_payloads exists but has no rows. Import study payloads before cloud smoke.',
  );
}

console.log(
  JSON.stringify(
    {
      table: 'study_entry_payloads',
      sampleRows: rows.length,
      firstSourceId: rows[0].source_id,
      firstWord: rows[0].word,
      sharedRootAffixRows: sharedRootRows.length,
      medicalRootAffixRows: medicalRootRows.length,
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
