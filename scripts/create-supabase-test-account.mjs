import { readFileSync } from 'node:fs';

const env = readEnv('D:/projects/word-mobile-rn/.env.supabase.local');
const supabaseUrl = mustEnv(env, 'SUPABASE_URL').replace(/\/$/, '');
const serviceRoleKey = mustEnv(env, 'SUPABASE_SERVICE_ROLE_KEY');

const suffix = new Date().toISOString().replace(/[-:.TZ]/g, '').slice(0, 12);
const email = `word-test-${suffix}@example.com`;
const password = `WordTest-${suffix}!`;

const headers = {
  apikey: serviceRoleKey,
  authorization: `Bearer ${serviceRoleKey}`,
  'Content-Type': 'application/json',
};

const user = await createAuthUser(email, password);
await seedBusinessData(user.id, email);
const summary = await countRows(user.id);

console.log(
  JSON.stringify(
    {
      email,
      password,
      userId: user.id,
      summary,
    },
    null,
    2,
  ),
);

async function createAuthUser(email, password) {
  const response = await fetch(`${supabaseUrl}/auth/v1/admin/users`, {
    method: 'POST',
    headers,
    body: JSON.stringify({
      email,
      password,
      email_confirm: true,
      user_metadata: {
        display_name: 'Supabase 测试账号',
        locale: 'zh-CN',
      },
    }),
  });
  if (!response.ok) {
    const body = await response.text();
    throw new Error(`Create auth user failed: ${response.status} ${body.slice(0, 500)}`);
  }
  return response.json();
}

async function seedBusinessData(userId, email) {
  await upsert('profiles', [
    {
      user_id: userId,
      display_name: 'Supabase 测试账号',
      locale: 'zh-CN',
    },
  ], 'user_id');

  await upsert('plan_configs', [
    {
      user_id: userId,
      name: '测试计划',
      new_words_per_day: 12,
      review_words_per_day: 16,
      mixed_test_per_day: 8,
      wrong_word_test_per_day: 6,
      root_affix_per_day: 2,
      growth_rule_mode: 'shared',
      shared_growth_rule: { intervalDays: 7, increment: 4 },
      growth_rules_by_mode: {},
      version: Date.now(),
    },
  ], 'user_id');

  await upsert('wordbook_preferences', [
    { user_id: userId, wordbook_id: 1, is_active: false },
    { user_id: userId, wordbook_id: 2, is_active: true },
    { user_id: userId, wordbook_id: 3, is_active: false },
    { user_id: userId, wordbook_id: 4, is_active: false },
  ], 'user_id,wordbook_id');

  await upsert('croc_bti_profiles', [
    {
      user_id: userId,
      result_code: 'CIRA',
      title: '鳄隐士',
      summary: '测试账号使用低压复习和语境理解的学习人格。',
      advice: '保持少量新词、稳定复习和例句理解。',
      answers_json: {
        'vc-context-guessing': 2,
        'io-recognize-enough': 2,
        'nr-slower-solid': 2,
        'at-examples': 2,
      },
      axis_scores_json: {
        vc: { score: -4, selectedTrait: 'C', strength: 'clear' },
        io: { score: -3, selectedTrait: 'I', strength: 'clear' },
        nr: { score: -4, selectedTrait: 'R', strength: 'clear' },
        at: { score: -2, selectedTrait: 'A', strength: 'soft' },
      },
      weights_json: {
        newWords: 20,
        review: 35,
        mixedTest: 15,
        wrongWordReview: 15,
        contextExamples: 15,
      },
      plan_input_json: {
        newWordsPerDay: 8,
        reviewWordsPerDay: 28,
        mixedTestPerDay: 12,
        wrongWordTestPerDay: 8,
      },
      question_type_weights_json: {
        review: { enToCnChoice: 25, exampleToCnChoice: 35, enToCnInput: 40 },
        mixedTest: { enToCnChoice: 20, exampleToCnChoiceNoTranslation: 45, wordSkeletonInput: 35 },
        wrongWordReinforcement: { enToCnChoice: 30, enToCnInput: 40, exampleToCnChoice: 30 },
      },
      daily_learning_minutes: 25,
      source: 'test_seed',
      version: Date.now(),
      evaluated_at: new Date().toISOString(),
    },
  ], 'user_id');

  await upsert('study_word_points', [
    point(userId, '2026-05-24', 1, 'newWord', 'enToCnChoice', 4, 3, 1, 18000),
    point(userId, '2026-05-24', 2, 'newWord', 'cnToEnChoice', 4, 2, 2, 22000),
    point(userId, '2026-05-25', 983, 'review', 'enToCnInput', 3, 2, 1, 16000),
    point(userId, '2026-05-25', 1481, 'wrongWordReinforcement', 'exampleToCnChoice', 2, 1, 1, 9000),
    point(userId, '2026-05-26', 3092, 'mixedTest', 'wordSkeletonInput', 3, 2, 1, 14000),
  ], 'user_id,point_date,entry_id,mode,question_type');

  await upsert('wrong_word_entries', [
    {
      user_id: userId,
      entry_id: 2,
      error_count: 3,
      last_wrong_at: '2026-05-24T08:10:00Z',
      priority_score: 8.4,
      hint_text: 'explosive 可指爆炸性的，也可指极易引起争论的。',
      hint_source: 'test_seed',
      hint_updated_at: '2026-05-24T08:12:00Z',
      projection_version: 1,
    },
    {
      user_id: userId,
      entry_id: 1481,
      error_count: 2,
      last_wrong_at: '2026-05-25T09:20:00Z',
      priority_score: 7.2,
      hint_text: 'defect 作动词时可表示变节、叛变。',
      hint_source: 'test_seed',
      hint_updated_at: '2026-05-25T09:21:00Z',
      projection_version: 1,
    },
  ], 'user_id,entry_id');

  await upsert('report_snapshots', [
    report(userId, '2026-05-25', 2, 18, 25, 72),
    report(userId, '2026-05-26', 3, 28, 36, 77.78),
  ], 'user_id,snapshot_date');

  await upsert('leaderboard_stats', [
    {
      user_id: userId,
      period: 'weekly',
      period_start: '2026-05-25',
      display_name: 'Supabase 测试账号',
      total_questions: 61,
      correct_count: 46,
      learned_words: 12,
      mixed_test_total_questions: 10,
      mixed_test_correct_count: 7,
      current_streak_days: 3,
      last_summary_key: `test-${email}`,
      last_summary_at: new Date().toISOString(),
    },
  ], 'user_id,period,period_start');

  await upsert('ai_passages', [
    {
      user_id: userId,
      title: '测试短文：港口里的词汇课',
      payload_json: {
        date: '2026-05-26',
        title: '测试短文：港口里的词汇课',
        passageId: `test_passage_${suffix}`,
        targetLevel: 'intermediate',
        validationStatus: 'passed',
        wordCount: 180,
        wrongWords: [
          { entryId: 2, word: 'explosive', partOfSpeech: 'adj', primaryGloss: '爆炸的；极易引起争论的' },
          { entryId: 1481, word: 'defect', partOfSpeech: 'v', primaryGloss: '变节，叛变' },
        ],
        blocks: [
          {
            blockType: 'paragraph',
            segments: [
              { type: 'text', text: '老师用港口新闻说明 ' },
              { type: 'word', text: 'explosive', entryId: 2, glossZh: '爆炸的；极易引起争论的', highlighted: true },
              { type: 'text', text: ' 也可以描述争议性很强的话题。' },
            ],
          },
        ],
      },
      validation_status: 'passed',
      generated_at: '2026-05-26T10:00:00Z',
    },
  ], 'passage_id');
}

