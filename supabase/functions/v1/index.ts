type JsonRecord = Record<string, unknown>;

interface AccountContext {
  internalUserId: string;
  supabaseUserId?: string;
}

type StudyMode =
  | 'newWord'
  | 'review'
  | 'mixedTest'
  | 'wrongWordReinforcement'
  | 'rootAffix';

interface StudyQuestion {
  questionId: string;
  questionType: string;
  entrySourceId: string;
  entryId?: number;
  word: string;
  prompt: string;
  partOfSpeech?: string;
  phoneticUk?: string;
  acceptedMeanings: string[];
  choices?: Array<{ label: string; value: string; text: string }>;
  correctChoiceLabel?: string;
  questionIndex: number;
  totalQuestions: number;
  exampleSentence?: string;
  exampleTranslation?: string;
  hasHint: boolean;
  userHint?: string;
}

interface StudyResult {
  questionId: string;
  entrySourceId: string;
  questionType: string;
  userResponse: string;
  normalizedResponse?: string;
  correctAnswer: string;
  outcome: 'correct' | 'fuzzyCorrect' | 'incorrect' | 'skipped';
  responseTimeMs: number;
  answeredAt: string;
}

interface ActiveStudySession {
  sessionId: string;
  ownerId: string;
  mode: StudyMode;
  startedAt: string;
  questions: StudyQuestion[];
  answeredQuestions: Array<{ question: StudyQuestion; result: StudyResult }>;
}

const corsHeaders = {
  'access-control-allow-origin': '*',
  'access-control-allow-headers': 'authorization, x-client-info, apikey, content-type',
  'access-control-allow-methods': 'GET, POST, PATCH, OPTIONS',
};

const supabaseUrl = Deno.env.get('SUPABASE_URL') ?? '';
const serviceRoleKey =
  Deno.env.get('SUPABASE_SERVICE_ROLE_KEY') ??
  Deno.env.get('WORD_SERVICE_ROLE_KEY') ??
  '';

const defaultPlan = {
  id: 1,
  name: '鳄甲卫',
  newWordsPerDay: 20,
  reviewWordsPerDay: 28,
  mixedTestPerDay: 34,
  wrongWordTestPerDay: 26,
  rootAffixPerDay: 4,
  growthRuleEnabled: true,
  growthRuleMode: 'shared',
  growthIntervalDays: 7,
  growthIncrement: 5,
  sharedGrowthRule: { intervalDays: 7, increment: 5 },
  growthRulesByMode: {
    newWord: { intervalDays: 7, increment: 4 },
    review: { intervalDays: 7, increment: 5 },
    mixedTest: { intervalDays: 7, increment: 5 },
    wrongWordReinforcement: { intervalDays: 7, increment: 3 },
    rootAffix: { intervalDays: 14, increment: 1 },
  },
};

const defaultWordbooks = [
  { id: 3, code: 'kaoyan', name: '考研', category: 'exam', totalEntries: 5500, isActive: true },
];

const wordbookCatalog = [
  { id: 1984431478, code: 'cet4', name: 'CET-4', category: 'exam', totalEntries: 2607, isActive: false },
  { id: 1984433400, code: 'cet6', name: 'CET-6', category: 'exam', totalEntries: 2345, isActive: false },
  { id: 1018327649, code: 'kaoyan', name: 'KaoYan', category: 'exam', totalEntries: 3728, isActive: true },
  { id: 3264070878, code: 'medical', name: 'Medical English', category: 'specialized', totalEntries: 2857, isActive: false },
];

const legacyWordbookIds: Record<number, number> = {
  1: 1984431478,
  2: 1984433400,
  3: 1018327649,
  4: 3264070878,
};

const activeStudySessions = new Map<string, ActiveStudySession>();

const rewardCatalog = [
  {
    rewardId: 'reward-001-com-hihonor-photos-20260329190156-edit-752572937',
    title: 'Reward 001',
    imageUrl: '/assets/rewards/webp_q60_540/reward_001.webp',
    mimeType: 'image/webp',
  },
  {
    rewardId: 'reward-009-com-ss-android-ugc-aweme-20250307152314',
    title: 'Reward 009',
    imageUrl: '/assets/rewards/webp_q60_540/reward_009.webp',
    mimeType: 'image/webp',
  },
  {
    rewardId: 'reward-018-com-ss-android-ugc-aweme-20250323151148',
    title: 'Reward 018',
    imageUrl: '/assets/rewards/webp_q60_540/reward_018.webp',
    mimeType: 'image/webp',
  },
  {
    rewardId: 'reward-037-com-ss-android-ugc-aweme-20250412103101',
    title: 'Reward 037',
    imageUrl: '/assets/rewards/webp_q60_540/reward_037.webp',
    mimeType: 'image/webp',
  },
  {
    rewardId: 'reward-054-com-ss-android-ugc-aweme-20250531184353',
    title: 'Reward 054',
    imageUrl: '/assets/rewards/webp_q60_540/reward_054.webp',
    mimeType: 'image/webp',
  },
  {
    rewardId: 'reward-063-com-ss-android-ugc-aweme-20250613185422',
    title: 'Reward 063',
    imageUrl: '/assets/rewards/webp_q60_540/reward_063.webp',
    mimeType: 'image/webp',
  },
];

function json(status: number, body: unknown) {
  return new Response(JSON.stringify(body), {
    status,
    headers: {
      ...corsHeaders,
      'content-type': 'application/json; charset=utf-8',
    },
  });
}

function error(status: number, code: string, message: string) {
  return json(status, { code, message });
}

function normalizePath(requestUrl: string) {
  const url = new URL(requestUrl);
  return url.pathname
    .replace(/^\/functions\/v1\/v1/, '/v1')
    .replace(/^\/v1\/v1/, '/v1');
}

async function readJson(request: Request): Promise<JsonRecord> {
  if (request.method === 'GET') return {};
  const text = await request.text();
  if (!text.trim()) return {};
  return JSON.parse(text) as JsonRecord;
}

function base64UrlEncode(input: string) {
  return btoa(input).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/g, '');
}

function base64UrlDecode(input: string) {
  const padded = input.replace(/-/g, '+').replace(/_/g, '/').padEnd(Math.ceil(input.length / 4) * 4, '=');
  return atob(padded);
}

function createAccessToken(context: AccountContext, sessionId: string) {
  const now = Math.floor(Date.now() / 1000);
  return base64UrlEncode(
    JSON.stringify({
      sub: context.internalUserId,
      sid: sessionId,
      supabase_user_id: context.supabaseUserId,
      aud: 'word-miniprogram',
      iat: now,
      exp: now + 15 * 60,
      identity_provider: 'wechat_mp',
    }),
  );
}

function createRefreshToken(context: AccountContext) {
  return `refresh_${base64UrlEncode(JSON.stringify({
    sub: context.internalUserId,
    supabase_user_id: context.supabaseUserId,
    sid: `wmp_${crypto.randomUUID()}`,
    iat: Math.floor(Date.now() / 1000),
  }))}`;
}

function authContext(request: Request): AccountContext | undefined {
  const header = request.headers.get('authorization') ?? '';
  const token = header.replace(/^Bearer\s+/i, '').trim();
  if (!token) return undefined;
  try {
    const payload = JSON.parse(base64UrlDecode(token)) as {
      sub?: string;
      sid?: string;
      exp?: number;
      supabase_user_id?: string;
    };
    if (!payload.sub || !payload.sid || (payload.exp ?? 0) < Math.floor(Date.now() / 1000)) {
      return undefined;
    }
    return { internalUserId: payload.sub, supabaseUserId: payload.supabase_user_id };
  } catch {
    return undefined;
  }
}

function requireServiceRole() {
  if (!supabaseUrl || !serviceRoleKey) {
    throw new Error('SUPABASE_URL and SUPABASE_SERVICE_ROLE_KEY are required for cloud sync endpoints');
  }
}

