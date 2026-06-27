import { SupabaseRestClient } from '../src/supabase/rest-client.js';

const calls = [];
const fetchImpl = async (url, init) => {
  calls.push({ url, init });
  return {
    ok: true,
    status: 200,
    async json() {
      return [];
    },
  };
};

const client = new SupabaseRestClient({
  url: 'https://project.supabase.co/',
  serviceRoleKey: 'service-role-key',
  fetchImpl,
});

await client.select('internal_accounts', '?select=internal_user_id');
await client.insert('miniprogram_sessions', [{ internal_user_id: 'u1' }]);
await client.rpc('example_rpc', { value: 1 });
await client.upsert('study_entry_payloads', [{ entry_id: 1 }], { onConflict: 'entry_id' });

for (const call of calls) {
  if (!call.init.headers.Authorization.includes('service-role-key')) {
    throw new Error('Expected service-role Authorization header');
  }
  if (!call.init.headers.apikey) {
    throw new Error('Expected Supabase apikey header');
  }
}

console.log(
  JSON.stringify(
    {
      callCount: calls.length,
      firstUrl: calls[0].url,
      insertMethod: calls[1].init.method,
      rpcUrl: calls[2].url,
      upsertUrl: calls[3].url,
    },
    null,
    2,
  ),
);
