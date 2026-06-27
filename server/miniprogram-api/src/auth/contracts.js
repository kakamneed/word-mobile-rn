export const sessionAudience = 'word-mobile-miniprogram';

export const identityProviders = ['wechat_mp', 'wechat_app', 'email'];

export const accountStatePhases = [
  'guest_local_only',
  'signed_in_active',
  'merge_decision_required',
];

export const wechatLoginRequestExample = {
  code: 'wx-login-code',
  appVersion: '1.0.0',
  client: {
    platform: 'wechat_mp',
    sdkVersion: 'optional',
    language: 'zh-CN',
  },
};

export const authSessionResponseExample = {
  user: {
    internalUserId: '00000000-0000-0000-0000-000000000000',
    primaryIdentity: 'wechat_mp',
    hasEmailBinding: false,
    hasWechatBinding: true,
  },
  session: {
    accessToken: 'short-lived-token',
    refreshToken: 'opaque-refresh-token',
    expiresAt: '2026-05-20T12:00:00Z',
  },
  accountState: {
    phase: 'signed_in_active',
    needsBindDecision: false,
    cloudDataState: 'empty_or_ready',
  },
};