async function supabaseRequest<T>(
  method: string,
  path: string,
  body?: unknown,
  query?: Record<string, string>,
) {
  requireServiceRole();
  const url = new URL(`${supabaseUrl}/rest/v1/${path}`);
  Object.entries(query ?? {}).forEach(([key, value]) => url.searchParams.set(key, value));
  const response = await fetch(url, {
    method,
    headers: {
      apikey: serviceRoleKey,
      authorization: `Bearer ${serviceRoleKey}`,
      'content-type': 'application/json',
      prefer: 'return=representation,resolution=merge-duplicates',
    },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const text = await response.text();
  const payload = text ? JSON.parse(text) : null;
  if (!response.ok) {
    const message = payload?.message ?? payload?.hint ?? `${method} ${path} failed with ${response.status}`;
    throw new Error(message);
  }
  return payload as T;
}

async function findAuthUserByEmail(email: string) {
  requireServiceRole();
  const url = new URL(`${supabaseUrl}/auth/v1/admin/users`);
  url.searchParams.set('page', '1');
  url.searchParams.set('per_page', '100');
  const response = await fetch(url, {
    headers: {
      apikey: serviceRoleKey,
      authorization: `Bearer ${serviceRoleKey}`,
    },
  });
  const payload = await response.json() as { users?: Array<{ id: string; email?: string }> };
  if (!response.ok) throw new Error('Failed to query Supabase Auth users');
  return (payload.users ?? []).find((user) => user.email?.toLowerCase() === email);
}

function accountState(context: AccountContext, bindings?: { email?: boolean; wechat?: boolean }) {
  return {
    user: {
      internalUserId: context.internalUserId,
      primaryIdentity: 'wechat_mp',
      hasEmailBinding: Boolean(bindings?.email ?? context.supabaseUserId),
      hasWechatBinding: bindings?.wechat ?? true,
    },
    accountState: {
      phase: context.supabaseUserId ? 'signed_in_active' : 'signed_in_active',
      needsBindDecision: false,
      cloudDataState: context.supabaseUserId ? 'empty_or_ready' : 'unavailable',
    },
  };
}

async function exchangeWechatCode(code: string) {
  const appid = Deno.env.get('WECHAT_MP_APPID');
  const secret = Deno.env.get('WECHAT_MP_SECRET');
  if (!appid || !secret) return { openid: `dev_${code.slice(0, 18)}` };

  const url = new URL('https://api.weixin.qq.com/sns/jscode2session');
  url.searchParams.set('appid', appid);
  url.searchParams.set('secret', secret);
  url.searchParams.set('js_code', code);
  url.searchParams.set('grant_type', 'authorization_code');
  const response = await fetch(url);
  const payload = await response.json() as { openid?: string; unionid?: string; errcode?: number; errmsg?: string };
  if (!response.ok || payload.errcode || !payload.openid) {
    throw new Error(payload.errmsg ?? `WeChat code2session failed with ${response.status}`);
  }
  return { openid: payload.openid, unionid: payload.unionid };
}

async function upsertWechatAccount(identity: { openid: string; unionid?: string }) {
  const subject = identity.openid;
  const existing = await supabaseRequest<Array<{
    internal_user_id: string;
    supabase_user_id?: string;
    provider_subject_secondary?: string;
  }>>(
    'GET',
    'user_identities',
    undefined,
    {
      provider: 'eq.wechat_mp',
      provider_subject: `eq.${subject}`,
      select: 'internal_user_id,supabase_user_id,provider_subject_secondary',
      limit: '1',
    },
  );

  let internalUserId = existing[0]?.internal_user_id;
  if (!internalUserId) {
    const accounts = await supabaseRequest<Array<{ internal_user_id: string }>>(
      'POST',
      'internal_accounts',
      { primary_identity: 'wechat_mp', last_login_at: new Date().toISOString() },
    );
    internalUserId = accounts[0].internal_user_id;
  } else {
    await supabaseRequest('PATCH', 'internal_accounts', { last_login_at: new Date().toISOString() }, {
      internal_user_id: `eq.${internalUserId}`,
    });
  }

  const identityRows = await supabaseRequest<Array<{ supabase_user_id?: string }>>(
    'POST',
    'user_identities',
    {
      internal_user_id: internalUserId,
      provider: 'wechat_mp',
      provider_subject: subject,
      provider_subject_secondary: identity.unionid ?? null,
      is_verified: true,
      metadata_json: { source: 'wechat_mp_login' },
      last_seen_at: new Date().toISOString(),
    },
    { on_conflict: 'provider,provider_subject' },
  );

  return {
    internalUserId,
    supabaseUserId: identityRows[0]?.supabase_user_id ?? existing[0]?.supabase_user_id,
  };
}

async function getAccountContext(internalUserId: string): Promise<AccountContext> {
  const accounts = await supabaseRequest<Array<{ supabase_owner_user_id?: string }>>(
    'GET',
    'internal_accounts',
    undefined,
    {
      internal_user_id: `eq.${internalUserId}`,
      select: 'supabase_owner_user_id',
      limit: '1',
    },
  );
  return { internalUserId, supabaseUserId: accounts[0]?.supabase_owner_user_id };
}

async function getBindingFlags(context: AccountContext) {
  const identities = await supabaseRequest<Array<{ provider: string }>>(
    'GET',
    'user_identities',
    undefined,
    {
      internal_user_id: `eq.${context.internalUserId}`,
      select: 'provider',
    },
  );
  return {
    wechat: identities.some((row) => row.provider === 'wechat_mp'),
    email: identities.some((row) => row.provider === 'email') || Boolean(context.supabaseUserId),
  };
}

function planFromRow(row?: Record<string, unknown>) {
  if (!row) return defaultPlan;
  return {
    id: 1,
    name: String(row.name ?? defaultPlan.name),
    newWordsPerDay: Number(row.new_words_per_day ?? defaultPlan.newWordsPerDay),
    reviewWordsPerDay: Number(row.review_words_per_day ?? defaultPlan.reviewWordsPerDay),
    mixedTestPerDay: Number(row.mixed_test_per_day ?? defaultPlan.mixedTestPerDay),
    wrongWordTestPerDay: Number(row.wrong_word_test_per_day ?? defaultPlan.wrongWordTestPerDay),
    rootAffixPerDay: Number(row.root_affix_per_day ?? defaultPlan.rootAffixPerDay),
    growthRuleEnabled: true,
    growthRuleMode: String(row.growth_rule_mode ?? defaultPlan.growthRuleMode),
    growthIntervalDays: defaultPlan.growthIntervalDays,
    growthIncrement: defaultPlan.growthIncrement,
    sharedGrowthRule: row.shared_growth_rule ?? defaultPlan.sharedGrowthRule,
    growthRulesByMode: row.growth_rules_by_mode ?? defaultPlan.growthRulesByMode,
  };
}

async function savePlan(context: AccountContext, body: JsonRecord) {
  if (!context.supabaseUserId) {
    return error(409, 'EMAIL_BIND_REQUIRED', 'WeChat account must bind an email account before saving cloud plan data');
  }
  const row = {
    user_id: context.supabaseUserId,
    name: String(body.name ?? defaultPlan.name),
    new_words_per_day: Number(body.newWordsPerDay ?? defaultPlan.newWordsPerDay),
    review_words_per_day: Number(body.reviewWordsPerDay ?? defaultPlan.reviewWordsPerDay),
    mixed_test_per_day: Number(body.mixedTestPerDay ?? defaultPlan.mixedTestPerDay),
    wrong_word_test_per_day: Number(body.wrongWordTestPerDay ?? defaultPlan.wrongWordTestPerDay),
    root_affix_per_day: Number(body.rootAffixPerDay ?? defaultPlan.rootAffixPerDay),
    growth_rule_mode: String(body.growthRuleMode ?? defaultPlan.growthRuleMode),
    shared_growth_rule: body.sharedGrowthRule ?? defaultPlan.sharedGrowthRule,
    growth_rules_by_mode: body.growthRulesByMode ?? defaultPlan.growthRulesByMode,
    version: Date.now(),
  };
  const rows = await supabaseRequest<Array<Record<string, unknown>>>(
    'POST',
    'plan_configs',
    row,
    { on_conflict: 'user_id' },
  );
  return json(200, planFromRow(rows[0] ?? row));
}

async function applyPlanToToday(context: AccountContext) {
  return json(200, await getActivePlan(context));
}

async function getWordbooks(context: AccountContext) {
  if (!context.supabaseUserId) return wordbookCatalog;
  const prefs = await supabaseRequest<Array<{ wordbook_id: number; is_active: boolean }>>(
    'GET',
    'wordbook_preferences',
    undefined,
    {
      user_id: `eq.${context.supabaseUserId}`,
      select: 'wordbook_id,is_active',
    },
  );
  const catalogIds = new Set(wordbookCatalog.map((wordbook) => wordbook.id));
  const activeIds = new Set(
    prefs
      .filter((pref) => pref.is_active)
      .map((pref) => legacyWordbookIds[Number(pref.wordbook_id)] ?? Number(pref.wordbook_id))
      .filter((id) => catalogIds.has(id)),
  );
  return wordbookCatalog.map((wordbook) => ({
    ...wordbook,
    isActive: activeIds.size ? activeIds.has(wordbook.id) : wordbook.isActive,
  }));
}

async function setActiveWordbook(context: AccountContext, wordbookId: number, isActive: boolean) {
  if (!context.supabaseUserId) {
    return error(409, 'EMAIL_BIND_REQUIRED', 'WeChat account must bind an email account before saving wordbook preferences');
  }
  if (isActive) {
    await supabaseRequest('PATCH', 'wordbook_preferences', { is_active: false }, {
      user_id: `eq.${context.supabaseUserId}`,
    });
  }
  await supabaseRequest(
    'POST',
    'wordbook_preferences',
    {
      user_id: context.supabaseUserId,
      wordbook_id: wordbookId,
      is_active: isActive,
      updated_at: new Date().toISOString(),
    },
    { on_conflict: 'user_id,wordbook_id' },
  );
  return json(200, { ok: true });
}

async function getActivePlan(context: AccountContext) {
  if (!context.supabaseUserId) return defaultPlan;
  const rows = await supabaseRequest<Array<Record<string, unknown>>>(
    'GET',
    'plan_configs',
    undefined,
    {
      user_id: `eq.${context.supabaseUserId}`,
      select: '*',
      limit: '1',
    },
  );
  return planFromRow(rows[0]);
}

async function getReports(context: AccountContext) {
  if (!context.supabaseUserId) {
    return {
      totalStudyDays: 0,
      totalWordsLearned: 0,
      totalQuestionsAnswered: 0,
      overallAccuracy: 0,
      streakInfo: { currentStreak: 0, longestStreak: 0 },
      dailySeries: [],
      modeBreakdown: [],
      modeSeries: {},
    };
  }
  const rows = await supabaseRequest<Array<{ snapshot_date: string; payload_json: Record<string, unknown> }>>(
    'GET',
    'report_snapshots',
    undefined,
    {
      user_id: `eq.${context.supabaseUserId}`,
      select: 'snapshot_date,payload_json',
      order: 'snapshot_date.desc',
      limit: '7',
    },
  );
  const latest = rows[0]?.payload_json;
  if (!latest) {
    return {
      totalStudyDays: 0,
      totalWordsLearned: 0,
      totalQuestionsAnswered: 0,
      overallAccuracy: 0,
      streakInfo: { currentStreak: 0, longestStreak: 0 },
      dailySeries: [],
      last7Days: [],
      modeBreakdown: [],
      modeSeries: {},
    };
  }

  const chronological = [...rows].reverse();
  const snapshotSeries = chronological
    .map((row) => {
      const payload = row.payload_json ?? {};
      const series = Array.isArray(payload.dailySeries)
        ? payload.dailySeries
        : Array.isArray(payload.last7Days)
          ? payload.last7Days
          : [];
      const byDate = series.find((item) =>
        typeof item === 'object' &&
        item !== null &&
        (item as JsonRecord).date === row.snapshot_date
      ) as JsonRecord | undefined;
      return byDate ?? {
        date: row.snapshot_date,
        totalQuestions: payload.totalQuestionsAnswered ?? 0,
        correctCount: Math.round(
          Number(payload.totalQuestionsAnswered ?? 0) *
            (Number(payload.overallAccuracy ?? 0) / 100),
        ),
        accuracyPercent: Number(payload.overallAccuracy ?? 0),
      };
    })
    .filter((item) => Boolean((item as JsonRecord).date));

  const dailySeries = snapshotSeries.length
    ? snapshotSeries
    : (Array.isArray(latest.dailySeries)
        ? latest.dailySeries
        : Array.isArray(latest.last7Days)
          ? latest.last7Days
          : []);

  return {
    ...latest,
    totalStudyDays: Number(latest.totalStudyDays ?? dailySeries.length),
    totalWordsLearned: Number(latest.totalWordsLearned ?? 0),
    totalQuestionsAnswered: Number(latest.totalQuestionsAnswered ?? 0),
    overallAccuracy: Number(latest.overallAccuracy ?? 0),
    streakInfo: latest.streakInfo ?? { currentStreak: 0, longestStreak: 0 },
    dailySeries,
    last7Days: Array.isArray(latest.last7Days) ? latest.last7Days : dailySeries,
    modeBreakdown: Array.isArray(latest.modeBreakdown) ? latest.modeBreakdown : [],
    modeSeries: latest.modeSeries ?? {},
  };
}

function meaningText(value: unknown) {
  if (typeof value === 'string') return value;
  if (value && typeof value === 'object') {
    const record = value as JsonRecord;
    return String(
      record.meaningCn ??
        record.meaning_cn ??
        record.meaning ??
        record.definitionCn ??
        record.definition ??
        record.text ??
        record.value ??
        '',
    ).trim();
  }
  return '';
}

function normalizeMeanings(value: unknown) {
  if (Array.isArray(value)) {
    return value
      .map((item) => {
        const text = meaningText(item);
        return text || undefined;
      })
      .filter(Boolean);
  }
  const text = meaningText(value);
  return text ? [text] : [];
}

function entryFromPayload(row: Record<string, unknown>) {
  const payload =
    (row.entry_payload as JsonRecord | undefined) ??
    (row.payload_json as JsonRecord | undefined) ??
    (row.metadata_json as JsonRecord | undefined) ??
    {};
  const entryId = Number(row.entry_id ?? payload.entryId ?? payload.entry_id ?? 0);
  const word = String(
    payload.word ??
      payload.lemma ??
      row.word ??
      (entryId > 0 ? `entry-${entryId}` : ''),
  );
  return {
    entryId,
    word,
    lemma: String(payload.lemma ?? word),
    phoneticUs: payload.phoneticUs ?? payload.phonetic_us ?? payload.phonetic_us,
    phoneticUk: payload.phoneticUk ?? payload.phonetic_uk ?? payload.phonetic_uk,
    partOfSpeech: String(payload.partOfSpeech ?? payload.part_of_speech ?? ''),
    meanings: normalizeMeanings(payload.meanings ?? payload.meaning ?? payload.meaningCn),
    examples: Array.isArray(payload.examples) ? payload.examples : [],
    entryKind: String(payload.entryKind ?? payload.entry_kind ?? 'word'),
  };
}

async function entryPayloadMap(context: AccountContext, entryIds: number[]) {
  const ids = [...new Set(entryIds.filter((id) => Number.isFinite(id) && id > 0))];
  const result = new Map<number, ReturnType<typeof entryFromPayload>>();
  if (!context.supabaseUserId || !ids.length) return result;

  const rows = await supabaseRequest<Array<Record<string, unknown>>>(
    'GET',
    'study_events',
    undefined,
    {
      user_id: `eq.${context.supabaseUserId}`,
      select: 'payload_json',
      order: 'occurred_at.desc',
      limit: '1000',
    },
  ).catch(() => []);

  for (const row of rows) {
    const payload = row.payload_json as JsonRecord | undefined;
    const candidates = [
      payload?.question,
      payload?.currentQuestion,
      payload?.entryPayload,
      payload,
    ].filter(Boolean) as JsonRecord[];
    for (const candidate of candidates) {
      const entry = entryFromPayload(candidate as Record<string, unknown>);
      if (ids.includes(entry.entryId) && !result.has(entry.entryId)) {
        result.set(entry.entryId, entry);
      }
    }
    if (result.size === ids.length) break;
  }

  return result;
}

async function studyPointStats(context: AccountContext) {
  if (!context.supabaseUserId) return { learned: 0 };
  const rows = await supabaseRequest<Array<{ entry_id: number }>>(
    'GET',
    'study_word_points',
    undefined,
    {
      user_id: `eq.${context.supabaseUserId}`,
      select: 'entry_id',
      correct_count: 'gt.0',
      limit: '1000',
    },
  ).catch(() => []);
  return { learned: new Set(rows.map((row) => row.entry_id)).size };
}

function emptyReports() {
  return {
    totalStudyDays: 0,
    totalWordsLearned: 0,
    totalQuestionsAnswered: 0,
    overallAccuracy: 0,
    streakInfo: { currentStreak: 0, longestStreak: 0 },
    dailySeries: [],
    last7Days: [],
    modeBreakdown: [],
    modeSeries: {},
  };
}

async function getWrongWords(context: AccountContext) {
  if (!context.supabaseUserId) return [];
  const rows = await supabaseRequest<Array<Record<string, unknown>>>(
    'GET',
    'wrong_word_entries',
    undefined,
    {
      user_id: `eq.${context.supabaseUserId}`,
      select: 'entry_id,error_count,last_wrong_at,priority_score,hint_text,hint_source,hint_updated_at',
      order: 'priority_score.desc',
      limit: '200',
    },
  );
  const payloads = await entryPayloadMap(context, rows.map((row) => Number(row.entry_id)));
  return rows.map((row) => ({
    ...entryFromPayload({
      ...(payloads.get(Number(row.entry_id)) ?? {}),
      entry_id: row.entry_id,
    }),
    errorCount: Number(row.error_count ?? 0),
    lastWrongAt: row.last_wrong_at,
    priorityScore: Number(row.priority_score ?? 0),
    isActive: true,
    hasHint: Boolean(row.hint_text),
    userHint: row.hint_text,
    hintSource: row.hint_source,
    hintUpdatedAt: row.hint_updated_at,
    hintSuggestions: [],
  }));
}

const modeCountKeys: Record<StudyMode, keyof typeof defaultPlan> = {
  newWord: 'newWordsPerDay',
  review: 'reviewWordsPerDay',
  mixedTest: 'mixedTestPerDay',
  wrongWordReinforcement: 'wrongWordTestPerDay',
  rootAffix: 'rootAffixPerDay',
};

function emptyModeProgress(activePlan: JsonRecord) {
  return (Object.keys(modeCountKeys) as StudyMode[]).reduce((result, mode) => {
    result[mode] = {
      completed: 0,
      total: modeQuestionTotal(activePlan, mode),
      isComplete: false,
    };
    return result;
  }, {} as Record<StudyMode, { completed: number; total: number; isComplete: boolean }>);
}

async function todayModeProgress(context: AccountContext, activePlan: JsonRecord, todayDate: string) {
  const progress = emptyModeProgress(activePlan);
  if (context.supabaseUserId) {
    const rows = await supabaseRequest<Array<{ mode: StudyMode; attempt_count: number }>>(
      'GET',
      'study_word_points',
      undefined,
      {
        user_id: `eq.${context.supabaseUserId}`,
        point_date: `eq.${todayDate}`,
        select: 'mode,attempt_count',
        limit: '1000',
      },
    ).catch(() => []);
    for (const row of rows) {
      const mode = String(row.mode) as StudyMode;
      if (!progress[mode]) continue;
      progress[mode].completed += Number(row.attempt_count ?? 0);
    }
  }

  for (const session of activeStudySessions.values()) {
    if (session.ownerId !== context.internalUserId) continue;
    const modeProgress = progress[session.mode];
    if (!modeProgress) continue;
    if (modeProgress.total <= 0) continue;
    modeProgress.completed = Math.max(modeProgress.completed, session.answeredQuestions.length);
    modeProgress.total = Math.max(modeProgress.total, session.questions.length);
  }

  for (const mode of Object.keys(progress) as StudyMode[]) {
    progress[mode].completed = Math.min(progress[mode].completed, progress[mode].total);
    progress[mode].isComplete = progress[mode].total > 0 && progress[mode].completed >= progress[mode].total;
  }
  return progress;
}

function resumeHintFromActiveSession(context: AccountContext, activePlan: JsonRecord) {
  const session = [...activeStudySessions.values()].find((candidate) =>
    candidate.ownerId === context.internalUserId &&
    modeQuestionTotal(activePlan, candidate.mode) > 0 &&
    candidate.answeredQuestions.length < candidate.questions.length
  );
  if (!session) return { hasResume: false };
  const nextQuestion = session.questions[session.answeredQuestions.length];
  return {
    hasResume: true,
    sessionId: session.sessionId,
    mode: session.mode,
    current: Math.min(session.answeredQuestions.length + 1, session.questions.length),
    total: session.questions.length,
    word: nextQuestion?.word,
  };
}

async function getToday(context: AccountContext) {
  const reports = await getReports(context);
  const activePlan = await getActivePlan(context);
  const wordbooks = await getWordbooks(context);
  const todayDate = todayDateString();
  const modeProgress = await todayModeProgress(context, activePlan as JsonRecord, todayDate);
  const completedTasks = Object.values(modeProgress).filter((progress) => progress.isComplete).length;
  const totalTasks = 5;
  return {
    todayDate,
    activePlan,
    wordbooks,
    dailyProgress: {
      completedTasks,
      totalTasks,
      accuracyPercent: Number(reports.overallAccuracy ?? 0),
      streakDays: Number((reports.streakInfo as JsonRecord | undefined)?.currentStreak ?? 0),
    },
    modeProgress,
    resumeHint: resumeHintFromActiveSession(context, activePlan as JsonRecord),
  };
}

function todayDateString() {
  return new Date().toISOString().slice(0, 10);
}

function rewardAssetForId(rewardId: string) {
  return rewardCatalog.find((asset) => asset.rewardId === rewardId) ?? {
    rewardId,
    title: rewardId,
    imageUrl: '/assets/rewards/webp_q60_540/reward_001.webp',
    mimeType: 'image/webp',
  };
}

function rewardStateFromRow(row: JsonRecord | undefined, todayDate: string, canClaim: boolean) {
  if (!row) {
    return {
      todayDate,
      rewardId: undefined,
      claimedAt: undefined,
      canClaim,
      asset: undefined,
    };
  }
  const rewardId = String(row.reward_id ?? '');
  const fallbackAsset = rewardAssetForId(rewardId);
  return {
    todayDate,
    rewardId,
    claimedAt: String(row.claimed_at ?? ''),
    canClaim: false,
    asset: {
      rewardId,
      title: String(row.reward_title ?? fallbackAsset.title),
      imageUrl: String(row.reward_image_url ?? fallbackAsset.imageUrl),
      mimeType: String(row.reward_mime_type ?? fallbackAsset.mimeType),
    },
  };
}

async function hasCompletedTodayTasks(context: AccountContext, todayDate: string) {
  const activePlan = await getActivePlan(context) as JsonRecord;
  const progress = await todayModeProgress(context, activePlan, todayDate);
  const enabledModes = Object.values(progress).filter((modeProgress) => modeProgress.total > 0);
  return enabledModes.length > 0 && enabledModes.every((modeProgress) => modeProgress.isComplete);
}

async function getTodayRewardState(context: AccountContext) {
  const todayDate = todayDateString();
  const canClaim = context.supabaseUserId
    ? await hasCompletedTodayTasks(context, todayDate)
    : false;
  if (!context.supabaseUserId) {
    return rewardStateFromRow(undefined, todayDate, canClaim);
  }
  const rows = await supabaseRequest<JsonRecord[]>(
    'GET',
    'daily_reward_claims',
    undefined,
    {
      user_id: `eq.${context.supabaseUserId}`,
      reward_date: `eq.${todayDate}`,
      select: '*',
      limit: '1',
    },
  ).catch(() => []);
  return rewardStateFromRow(rows[0], todayDate, canClaim);
}

async function claimTodayReward(context: AccountContext) {
  const todayDate = todayDateString();
  if (!context.supabaseUserId) {
    return error(409, 'EMAIL_BIND_REQUIRED', 'Email binding is required before claiming cloud rewards');
  }

  const current = await getTodayRewardState(context) as JsonRecord;
  if (typeof current.rewardId === 'string' && current.rewardId) {
    return json(200, current);
  }

  if (!await hasCompletedTodayTasks(context, todayDate)) {
    return error(409, 'TODAY_TASKS_INCOMPLETE', 'Complete all enabled today tasks before claiming a reward');
  }

  const picked = rewardCatalog[stableHash(`${context.supabaseUserId}:${todayDate}`) % rewardCatalog.length];
  const rows = await supabaseRequest<JsonRecord[]>(
    'POST',
    'daily_reward_claims',
    {
      user_id: context.supabaseUserId,
      reward_date: todayDate,
      reward_id: picked.rewardId,
      reward_title: picked.title,
      reward_image_url: picked.imageUrl,
      reward_mime_type: picked.mimeType,
    },
    { on_conflict: 'user_id,reward_date' },
  );
  return json(200, rewardStateFromRow(rows[0], todayDate, false));
}

async function startEmailBind(context: AccountContext, body: JsonRecord) {
  const email = String(body.email ?? '').trim().toLowerCase();
  if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)) {
    return error(400, 'INVALID_EMAIL', '请输入有效邮箱');
  }
  const targetUser = await findAuthUserByEmail(email);
  const targetUserId = targetUser?.id;
  if (targetUserId) {
    await supabaseRequest('PATCH', 'internal_accounts', { supabase_owner_user_id: targetUserId }, {
      internal_user_id: `eq.${context.internalUserId}`,
    });
    await supabaseRequest('POST', 'user_identities', {
      internal_user_id: context.internalUserId,
      provider: 'email',
      provider_subject: email,
      normalized_email: email,
      supabase_user_id: targetUserId,
      is_verified: true,
      metadata_json: { source: 'miniprogram_test_bind' },
    }, { on_conflict: 'provider,provider_subject' });
  }
  const decision = await supabaseRequest<Array<{ merge_decision_id: string }>>(
    'POST',
    'account_merge_decisions',
    {
      source_internal_user_id: context.internalUserId,
      target_internal_user_id: context.internalUserId,
      state: targetUserId ? 'pending' : 'cancelled',
      preview_json: {
        email,
        targetSupabaseUserId: targetUserId,
      mode: targetUserId ? 'linked_existing_flutter_user_for_test' : 'email_not_found',
      },
    },
  );
  return json(200, {
    email,
    targetSupabaseUserId: targetUserId,
    mergeDecisionId: decision[0]?.merge_decision_id,
    requiresConfirmation: Boolean(targetUserId),
    message: targetUserId
      ? '已找到 Flutter 邮箱账号，并在测试模式下建立映射。请只使用专用测试邮箱。'
      : '没有找到对应 Flutter 邮箱账号，未执行绑定。',
  });
}

