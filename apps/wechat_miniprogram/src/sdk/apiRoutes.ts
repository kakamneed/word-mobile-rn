export type ApiMethod = 'GET' | 'POST' | 'PATCH' | 'DELETE';

export interface ApiRoute {
  method: ApiMethod;
  path: string;
  authRequired: boolean;
  owner: 'auth' | 'today' | 'plan' | 'study' | 'wrongWords' | 'reports' | 'rewards' | 'leaderboard' | 'crocBti';
}

export const apiRoutes = {
  me: { method: 'GET', path: '/v1/me', authRequired: true, owner: 'auth' },
  wechatLogin: { method: 'POST', path: '/v1/auth/wechat-mp/login', authRequired: false, owner: 'auth' },
  refresh: { method: 'POST', path: '/v1/auth/refresh', authRequired: false, owner: 'auth' },
  logout: { method: 'POST', path: '/v1/auth/logout', authRequired: true, owner: 'auth' },
  startEmailBind: { method: 'POST', path: '/v1/account/email-bind/start', authRequired: true, owner: 'auth' },
  verifyEmailBind: { method: 'POST', path: '/v1/account/email-bind/verify', authRequired: true, owner: 'auth' },
  mergePreview: { method: 'GET', path: '/v1/account/merge-preview/:mergeDecisionId', authRequired: true, owner: 'auth' },
  mergeConfirm: { method: 'POST', path: '/v1/account/merge-confirm', authRequired: true, owner: 'auth' },
  today: { method: 'GET', path: '/v1/today', authRequired: true, owner: 'today' },
  activePlan: { method: 'GET', path: '/v1/plan/active', authRequired: true, owner: 'plan' },
  savePlan: { method: 'PATCH', path: '/v1/plan/:planId', authRequired: true, owner: 'plan' },
  applyPlan: { method: 'POST', path: '/v1/plan/apply-to-today', authRequired: true, owner: 'plan' },
  wordbooks: { method: 'GET', path: '/v1/wordbooks', authRequired: true, owner: 'plan' },
  toggleWordbook: { method: 'POST', path: '/v1/wordbooks/:wordbookId/toggle', authRequired: true, owner: 'plan' },
  crocBtiProfile: { method: 'GET', path: '/v1/croc-bti/profile', authRequired: true, owner: 'crocBti' },
  saveCrocBtiProfile: { method: 'POST', path: '/v1/croc-bti/profile', authRequired: true, owner: 'crocBti' },
  startSession: { method: 'POST', path: '/v1/study/sessions', authRequired: true, owner: 'study' },
  resumeSessionHint: { method: 'GET', path: '/v1/study/resume-hint', authRequired: true, owner: 'study' },
  submitAnswer: { method: 'POST', path: '/v1/study/answers', authRequired: true, owner: 'study' },
  markStudyEntryMastered: { method: 'POST', path: '/v1/study/mastered', authRequired: true, owner: 'study' },
  acceptDisputedMeaning: { method: 'POST', path: '/v1/study/disputed-meaning/accept', authRequired: true, owner: 'study' },
  completeSession: { method: 'POST', path: '/v1/study/sessions/:sessionId/complete', authRequired: true, owner: 'study' },
  cancelSession: { method: 'POST', path: '/v1/study/sessions/:sessionId/cancel', authRequired: true, owner: 'study' },
  wrongWords: { method: 'GET', path: '/v1/wrong-words', authRequired: true, owner: 'wrongWords' },
  wrongWordDetail: { method: 'GET', path: '/v1/wrong-words/:entryId', authRequired: true, owner: 'wrongWords' },
  reportsOverview: { method: 'GET', path: '/v1/reports/overview', authRequired: true, owner: 'reports' },
  todayReward: { method: 'GET', path: '/v1/rewards/today', authRequired: true, owner: 'rewards' },
  claimReward: { method: 'POST', path: '/v1/rewards/today/claim', authRequired: true, owner: 'rewards' },
  leaderboard: { method: 'GET', path: '/v1/leaderboard/:metric', authRequired: true, owner: 'leaderboard' },
} as const satisfies Record<string, ApiRoute>;

export type ApiRouteKey = keyof typeof apiRoutes;

export const publicRouteKeys = new Set<ApiRouteKey>(['wechatLogin', 'refresh']);

export function applyParams(
  path: string,
  params: Record<string, string | number> = {},
) {
  return Object.entries(params).reduce(
    (nextPath, [key, value]) =>
      nextPath.replace(`:${key}`, encodeURIComponent(String(value))),
    path,
  );
}
