export const requiredSupabaseAdapterMethods = [
  'exchangeWechatCode',
  'findOrCreateWechatIdentity',
  'issueMiniprogramSession',
  'refreshMiniprogramSession',
  'revokeMiniprogramSession',
  'getAccountState',
  'startEmailBind',
  'verifyEmailBind',
  'getMergePreview',
  'confirmMerge',
  'resolveSupabaseOwner',
  'getToday',
  'getActivePlan',
  'applyPlanToToday',
  'startStudySession',
  'submitStudyAnswer',
  'listWrongWords',
  'getWrongWordDetail',
  'getReportsOverview',
  'getTodayReward',
  'claimTodayReward',
  'getLeaderboard',
];

export function createSupabaseAdapter(_config) {
  throw new Error(
    'Supabase adapter implementation pending. This scaffold defines the scheme B boundary only.',
  );
}