function normalizeAnswer(value?: string | null) {
  return (value ?? '').trim().toLowerCase().replace(/\s+/g, ' ');
}

function modeQuestionTotal(plan: JsonRecord, mode: StudyMode) {
  const totals: Record<StudyMode, number> = {
    newWord: Number(plan.newWordsPerDay ?? 20),
    review: Number(plan.reviewWordsPerDay ?? 28),
    mixedTest: Number(plan.mixedTestPerDay ?? 34),
    wrongWordReinforcement: Number(plan.wrongWordTestPerDay ?? 26),
    rootAffix: Number(plan.rootAffixPerDay ?? 4),
  };
  return Math.max(0, Math.min(60, Math.round(totals[mode] ?? 20)));
}

function modeEntryCount(plan: JsonRecord, mode: StudyMode) {
  const questionTotal = modeQuestionTotal(plan, mode);
  if (questionTotal <= 0) return 0;
  if (mode === 'newWord') return Math.max(1, Math.ceil(questionTotal / 4));
  return questionTotal;
}

function entryPayloadFromRow(row: Record<string, unknown>): JsonRecord {
  return {
    sourceId: String(row.source_id ?? row.entry_id ?? ''),
    entry_id: row.entry_id,
    word: row.word,
    partOfSpeech: row.part_of_speech,
    frequency: row.frequency,
    phoneticUs: row.phonetic_us,
    phoneticUk: row.phonetic_uk,
    meanings: row.meanings_json,
    meaningDetails: row.meaning_details_json,
    exampleSentence: row.example_sentence,
    exampleTranslation: row.example_translation,
    wordbookId: row.wordbook_id,
    rankInBook: row.rank_in_book,
  };
}

