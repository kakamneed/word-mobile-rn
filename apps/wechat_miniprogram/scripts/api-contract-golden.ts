import { apiRoutes, applyParams, publicRouteKeys } from '../src/sdk/apiRoutes';

declare const require: (id: string) => any;
declare const __dirname: string;

const { readFileSync } = require('fs');
const { join } = require('path');

const requiredRoutes = [
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
] as const;

for (const routeKey of requiredRoutes) {
  const route = apiRoutes[routeKey];
  if (!route.path.startsWith('/v1/')) {
    throw new Error(`${routeKey} must be versioned under /v1`);
  }

  if (!publicRouteKeys.has(routeKey) && !route.authRequired) {
    throw new Error(`${routeKey} must require auth`);
  }
}

const leaderboardPath = applyParams(apiRoutes.leaderboard.path, {
  metric: 'weekly',
});

if (leaderboardPath !== '/v1/leaderboard/weekly') {
  throw new Error(`Unexpected leaderboard path: ${leaderboardPath}`);
}

const sdkIndex = readFileSync(join(__dirname, '..', 'src', 'sdk', 'index.ts'), 'utf8');
const transport = readFileSync(join(__dirname, '..', 'src', 'sdk', 'transport.ts'), 'utf8');
if (!sdkIndex.includes("TARO_APP_SDK_MODE === 'mock'")) throw new Error('SDK mock mode must be explicit');
if (!sdkIndex.includes('TARO_APP_API_BASE_URL is required when TARO_APP_SDK_MODE=http')) throw new Error('HTTP mode must require base URL');
if (sdkIndex.includes('shouldUseHttpSdk ?')) throw new Error('SDK must not silently choose HTTP/mock');
if (!transport.includes('statusCode === 401')) throw new Error('Transport must handle 401 refresh');
if (!transport.includes('refreshSession')) throw new Error('Transport must call refreshSession');
if (!transport.includes('hasRetried')) throw new Error('Transport must guard retry once');

console.log(
  JSON.stringify(
    {
      routeCount: Object.keys(apiRoutes).length,
      requiredRoutes: requiredRoutes.length,
      leaderboardPath,
    },
    null,
    2,
  ),
);
