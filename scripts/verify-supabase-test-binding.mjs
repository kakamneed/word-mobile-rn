import { readFileSync } from 'node:fs';

const email = (process.argv[2] ?? '').trim().toLowerCase();
if (!email) {
  throw new Error('Usage: node scripts/verify-supabase-test-binding.mjs <email>');
}

const env = readEnv('D:/projects/word-mobile-rn/.env.supabase.local');
const supabaseUrl = mustEnv(env, 'SUPABASE_URL').replace(/\/$/, '');
const serviceRoleKey = mustEnv(env, 'SUPABASE_SERVICE_ROLE_KEY');

const headers = {
  apikey: serviceRoleKey,
  authorization: `Bearer ${serviceRoleKey}`,
  'Content-Type': 'application/json',
};

const users = await listAuthUsers();
const authUser = users.find((user) => (user.email ?? '').toLowerCase() === email);
const userId = authUser?.id ?? null;

const [identityRows, accountRows, challengeRows] = await Promise.all([
  restGet(
    `user_identities?select=identity_id,internal_user_id,provider,provider_subject,supabase_user_id,normalized_email,is_verified,bound_at,last_seen_at,metadata_json&normalized_email=eq.${encodeURIComponent(email)}`,
  ),
  restGet(
    userId
      ? `internal_accounts?select=internal_user_id,status,primary_identity,supabase_owner_user_id,created_at,updated_at,last_login_at&supabase_owner_user_id=eq.${userId}`
      : 'internal_accounts?select=internal_user_id,status,primary_identity,supabase_owner_user_id,created_at,updated_at,last_login_at&limit=0',
  ),
  restGet(
    `email_bind_challenges?select=challenge_id,internal_user_id,normalized_email,expires_at,consumed_at,created_at,attempt_count&normalized_email=eq.${encodeURIComponent(email)}&order=created_at.desc`,
  ),
]);

const identityInternalIds = [...new Set(identityRows.map((row) => row.internal_user_id).filter(Boolean))];
const accountByIdentity = identityInternalIds.length
  ? await restGet(
      `internal_accounts?select=internal_user_id,status,primary_identity,supabase_owner_user_id,created_at,updated_at,last_login_at&internal_user_id=in.(${identityInternalIds.join(',')})`,
    )
  : [];

const counts = userId ? await countBusinessRows(userId) : {};

console.log(
  JSON.stringify(
    {
      email,
      authUser: authUser
        ? {
            id: authUser.id,
            email: authUser.email,
            emailConfirmedAt: authUser.email_confirmed_at ?? null,
            createdAt: authUser.created_at ?? null,
            lastSignInAt: authUser.last_sign_in_at ?? null,
          }
        : null,
      businessRowsForAuthUser: counts,
      identitiesForEmail: identityRows,
      internalAccountsByOwner: accountRows,
      internalAccountsByIdentity: accountByIdentity,
      emailBindChallenges: challengeRows,
    },
    null,
    2,
  ),
);

async function listAuthUsers() {
  const users = [];
  for (let page = 1; page <= 20; page += 1) {
    const response = await fetch(`${supabaseUrl}/auth/v1/admin/users?page=${page}&per_page=100`, {
      headers,
    });
    if (!response.ok) {
      const body = await response.text();
      throw new Error(`List auth users failed: ${response.status} ${body.slice(0, 500)}`);
    }
    const body = await response.json();
    const batch = Array.isArray(body.users) ? body.users : [];
    users.push(...batch);
    if (batch.length < 100) break;
  }
  return users;
}

async function countBusinessRows(userId) {
  const tables = [
    'profiles',
    'plan_configs',
    'wordbook_preferences',
    'study_word_points',
    'wrong_word_entries',
    'report_snapshots',
    'leaderboard_stats',
    'ai_passages',
    'croc_bti_profiles',
  ];
  const result = {};
  for (const table of tables) {
    const response = await fetch(`${supabaseUrl}/rest/v1/${table}?select=*&user_id=eq.${userId}`, {
      method: 'HEAD',
      headers: { ...headers, Prefer: 'count=exact' },
    });
    result[table] = response.headers.get('content-range') ?? `status:${response.status}`;
  }
  return result;
}

async function restGet(path) {
  const response = await fetch(`${supabaseUrl}/rest/v1/${path}`, { headers });
  if (!response.ok) {
    const body = await response.text();
    throw new Error(`REST get failed for ${path}: ${response.status} ${body.slice(0, 500)}`);
  }
  return response.json();
}

function readEnv(path) {
  const result = {};
  const text = readFileSync(path, 'utf8');
  for (const line of text.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) continue;
    const index = trimmed.indexOf('=');
    if (index === -1) continue;
    result[trimmed.slice(0, index)] = trimmed.slice(index + 1);
  }
  return result;
}

function mustEnv(env, key) {
  const value = env[key];
  if (!value) throw new Error(`Missing ${key}`);
  return value;
}
