import { once } from 'node:events';

import { createMiniprogramApiServer } from '../src/server.js';
import { MemorySupabaseAdapter } from '../src/supabase/memory-adapter.js';
import { createFakeWechatProvider } from '../src/wechat/fake-provider.js';

const adapter = new MemorySupabaseAdapter();
adapter.seedEmailAccount('old.http@example.com');

const server = createMiniprogramApiServer({
  adapter,
  wechatProvider: createFakeWechatProvider({
    'http-code': { openid: 'openid-http', unionid: 'unionid-http' },
  }),
});

server.listen(0, '127.0.0.1');
await once(server, 'listening');
const address = server.address();
const baseUrl = `http://127.0.0.1:${address.port}`;

try {
  const login = await post('/v1/auth/wechat-mp/login', {
    code: 'http-code',
    appVersion: '1.0.0',
    client: { platform: 'wechat_mp' },
  });

  if (!login.user.internalUserId || !login.session.accessToken) {
    throw new Error('HTTP login must return user and session');
  }

  const me = await get('/v1/me', login.session.accessToken);
  if (me.user.internalUserId !== login.user.internalUserId) {
    throw new Error('HTTP getMe must resolve bearer token user');
  }

  const bindStart = await post(
    '/v1/account/email-bind/start',
    { email: 'http@example.com' },
    login.session.accessToken,
  );
  const bindVerify = await post(
    '/v1/account/email-bind/verify',
    { challengeId: bindStart.challengeId, token: '123456' },
    login.session.accessToken,
  );
  if (bindVerify.bindingState !== 'bound') {
    throw new Error('HTTP email bind should complete');
  }

  const refresh = await post('/v1/auth/refresh', {
    refreshToken: login.session.refreshToken,
  });
  if (refresh.user.internalUserId !== login.user.internalUserId) {
    throw new Error('HTTP refresh must preserve user');
  }

  const logout = await post('/v1/auth/logout', {}, login.session.accessToken);
  if (!logout.ok) {
    throw new Error('HTTP logout should return ok');
  }

  const conflictLogin = await post('/v1/auth/wechat-mp/login', {
    code: 'http-conflict-code',
    appVersion: '1.0.0',
    client: { platform: 'wechat_mp' },
  });
  const conflictStart = await post(
    '/v1/account/email-bind/start',
    { email: 'old.http@example.com' },
    conflictLogin.session.accessToken,
  );
  const conflict = await post(
    '/v1/account/email-bind/verify',
    { challengeId: conflictStart.challengeId, token: '123456' },
    conflictLogin.session.accessToken,
  );
  if (conflict.bindingState !== 'merge_decision_required') {
    throw new Error('HTTP conflict bind must require merge decision');
  }
  const preview = await get(
    `/v1/account/merge-preview/${conflict.mergeDecisionId}`,
    conflictLogin.session.accessToken,
  );
  if (!preview.mergePreview.emailUser.hasStudyEvents) {
    throw new Error('HTTP merge preview must include email user data');
  }
  const confirm = await post(
    '/v1/account/merge-confirm',
    { mergeDecisionId: conflict.mergeDecisionId },
    conflictLogin.session.accessToken,
  );
  if (confirm.state !== 'confirmed') {
    throw new Error('HTTP merge confirm must confirm decision');
  }

  console.log(
    JSON.stringify(
      {
        internalUserId: login.user.internalUserId,
        bindState: bindVerify.bindingState,
        refreshedUser: refresh.user.internalUserId,
        logout: logout.ok,
        conflictState: conflict.bindingState,
        mergeState: confirm.state,
      },
      null,
      2,
    ),
  );
} finally {
  server.close();
}

async function get(path, accessToken) {
  const response = await fetch(`${baseUrl}${path}`, {
    headers: {
      Authorization: `Bearer ${accessToken}`,
    },
  });
  return parse(response);
}

async function post(path, body, accessToken) {
  const response = await fetch(`${baseUrl}${path}`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      ...(accessToken ? { Authorization: `Bearer ${accessToken}` } : {}),
    },
    body: JSON.stringify(body),
  });
  return parse(response);
}

async function parse(response) {
  const body = await response.json();
  if (!response.ok) {
    throw new Error(`${response.status}: ${JSON.stringify(body)}`);
  }
  return body;
}