async function activeWordbookIds(context: AccountContext) {
  const wordbooks = await getWordbooks(context);
  return wordbooks.filter((wordbook) => wordbook.isActive).map((wordbook) => Number(wordbook.id));
}

function stableHash(value: string) {
  let hash = 2166136261;
  for (let index = 0; index < value.length; index += 1) {
    hash ^= value.charCodeAt(index);
    hash = Math.imul(hash, 16777619);
  }
  return hash >>> 0;
}

function dailyStudySeed() {
  return new Date().toISOString().slice(0, 10);
}

function stableDailySortRows(rows: Array<Record<string, unknown>>, seed: string) {
  return [...rows].sort((left, right) => {
    const leftId = String(left.source_id ?? left.entry_id ?? left.word ?? '');
    const rightId = String(right.source_id ?? right.entry_id ?? right.word ?? '');
    return stableHash(`${seed}:${leftId}`) - stableHash(`${seed}:${rightId}`) || leftId.localeCompare(rightId);
  });
}

function uniqueEntryRows(rows: Array<Record<string, unknown>>) {
  const seen = new Set<number>();
  const unique: Array<Record<string, unknown>> = [];
  for (const row of rows) {
    const entryId = Number(row.entry_id);
    if (!Number.isFinite(entryId) || seen.has(entryId)) continue;
    seen.add(entryId);
    unique.push(row);
  }
  return unique;
}

