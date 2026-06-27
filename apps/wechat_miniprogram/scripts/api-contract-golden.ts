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
  'savePlan',
  'applyPlan',
  'wordbooks',
  'toggleWordbook',
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
const taroConfig = readFileSync(join(__dirname, '..', 'config', 'index.ts'), 'utf8');
const transport = readFileSync(join(__dirname, '..', 'src', 'sdk', 'transport.ts'), 'utf8');
const httpClient = readFileSync(join(__dirname, '..', 'src', 'sdk', 'httpClient.ts'), 'utf8');
const accountPage = readFileSync(join(__dirname, '..', 'src', 'subpkg', 'account', 'index.tsx'), 'utf8');
const planPage = readFileSync(join(__dirname, '..', 'src', 'subpkg', 'plan', 'index.tsx'), 'utf8');
const todayPage = readFileSync(join(__dirname, '..', 'src', 'pages', 'today', 'index.tsx'), 'utf8');
const studyPage = readFileSync(join(__dirname, '..', 'src', 'pages', 'study', 'index.tsx'), 'utf8');
const packageJson = readFileSync(join(__dirname, '..', 'package.json'), 'utf8');
const app = readFileSync(join(__dirname, '..', 'src', 'app.ts'), 'utf8');
if (!sdkIndex.includes("TARO_APP_SDK_MODE === 'mock'")) throw new Error('SDK mock mode must be explicit');
if (!sdkIndex.includes('TARO_APP_API_BASE_URL is required when TARO_APP_SDK_MODE=http')) throw new Error('HTTP mode must require base URL');
if (!sdkIndex.includes('normalizeApiBaseUrl')) throw new Error('SDK must normalize API base URL');
if (!sdkIndex.includes('not /rest/v1')) throw new Error('SDK must reject Supabase REST table endpoint as backend base');
if (!taroConfig.includes('https://pmevdtgogsudnfgxjgiz.supabase.co/functions/v1')) throw new Error('HTTP mode must default to Supabase Functions root for testing');
if (sdkIndex.includes('shouldUseHttpSdk ?')) throw new Error('SDK must not silently choose HTTP/mock');
if (!sdkIndex.includes('bootstrapWechatLogin')) throw new Error('SDK must export bootstrapWechatLogin');
if (!app.includes('useLaunch') || !app.includes('bootstrapWechatLogin')) throw new Error('App launch must trigger WeChat login bootstrap');
if (!httpClient.includes('Taro.login()')) throw new Error('HTTP SDK must obtain a real WeChat login code');
if (!httpClient.includes('loginWithWechatCode')) throw new Error('HTTP SDK must exchange code through backend login');
if (!httpClient.includes('loginPromise')) throw new Error('HTTP SDK must deduplicate concurrent login attempts');
if (!transport.includes('ensureSession')) throw new Error('Transport must ensure a session before authenticated requests');
if (!transport.includes('statusCode === 401')) throw new Error('Transport must handle 401 refresh');
if (!transport.includes('refreshSession')) throw new Error('Transport must call refreshSession');
if (!transport.includes('hasRetried')) throw new Error('Transport must guard retry once');
if (!packageJson.includes('build:weapp:http')) throw new Error('Package scripts must include an explicit HTTP build');
if (!packageJson.includes('test:http-build-output')) throw new Error('Package scripts must include HTTP build output verification');
if (!packageJson.includes('test:supabase-function')) throw new Error('Package scripts must include Supabase function verification');
if (!accountPage.includes('isHttpSdk() ? null : mockAuthState')) throw new Error('Account page must not show demo auth in HTTP mode');
if (!accountPage.includes('if (!isHttpSdk())')) throw new Error('Account page must only fall back to mock auth in mock mode');
if (planPage.includes("../../sdk/client")) throw new Error('Plan page must use runtime SDK, not the mock client directly');
if (!planPage.includes("from '../../sdk'")) throw new Error('Plan page must import the runtime SDK selector');
if (!todayPage.includes('sdk.today.getTodayHomeState()')) throw new Error('Today page must refresh from cloud, not only local cache');
if (!todayPage.includes('saveStoredTodayState(cloudToday)')) throw new Error('Today page must cache cloud today state after refresh');
if (studyPage.includes('wordbookId: plan.id')) throw new Error('Study page must not use plan.id as wordbook id');
if (!studyPage.includes('sdk.plan.getWordbooks()')) throw new Error('Study page must load active wordbook before starting a session');
if (!httpClient.includes('saveActiveStudySession')) throw new Error('HTTP SDK must persist active study sessions for resume and today sync');
if (!httpClient.includes('getActiveStudySession')) throw new Error('HTTP SDK must restore active study sessions');
if (!httpClient.includes('clearActiveStudySession')) throw new Error('HTTP SDK must clear active sessions after completion or cancel');
if (!httpClient.includes('mergeActiveSessionIntoToday')) throw new Error('HTTP SDK Today reads must merge local unfinished sessions');
if (!httpClient.includes('mergeActiveSessionIntoToday(await apiRequest<TodayHomeState>')) {
  throw new Error('HTTP SDK today.getTodayHomeState must not bypass local unfinished session progress');
}

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
