import { apiRoutes, publicRouteKeys } from '../src/routes.js';
import {
  accountStatePhases,
  identityProviders,
  sessionAudience,
} from '../src/auth/contracts.js';
import { requiredSupabaseAdapterMethods } from '../src/supabase/adapter.js';

const requiredRouteKeys = [
  'wechatLogin',
  'refresh',
  'logout',
  'me',
  'startEmailBind',
  'verifyEmailBind',
  'today',
  'activePlan',
  'startSession',
  'submitAnswer',
  'wrongWords',
  'reportsOverview',
  'todayReward',
  'claimReward',
  'leaderboard',
];

for (const key of requiredRouteKeys) {
  const route = apiRoutes[key];
  if (!route) {
    throw new Error(`Missing required route: ${key}`);
  }
  if (!route.path.startsWith('/v1/')) {
    throw new Error(`Route ${key} must be versioned under /v1`);
  }
  if (!publicRouteKeys.has(key) && route.authRequired !== true) {
    throw new Error(`Route ${key} must require first-party auth`);
  }
}

if (apiRoutes.wechatLogin.path !== '/v1/auth/wechat-mp/login') {
  throw new Error('WeChat login path must match Mini Program SDK contract');
}

if (sessionAudience !== 'word-mobile-miniprogram') {
  throw new Error('Unexpected Mini Program token audience');
}

for (const provider of ['wechat_mp', 'wechat_app', 'email']) {
  if (!identityProviders.includes(provider)) {
    throw new Error(`Missing identity provider: ${provider}`);
  }
}

for (const phase of ['guest_local_only', 'signed_in_active', 'merge_decision_required']) {
  if (!accountStatePhases.includes(phase)) {
    throw new Error(`Missing account state phase: ${phase}`);
  }
}

if (requiredSupabaseAdapterMethods.length < 20) {
  throw new Error('Supabase adapter boundary is too small for scheme B');
}

console.log(
  JSON.stringify(
    {
      routeCount: Object.keys(apiRoutes).length,
      requiredRouteCount: requiredRouteKeys.length,
      adapterMethodCount: requiredSupabaseAdapterMethods.length,
      audience: sessionAudience,
    },
    null,
    2,
  ),
);