function orderRowsByIds(rows: Array<Record<string, unknown>>, ids: number[], idField: 'entry_id' | 'rank_in_book') {
  const order = new Map(ids.map((id, index) => [id, index]));
  return [...rows].sort((left, right) => {
    const leftOrder = order.get(Number(left[idField])) ?? Number.MAX_SAFE_INTEGER;
    const rightOrder = order.get(Number(right[idField])) ?? Number.MAX_SAFE_INTEGER;
    return leftOrder - rightOrder;
  });
}

async function loadRootAffixPayloads(selectedWordbookIds: number[], count: number) {
  if (count <= 0) return [];
  const rows = await supabaseRequest<Array<Record<string, unknown>>>(
    'GET',
    'study_entry_payloads',
    undefined,
    {
      wordbook_id: 'is.null',
      is_active: 'eq.true',
      select: '*',
      limit: '500',
    },
  );
  const includeMedical = selectedWordbookIds.some((id) => id === 4 || id === 3264070878);
  const includeShared =
    selectedWordbookIds.length === 0 ||
    selectedWordbookIds.some((id) => id === 1 || id === 2 || id === 3 || id !== 3264070878);
  const scoped = rows.filter((row) => {
    const sourceId = String(row.source_id ?? '');
    if (sourceId.includes('_medical_')) return includeMedical;
    if (sourceId.includes('_shared_')) return includeShared;
    return includeMedical || includeShared;
  });
  const pool = scoped.length ? scoped : rows;
  return stableDailySortRows(uniqueEntryRows(pool), dailyStudySeed()).slice(0, count).map(entryPayloadFromRow);
}

async function loadWordbookPayloadPool(wordbookId: number | undefined, limit: number, excludedEntryIds = new Set<number>()) {
  const query: Record<string, string> = {
    is_active: 'eq.true',
    select: '*',
    order: 'rank_in_book.asc',
    limit: String(Math.max(1, Math.min(1000, limit))),
  };
  if (wordbookId) query.wordbook_id = `eq.${wordbookId}`;
  const rows = await supabaseRequest<Array<Record<string, unknown>>>(
    'GET',
    'study_entry_payloads',
    undefined,
    query,
  );
  return rows.filter((row) => !excludedEntryIds.has(Number(row.entry_id)));
}

async function loadMixedTestPayloads(wordbookId: number | undefined, count: number) {
  if (count <= 0) return [];
  const seed = dailyStudySeed();
  const primaryPool = await loadWordbookPayloadPool(wordbookId, Math.max(count * 12, 120));
  const selected = stableDailySortRows(uniqueEntryRows(primaryPool), seed).slice(0, count);
  if (selected.length >= count || !wordbookId) return selected.map(entryPayloadFromRow);

  const excluded = new Set(selected.map((row) => Number(row.entry_id)));
  const fallbackPool = await loadWordbookPayloadPool(undefined, Math.max((count - selected.length) * 12, 120), excluded);
  const filled = selected.concat(stableDailySortRows(uniqueEntryRows(fallbackPool), seed).slice(0, count - selected.length));
  return filled.map(entryPayloadFromRow);
}

