export class SupabaseRestClient {
  constructor({ url, serviceRoleKey, fetchImpl = globalThis.fetch }) {
    if (!url || !serviceRoleKey) {
      throw new Error('Supabase URL and service role key are required');
    }
    if (!fetchImpl) {
      throw new Error('A fetch implementation is required');
    }
    this.url = url.replace(/\/$/, '');
    this.serviceRoleKey = serviceRoleKey;
    this.fetchImpl = fetchImpl;
  }

  async select(table, query = '') {
    return this.request(`/rest/v1/${table}${query}`, {
      method: 'GET',
    });
  }

  async insert(table, rows) {
    return this.request(`/rest/v1/${table}`, {
      method: 'POST',
      body: JSON.stringify(rows),
      headers: { Prefer: 'return=representation' },
    });
  }

  async update(table, query, patch) {
    return this.request(`/rest/v1/${table}${query}`, {
      method: 'PATCH',
      body: JSON.stringify(patch),
      headers: { Prefer: 'return=representation' },
    });
  }

  async upsert(table, rows, { onConflict } = {}) {
    const query = onConflict ? `?on_conflict=${encodeURIComponent(onConflict)}` : '';
    return this.request(`/rest/v1/${table}${query}`, {
      method: 'POST',
      body: JSON.stringify(rows),
      headers: { Prefer: 'return=representation,resolution=merge-duplicates' },
    });
  }

  async rpc(name, body = {}) {
    return this.request(`/rest/v1/rpc/${name}`, {
      method: 'POST',
      body: JSON.stringify(body),
    });
  }

  async request(path, init) {
    const response = await this.fetchImpl(`${this.url}${path}`, {
      ...init,
      headers: {
        apikey: this.serviceRoleKey,
        Authorization: `Bearer ${this.serviceRoleKey}`,
        'Content-Type': 'application/json',
        ...(init.headers ?? {}),
      },
    });

    if (!response.ok) {
      const text = await response.text();
      throw new Error(`Supabase request failed ${response.status}: ${text}`);
    }

    if (response.status === 204) {
      return undefined;
    }
    return response.json();
  }
}