function point(userId, date, entryId, mode, questionType, attempts, correct, wrong, timeMs) {
  return {
    user_id: userId,
    point_date: date,
    entry_id: entryId,
    mode,
    question_type: questionType,
    attempt_count: attempts,
    correct_count: correct,
    wrong_count: wrong,
    total_response_time_ms: timeMs,
    last_answered_at: `${date}T09:00:00Z`,
  };
}

function report(userId, date, days, correct, total, accuracy) {
  return {
    user_id: userId,
    snapshot_date: date,
    payload_json: {
      last7Days: [
        { date, studyTimeMs: 180000, correctCount: correct, totalQuestions: total, accuracyPercent: accuracy },
      ],
      modeSeries: {},
      streakInfo: { currentStreak: days, longestStreak: days, lastStudyDate: date },
      dailySeries: [
        { date, studyTimeMs: 180000, correctCount: correct, totalQuestions: total, accuracyPercent: accuracy },
      ],
      modeBreakdown: [
        { mode: 'newWord', correctCount: Math.floor(correct / 2), totalQuestions: Math.floor(total / 2), accuracyPercent: accuracy },
      ],
      totalStudyDays: days,
      overallAccuracy: accuracy,
      totalWordsLearned: 12,
      totalQuestionsAnswered: total,
    },
  };
}

async function upsert(table, rows, conflict) {
  if (!rows.length) return;
  const url = new URL(`${supabaseUrl}/rest/v1/${table}`);
  url.searchParams.set('on_conflict', conflict);
  const response = await fetch(url, {
    method: 'POST',
    headers: {
      ...headers,
      Prefer: 'resolution=merge-duplicates,return=minimal',
    },
    body: JSON.stringify(rows),
  });
  if (!response.ok) {
    const body = await response.text();
    throw new Error(`${table} upsert failed: ${response.status} ${body.slice(0, 500)}`);
  }
}

async function countRows(userId) {
  const tables = [
    'profiles',
    'plan_configs',
    'croc_bti_profiles',
    'wordbook_preferences',
    'study_word_points',
    'wrong_word_entries',
    'report_snapshots',
    'leaderboard_stats',
    'ai_passages',
  ];
  const result = {};
  for (const table of tables) {
    const response = await fetch(`${supabaseUrl}/rest/v1/${table}?select=*&user_id=eq.${userId}`, {
      method: 'HEAD',
      headers: { ...headers, Prefer: 'count=exact' },
    });
    result[table] = response.headers.get('content-range') ?? `status:${response.status}`;
  }
  return result;
}

function readEnv(path) {
  const result = {};
  const text = readFileSync(path, 'utf8');
  for (const line of text.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) continue;
    const index = trimmed.indexOf('=');
    if (index === -1) continue;
    result[trimmed.slice(0, index)] = trimmed.slice(index + 1);
  }
  return result;
}

function mustEnv(env, key) {
  const value = env[key];
  if (!value) throw new Error(`Missing ${key}`);
  return value;
}