async function loadWrongWordPayloads(context: AccountContext, wordbookId: number | undefined, count: number) {
  if (count <= 0 || !context.supabaseUserId) return [];
  const wrongRows = await supabaseRequest<Array<{ entry_id: number }>>(
    'GET',
    'wrong_word_entries',
    undefined,
    {
      user_id: `eq.${context.supabaseUserId}`,
      select: 'entry_id',
      order: 'priority_score.desc,last_wrong_at.desc',
      limit: String(Math.max(count * 3, count)),
    },
  );
  const ids = wrongRows.map((row) => Number(row.entry_id)).filter((id) => Number.isFinite(id));
  if (!ids.length) return [];

  const exactRows = await supabaseRequest<Array<Record<string, unknown>>>(
    'GET',
    'study_entry_payloads',
    undefined,
    {
      entry_id: `in.(${ids.join(',')})`,
      is_active: 'eq.true',
      select: '*',
      limit: String(ids.length),
    },
  );

  const byEntryId = orderRowsByIds(exactRows, ids, 'entry_id');
  const selected = uniqueEntryRows(byEntryId);
  if (selected.length < count) {
    const rankQuery: Record<string, string> = {
      rank_in_book: `in.(${ids.join(',')})`,
      is_active: 'eq.true',
      select: '*',
      limit: String(ids.length),
    };
    if (wordbookId) rankQuery.wordbook_id = `eq.${wordbookId}`;
    const rankRows = await supabaseRequest<Array<Record<string, unknown>>>(
      'GET',
      'study_entry_payloads',
      undefined,
      rankQuery,
    );
    selected.push(...orderRowsByIds(rankRows, ids, 'rank_in_book'));
  }

  const unique = uniqueEntryRows(selected).slice(0, count);
  if (unique.length < count) {
    const excluded = new Set(unique.map((row) => Number(row.entry_id)));
    const fallbackPool = await loadWordbookPayloadPool(wordbookId, Math.max((count - unique.length) * 12, 120), excluded);
    unique.push(...stableDailySortRows(uniqueEntryRows(fallbackPool), dailyStudySeed()).slice(0, count - unique.length));
  }

  return unique.slice(0, count).map(entryPayloadFromRow);
}

async function loadStudyEntryPayloads(
  context: AccountContext,
  mode: StudyMode,
  count: number,
  requestedWordbookId?: number,
) {
  const selectedWordbookIds = requestedWordbookId
    ? [requestedWordbookId]
    : await activeWordbookIds(context);
  const wordbookId = selectedWordbookIds[0] ?? wordbookCatalog.find((wordbook) => wordbook.isActive)?.id;

  if (count <= 0) return [];

  if (mode === 'rootAffix') {
    return loadRootAffixPayloads(selectedWordbookIds, count);
  }

  if (mode === 'mixedTest') {
    return loadMixedTestPayloads(wordbookId, count);
  }

  if (mode === 'wrongWordReinforcement') {
    return loadWrongWordPayloads(context, wordbookId, count);
  }

  const query: Record<string, string> = {
    is_active: 'eq.true',
    select: '*',
    order: mode === 'newWord' ? 'rank_in_book.asc' : 'frequency.desc',
    limit: String(count),
  };
  if (wordbookId) query.wordbook_id = `eq.${wordbookId}`;
  const rows = await supabaseRequest<Array<Record<string, unknown>>>(
    'GET',
    'study_entry_payloads',
    undefined,
    query,
  );
  return rows.map(entryPayloadFromRow);
}

const fallbackStudySeeds = [
  ['herald', '预告，先兆；传令官', 'a bowl of daffodils, the first bright heralds of spring', '一盆水仙花，春天的第一个鲜明的预兆'],
  ['resilient', '能迅速恢复的，有韧性的', 'The team stayed resilient during the long release.', '团队在漫长发布期间保持韧性。'],
  ['meticulous', '一丝不苟的，极仔细的', 'A meticulous review caught the sync bug.', '一次细致的检查发现了同步问题。'],
  ['coherent', '连贯一致的，有条理的', 'The explanation became coherent after revision.', '修改之后解释变得连贯。'],
  ['fragile', '脆弱的，易碎的', 'The fragile cache failed after restart.', '脆弱的缓存重启后失效了。'],
  ['robust', '强健的，可靠的', 'A robust session survives interruption.', '可靠的会话能经受中断。'],
  ['precise', '精确的，准确的', 'A precise answer avoids guesswork.', '精确答案能避免猜测。'],
  ['retain', '保留，保持', 'The plan must retain question weights.', '计划必须保留题型权重。'],
];

const fallbackDistractors = [
  '姿势，体态；看法，态度',
  '正当理由；许可证；委任状',
  '翻译；译文，译本',
  '短暂的，临时的',
  '没有结构的，散乱的',
  '普通的，常见的',
];

function buildChoiceSet(answer: string, index: number) {
  const raw = [
    answer,
    fallbackDistractors[index % fallbackDistractors.length],
    fallbackDistractors[(index + 2) % fallbackDistractors.length],
    fallbackDistractors[(index + 4) % fallbackDistractors.length],
  ];
  const ordered = raw.map((_, choiceIndex) => raw[(choiceIndex - (index % 4) + 4) % 4]);
  const correctIndex = ordered.findIndex((item) => item === answer);
  return {
    correctChoiceLabel: ['A', 'B', 'C', 'D'][correctIndex],
    choices: ordered.map((value, choiceIndex) => ({
      label: ['A', 'B', 'C', 'D'][choiceIndex],
      value,
      text: value,
    })),
  };
}

function questionFromPayload(
  mode: StudyMode,
  payload: JsonRecord,
  index: number,
  total: number,
  questionType?: string,
): StudyQuestion {
  const word = String(payload.word ?? payload.lemma ?? payload.form ?? `word-${index + 1}`);
  const meanings = normalizeMeanings(
    payload.meanings ??
      payload.acceptedMeanings ??
      payload.accepted_meanings ??
      payload.meaningCn ??
      payload.meaning_cn ??
      payload.meaning ??
      payload.definitionCn ??
      payload.definition,
  );
  const answer = meanings[0] ?? '待补充释义';
  const isRootAffix = mode === 'rootAffix';
  const resolvedQuestionType = isRootAffix ? 'rootToGlossInput' : questionType ?? 'exampleToCnChoice';
  const choiceSet = isRootAffix || resolvedQuestionType.endsWith('Input') ? undefined : buildChoiceSet(answer, index);
  return {
    questionId: `${mode}-${String(payload.sourceId ?? payload.entry_id ?? word)}-${index + 1}`,
    questionType: resolvedQuestionType,
    entrySourceId: String(payload.sourceId ?? payload.entry_id ?? `entry-${word}`),
    entryId: Number(payload.entry_id),
    word,
    prompt: isRootAffix ? '根据词根/词缀填写含义' : '根据例句选择中文释义',
    partOfSpeech: String(payload.partOfSpeech ?? payload.part_of_speech ?? 'n'),
    phoneticUk: String(payload.phoneticUk ?? payload.phonetic_uk ?? `'${word}`),
    acceptedMeanings: [answer],
    choices: choiceSet?.choices,
    correctChoiceLabel: choiceSet?.correctChoiceLabel,
    questionIndex: index + 1,
    totalQuestions: total,
    exampleSentence: String(payload.exampleSentence ?? payload.example_sentence ?? payload.exampleWords ?? ''),
    exampleTranslation: String(payload.exampleTranslation ?? payload.example_translation ?? payload.exampleGlosses ?? ''),
    hasHint: true,
    userHint: `${word}: ${answer}`,
  };
}

function fallbackQuestionPayloads(mode: StudyMode, total: number): JsonRecord[] {
  if (mode === 'rootAffix') {
    return [
      { sourceId: 'root-hyper', form: 'hyper-', meaningCn: '高、过度、超过', exampleWords: 'hypertension, hyperactive', exampleGlosses: '高血压，过度活跃' },
      { sourceId: 'root-hypo', form: 'hypo-', meaningCn: '低、不足、在下', exampleWords: 'hypoxia, hypoglycemia', exampleGlosses: '缺氧，低血糖' },
      { sourceId: 'root-broncho', form: 'broncho-', meaningCn: '支气管', exampleWords: 'bronchitis, bronchoscopy', exampleGlosses: '支气管炎，支气管镜检查' },
      { sourceId: 'root-cardio', form: 'cardio-', meaningCn: '心脏', exampleWords: 'cardiology, cardiovascular', exampleGlosses: '心脏病学，心血管的' },
    ].slice(0, total);
  }
  return Array.from({ length: total }, (_, index) => {
    const [word, meaning, exampleSentence, exampleTranslation] =
      fallbackStudySeeds[index % fallbackStudySeeds.length];
    return {
      sourceId: `entry-${word}`,
      word,
      meanings: [meaning],
      exampleSentence,
      exampleTranslation,
    };
  });
}

