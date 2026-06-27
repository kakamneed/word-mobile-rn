import { AuthService } from '../src/auth/service.js';
import { decodeAccessToken } from '../src/auth/tokens.js';
import { MemorySupabaseAdapter } from '../src/supabase/memory-adapter.js';
import { createFakeWechatProvider } from '../src/wechat/fake-provider.js';

const adapter = new MemorySupabaseAdapter();
const service = new AuthService({
  adapter,
  wechatProvider: createFakeWechatProvider({
    'valid-code': { openid: 'openid-demo', unionid: 'unionid-demo' },
  }),
});

const login = await service.loginWithWechatCode({
  code: 'valid-code',
  appVersion: '1.0.0',
  client: { platform: 'wechat_mp', language: 'zh-CN' },
});

const payload = decodeAccessToken(login.session.accessToken);
if (payload.sub !== login.user.internalUserId) {
  throw new Error('Access token subject must match internal user id');
}

const me = await service.getMe(login.user.internalUserId);
if (me.user.internalUserId !== login.user.internalUserId) {
  throw new Error('getMe must return the logged in internal user');
}

const secondLogin = await service.loginWithWechatCode({
  code: 'valid-code',
  appVersion: '1.0.0',
  client: { platform: 'wechat_mp', language: 'zh-CN' },
});
if (secondLogin.user.internalUserId !== login.user.internalUserId) {
  throw new Error('Same WeChat openid must resolve to the same internal user');
}

const refreshed = await service.refreshSession(login.session.refreshToken);
if (refreshed.user.internalUserId !== login.user.internalUserId) {
  throw new Error('Refresh must keep the same internal user');
}

await service.logout(payload.sid);

console.log(
  JSON.stringify(
    {
      internalUserId: login.user.internalUserId,
      hasWechatBinding: login.user.hasWechatBinding,
      tokenAudience: payload.aud,
      refreshUser: refreshed.user.internalUserId,
    },
    null,
    2,
  ),
);