function questionsFromPayloads(mode: StudyMode, payloads: JsonRecord[], total: number) {
  if (mode !== 'newWord') {
    return payloads
      .slice(0, total)
      .map((payload, index) => questionFromPayload(mode, payload, index, Math.min(payloads.length, total)));
  }

  const rounds = ['exampleToCnChoice', 'enToCnChoice', 'cnToEnChoice', 'enToCnInput'];
  const questions: StudyQuestion[] = [];
  for (const questionType of rounds) {
    for (const payload of payloads) {
      if (questions.length >= total) break;
      questions.push(questionFromPayload(mode, payload, questions.length, total, questionType));
    }
  }
  return questions;
}

async function startStudySession(context: AccountContext, body: JsonRecord) {
  const mode = (String(body.mode ?? 'review') as StudyMode);
  const plan = await getActivePlan(context) as JsonRecord;
  const total = modeQuestionTotal(plan, mode);
  const entryCount = mode === 'newWord' ? modeEntryCount(plan, mode) : total;
  if (total <= 0 || entryCount <= 0) {
    return error(409, 'STUDY_MODE_DISABLED', '该模式今日计划为 0，已在今日任务中禁用。');
  }
  const requestPayloads = Array.isArray(body.entryPayloads)
    ? body.entryPayloads.filter((item): item is JsonRecord => Boolean(item && typeof item === 'object'))
    : [];
  const requestedWordbookId = Number(body.wordbookId);
  const loadedPayloads = requestPayloads.length
    ? requestPayloads
    : await loadStudyEntryPayloads(
        context,
        mode,
        entryCount,
        Number.isFinite(requestedWordbookId) ? requestedWordbookId : undefined,
      );
  if (!loadedPayloads.length && context.supabaseUserId) {
    return error(409, 'STUDY_WORD_PAYLOADS_EMPTY', '当前词书没有可用题库数据，请重新选择词书或同步词库。');
  }
  const payloads = (loadedPayloads.length ? loadedPayloads : fallbackQuestionPayloads(mode, entryCount)).slice(0, entryCount);
  const questions = questionsFromPayloads(mode, payloads, total);
  const sessionId = `wmp-study-${crypto.randomUUID()}`;
  const startedAt = new Date().toISOString();
  const activeSession: ActiveStudySession = {
    sessionId,
    ownerId: context.internalUserId,
    mode,
    startedAt,
    questions,
    answeredQuestions: [],
  };
  activeStudySessions.set(sessionId, activeSession);
  return json(200, {
    session: {
      sessionId,
      mode,
      totalWords: questions.length,
      startedAt,
    },
    currentQuestion: questions[0],
    progress: { current: 1, total: questions.length },
    answeredQuestions: [],
    questions,
  });
}

function correctAnswerForQuestion(question: StudyQuestion) {
  if (question.choices?.length && question.correctChoiceLabel) {
    return question.choices.find(
      (choice) => normalizeAnswer(choice.label) === normalizeAnswer(question.correctChoiceLabel),
    )?.text ?? question.acceptedMeanings[0] ?? '';
  }
  return question.acceptedMeanings[0] ?? '';
}

async function persistStudyAnswer(
  context: AccountContext,
  session: ActiveStudySession,
  question: StudyQuestion,
  result: StudyResult,
) {
  if (!context.supabaseUserId) return;
  const now = result.answeredAt;
  const entryId = Number(question.entryId);
  const deviceId = '00000000-0000-4000-8000-000000000001';

  await supabaseRequest('POST', 'devices', {
    device_id: deviceId,
    user_id: context.supabaseUserId,
    platform: 'wechat_mp',
    device_label: 'WeChat Mini Program',
    app_version: '0.1.0',
    last_seen_at: now,
  }, { on_conflict: 'device_id' });

  await supabaseRequest('POST', 'study_events', {
    user_id: context.supabaseUserId,
    device_id: deviceId,
    session_id: session.sessionId,
    event_type: 'answer_submitted',
    payload_json: { question, result, mode: session.mode },
    occurred_at: now,
    idempotency_key: `wechat:${context.internalUserId}:${result.questionId}:${now}`,
  });

  if (Number.isFinite(entryId) && entryId > 0) {
    await supabaseRequest('POST', 'study_word_points', {
      user_id: context.supabaseUserId,
      point_date: now.slice(0, 10),
      entry_id: entryId,
      mode: session.mode,
      question_type: question.questionType,
      attempt_count: 1,
      correct_count: result.outcome === 'correct' ? 1 : 0,
      wrong_count: result.outcome === 'correct' ? 0 : 1,
      total_response_time_ms: result.responseTimeMs,
      last_answered_at: now,
    }, { on_conflict: 'user_id,point_date,entry_id,mode,question_type' });
  }
}

function summarizeActiveSession(session: ActiveStudySession, completedAt: string) {
  const correctCount = session.answeredQuestions.filter(
    (item) => item.result.outcome === 'correct' || item.result.outcome === 'fuzzyCorrect',
  ).length;
  const skippedCount = session.answeredQuestions.filter((item) => item.result.outcome === 'skipped').length;
  const incorrectCount = session.answeredQuestions.length - correctCount - skippedCount;
  return {
    sessionId: session.sessionId,
    totalQuestions: session.answeredQuestions.length,
    correctCount,
    incorrectCount,
    skippedCount,
    totalWords: session.questions.length,
    wrongWordCount: incorrectCount + skippedCount,
    accuracyPercent: session.answeredQuestions.length
      ? Math.round((correctCount / session.answeredQuestions.length) * 100)
      : 0,
    totalTimeMs: session.answeredQuestions.reduce((sum, item) => sum + item.result.responseTimeMs, 0),
    completedAt,
  };
}

async function submitStudyAnswer(context: AccountContext, body: JsonRecord) {
  const questionId = String(body.questionId ?? '');
  const snapshot = body.sessionSnapshot as JsonRecord | undefined;
  const snapshotSession = snapshot?.session as JsonRecord | undefined;
  const snapshotQuestions = Array.isArray(snapshot?.questions)
    ? snapshot.questions as StudyQuestion[]
    : [];
  const snapshotAnswered = Array.isArray(snapshot?.answeredQuestions)
    ? snapshot.answeredQuestions as Array<{ question: StudyQuestion; result: StudyResult }>
    : [];
  const sessionFromSnapshot: ActiveStudySession | undefined =
    snapshotSession && snapshotQuestions.length
      ? {
          sessionId: String(snapshotSession.sessionId ?? ''),
          ownerId: context.internalUserId,
          mode: String(snapshotSession.mode ?? 'review') as StudyMode,
          startedAt: String(snapshotSession.startedAt ?? new Date().toISOString()),
          questions: snapshotQuestions,
          answeredQuestions: snapshotAnswered,
        }
      : undefined;
  const session = sessionFromSnapshot ?? [...activeStudySessions.values()].find(
    (candidate) =>
      candidate.ownerId === context.internalUserId &&
      candidate.questions[candidate.answeredQuestions.length]?.questionId === questionId,
  );
  if (!session) return error(409, 'STUDY_SESSION_NOT_FOUND', '没有找到可继续的学习会话，请重新进入学习。');

  const currentIndex = Math.max(0, session.answeredQuestions.length);
  const question =
    session.questions[currentIndex] ??
    session.questions.find((candidate) => candidate.questionId === questionId);
  if (!question) return error(409, 'STUDY_QUESTION_NOT_FOUND', '没有找到当前题目，请重新进入学习。');
  const response = String(body.response ?? '').trim();
  const correctAnswer = correctAnswerForQuestion(question);
  const responseToken = normalizeAnswer(response);
  const correctTokens = new Set([
    normalizeAnswer(question.correctChoiceLabel),
    normalizeAnswer(correctAnswer),
    ...question.acceptedMeanings.map((meaning) => normalizeAnswer(meaning)),
  ].filter(Boolean));
  const skipped = responseToken.length === 0;
  const isCorrect = !skipped && correctTokens.has(responseToken);
  const answeredAt = new Date().toISOString();
  const result: StudyResult = {
    questionId: question.questionId,
    entrySourceId: question.entrySourceId,
    questionType: question.questionType,
    userResponse: response,
    normalizedResponse: response,
    correctAnswer,
    outcome: skipped ? 'skipped' : isCorrect ? 'correct' : 'incorrect',
    responseTimeMs: Number(body.responseTimeMs ?? 0),
    answeredAt,
  };
  session.answeredQuestions.push({ question, result });
  await persistStudyAnswer(context, session, question, result);

  const nextQuestion = session.questions[session.answeredQuestions.length];
  const isComplete = !nextQuestion;
  const summary = isComplete ? summarizeActiveSession(session, answeredAt) : undefined;
  if (isComplete) activeStudySessions.delete(session.sessionId);
  else activeStudySessions.set(session.sessionId, session);
  return json(200, {
    result,
    currentQuestion: nextQuestion,
    progress: {
      current: nextQuestion?.questionIndex ?? session.questions.length,
      total: session.questions.length,
    },
    answeredQuestions: session.answeredQuestions,
    isComplete,
    summary,
    nextAction: isComplete ? '返回今日' : '继续',
  });
}

function completeStudySession(context: AccountContext, sessionId: string) {
  const session = activeStudySessions.get(sessionId);
  if (!session || session.ownerId !== context.internalUserId) {
    return error(404, 'STUDY_SESSION_NOT_FOUND', '没有找到学习会话');
  }
  const completedAt = new Date().toISOString();
  const summary = summarizeActiveSession(session, completedAt);
  activeStudySessions.delete(sessionId);
  return json(200, { summary, nextAction: '返回今日' });
}

function cancelStudySession(context: AccountContext, sessionId: string) {
  const session = activeStudySessions.get(sessionId);
  if (session?.ownerId === context.internalUserId) activeStudySessions.delete(sessionId);
  return json(200, { ok: true });
}

async function recordAnswer(context: AccountContext, body: JsonRecord) {
  if (!context.supabaseUserId) {
    return error(409, 'EMAIL_BIND_REQUIRED', '微信账号需要先绑定邮箱账号后才能同步 Flutter 云学习数据');
  }
  const now = new Date().toISOString();
  const questionId = String(body.questionId ?? crypto.randomUUID());
  const response = String(body.response ?? '');
  const isCorrect = Boolean(body.isCorrect ?? body.outcome === 'correct');
  const entryId = Number(body.entryId ?? 0);
  const mode = String(body.mode ?? 'newWord');
  const questionType = String(body.questionType ?? 'enToCnChoice');
  const deviceId = '00000000-0000-4000-8000-000000000001';

  await supabaseRequest('POST', 'devices', {
    device_id: deviceId,
    user_id: context.supabaseUserId,
    platform: 'wechat_mp',
    device_label: 'WeChat Mini Program',
    app_version: '0.1.0',
    last_seen_at: now,
  }, { on_conflict: 'device_id' });

  await supabaseRequest('POST', 'study_events', {
    user_id: context.supabaseUserId,
    device_id: deviceId,
    session_id: String(body.sessionId ?? 'wechat-miniprogram'),
    event_type: 'answer_submitted',
    payload_json: body,
    occurred_at: now,
    idempotency_key: `wechat:${context.internalUserId}:${questionId}:${now}`,
  });

  if (entryId > 0) {
    await supabaseRequest('POST', 'study_word_points', {
      user_id: context.supabaseUserId,
      point_date: now.slice(0, 10),
      entry_id: entryId,
      mode,
      question_type: questionType,
      attempt_count: 1,
      correct_count: isCorrect ? 1 : 0,
      wrong_count: isCorrect ? 0 : 1,
      total_response_time_ms: Number(body.responseTimeMs ?? 0),
      last_answered_at: now,
    }, { on_conflict: 'user_id,point_date,entry_id,mode,question_type' });
  }

  return json(200, {
    result: {
      questionId,
      entrySourceId: String(body.entrySourceId ?? ''),
      questionType,
      userResponse: response,
      correctAnswer: String(body.correctAnswer ?? ''),
      outcome: isCorrect ? 'correct' : 'incorrect',
      responseTimeMs: Number(body.responseTimeMs ?? 0),
      answeredAt: now,
    },
    progress: { current: Number(body.current ?? 1), total: Number(body.total ?? 1) },
    answeredQuestions: [],
    isComplete: Boolean(body.isComplete),
  });
}

Deno.serve(async (request) => {
  if (request.method === 'OPTIONS') return new Response(null, { headers: corsHeaders });

  try {
    const path = normalizePath(request.url);
    const method = request.method;

    if (method === 'POST' && path === '/v1/auth/wechat-mp/login') {
      const body = await readJson(request);
      const code = typeof body.code === 'string' ? body.code.trim() : '';
      if (code.length < 3) return error(400, 'VALIDATION_FAILED', 'Valid WeChat login code is required');

      const identity = await exchangeWechatCode(code);
      const context = await upsertWechatAccount(identity);
      const bindings = await getBindingFlags(context);
      const sessionId = `wmp_${crypto.randomUUID()}`;
      return json(200, {
        ...accountState(context, bindings),
        session: {
          accessToken: createAccessToken(context, sessionId),
          refreshToken: createRefreshToken(context),
          expiresAt: new Date(Date.now() + 15 * 60 * 1000).toISOString(),
        },
      });
    }

    if (method === 'POST' && path === '/v1/auth/refresh') {
      const body = await readJson(request);
      const refreshToken = typeof body.refreshToken === 'string' ? body.refreshToken : '';
      let context: AccountContext | undefined;
      if (refreshToken.startsWith('refresh_')) {
        const payload = JSON.parse(base64UrlDecode(refreshToken.slice('refresh_'.length))) as AccountContext;
        context = await getAccountContext(payload.internalUserId ?? (payload as JsonRecord).sub as string);
      }
      if (!context) return error(401, 'SESSION_EXPIRED', 'Refresh session is invalid or expired');
      const sessionId = `wmp_${crypto.randomUUID()}`;
      return json(200, {
        ...accountState(context, await getBindingFlags(context)),
        session: {
          accessToken: createAccessToken(context, sessionId),
          refreshToken,
          expiresAt: new Date(Date.now() + 15 * 60 * 1000).toISOString(),
        },
      });
    }

    const auth = authContext(request);
    if (!auth) return error(401, 'AUTH_REQUIRED', 'Authentication is required');
    const context = await getAccountContext(auth.internalUserId);

    if (method === 'GET' && path === '/v1/me') return json(200, accountState(context, await getBindingFlags(context)));
    if (method === 'GET' && path === '/v1/today') return json(200, await getToday(context));
    if (method === 'GET' && path === '/v1/plan/active') return json(200, await getActivePlan(context));
    if (method === 'PATCH' && path.match(/^\/v1\/plan\/[^/]+$/)) return savePlan(context, await readJson(request));
    if (method === 'POST' && path === '/v1/plan/apply-to-today') return applyPlanToToday(context);
    if (method === 'GET' && path === '/v1/wordbooks') return json(200, await getWordbooks(context));
    if (method === 'POST' && path.match(/^\/v1\/wordbooks\/[^/]+\/toggle$/)) {
      const body = await readJson(request);
      return setActiveWordbook(context, Number(path.split('/')[3]), Boolean(body.isActive));
    }
    if (method === 'GET' && path === '/v1/reports/overview') return json(200, await getReports(context));
    if (method === 'GET' && path === '/v1/study/resume-hint') return json(200, { hasResume: false });
    if (method === 'POST' && path === '/v1/study/sessions') return startStudySession(context, await readJson(request));
    if (method === 'POST' && path.match(/^\/v1\/study\/sessions\/[^/]+\/complete$/)) {
      return completeStudySession(context, decodeURIComponent(path.split('/')[4] ?? ''));
    }
    if (method === 'POST' && path.match(/^\/v1\/study\/sessions\/[^/]+\/cancel$/)) {
      return cancelStudySession(context, decodeURIComponent(path.split('/')[4] ?? ''));
    }
    if (method === 'GET' && path === '/v1/rewards/today') return json(200, await getTodayRewardState(context));
    if (method === 'POST' && path === '/v1/rewards/today/claim') return claimTodayReward(context);
    if (method === 'GET' && path === '/v1/wrong-words') return json(200, await getWrongWords(context));
    if (method === 'POST' && path === '/v1/account/email-bind/start') {
      return startEmailBind(context, await readJson(request));
    }
    if (method === 'POST' && path === '/v1/study/answers') return submitStudyAnswer(context, await readJson(request));
    if (method === 'GET' && path.startsWith('/v1/leaderboard/')) {
      return json(200, {
        metric: path.split('/').pop() ?? 'weekly',
        periodLabel: '排行榜',
        generatedAt: new Date().toISOString(),
        currentUser: undefined,
        entries: [],
      });
    }
    if (method === 'POST' && path === '/v1/auth/logout') return json(200, { ok: true });

    return error(404, 'ROUTE_NOT_FOUND', `Route not found: ${method} ${path}`);
  } catch (caught) {
    const message = caught instanceof Error ? caught.message : 'Request failed';
    return error(500, 'REQUEST_FAILED', message);
  }
});
