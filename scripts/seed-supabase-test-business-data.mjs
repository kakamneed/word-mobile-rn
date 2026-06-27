import { readFileSync } from 'node:fs';

const email = (process.argv[2] ?? '').trim().toLowerCase();
if (!email) {
  throw new Error('Usage: node scripts/seed-supabase-test-business-data.mjs <email>');
}

const env = readEnv('D:/projects/word-mobile-rn/.env.supabase.local');
const supabaseUrl = mustEnv(env, 'SUPABASE_URL').replace(/\/$/, '');
const serviceRoleKey = mustEnv(env, 'SUPABASE_SERVICE_ROLE_KEY');

const headers = {
  apikey: serviceRoleKey,
  authorization: `Bearer ${serviceRoleKey}`,
  'Content-Type': 'application/json',
};

const authUser = await findAuthUserByEmail(email);
if (!authUser) {
  throw new Error(`No Supabase auth user found for ${email}`);
}

const userId = authUser.id;
await seedData(userId, email);
const summary = await countRows(userId);

console.log(
  JSON.stringify(
    {
      email,
      userId,
      summary,
    },
    null,
    2,
  ),
);

async function seedData(userId, email) {
  const nowVersion = Date.now();
  await upsert('profiles', [
    {
      user_id: userId,
      display_name: '小程序同步测试账号',
      locale: 'zh-CN',
    },
  ], 'user_id');

  await upsert('plan_configs', [
    {
      user_id: userId,
      name: '小程序同步测试计划',
      new_words_per_day: 18,
      review_words_per_day: 24,
      mixed_test_per_day: 12,
      wrong_word_test_per_day: 10,
      root_affix_per_day: 4,
      growth_rule_mode: 'byMode',
      shared_growth_rule: { intervalDays: 7, increment: 4 },
      growth_rules_by_mode: {
        newWord: { intervalDays: 5, increment: 2 },
        review: { intervalDays: 7, increment: 4 },
        mixedTest: { intervalDays: 10, increment: 2 },
        wrongWordReinforcement: { intervalDays: 7, increment: 2 },
      },
      version: nowVersion,
    },
  ], 'user_id');

  await upsert('wordbook_preferences', [
    { user_id: userId, wordbook_id: 1, is_active: true },
    { user_id: userId, wordbook_id: 2, is_active: true },
    { user_id: userId, wordbook_id: 3, is_active: false },
    { user_id: userId, wordbook_id: 4, is_active: true },
  ], 'user_id,wordbook_id');

  await upsert('croc_bti_profiles', [
    {
      user_id: userId,
      result_code: 'CIRA',
      title: '鳄隐士',
      summary: '偏好用语境慢慢拆词，适合稳定复习、少量新词和错词回收并行推进。',
      advice: '先保持每天固定学习，再用错词和 AI 短文把薄弱词放回具体语境里。',
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
        newWords: 22,
        review: 34,
        mixedTest: 16,
        wrongWordReview: 16,
        contextExamples: 12,
      },
      plan_input_json: {
        newWordsPerDay: 18,
        reviewWordsPerDay: 24,
        mixedTestPerDay: 12,
        wrongWordTestPerDay: 10,
      },
      question_type_weights_json: {
        review: { enToCnChoice: 25, exampleToCnChoice: 35, enToCnInput: 40 },
        mixedTest: { enToCnChoice: 20, exampleToCnChoiceNoTranslation: 45, wordSkeletonInput: 35 },
        wrongWordReinforcement: { enToCnChoice: 30, enToCnInput: 40, exampleToCnChoice: 30 },
      },
      daily_learning_minutes: 35,
      source: 'test_seed_expanded',
      version: nowVersion,
      evaluated_at: '2026-05-28T10:00:00Z',
    },
  ], 'user_id');

  await upsert('study_word_points', buildStudyPoints(userId), 'user_id,point_date,entry_id,mode,question_type');
  await upsert('wrong_word_entries', buildWrongWords(userId), 'user_id,entry_id');
  await upsert('report_snapshots', buildReports(userId), 'user_id,snapshot_date');
  await upsert('leaderboard_stats', [
    {
      user_id: userId,
      period: 'weekly',
      period_start: '2026-05-25',
      display_name: '小程序同步测试账号',
      total_questions: 318,
      correct_count: 247,
      learned_words: 54,
      mixed_test_total_questions: 64,
      mixed_test_correct_count: 47,
      current_streak_days: 7,
      last_summary_key: `expanded-test-${email}`,
      last_summary_at: '2026-05-28T10:30:00Z',
    },
  ], 'user_id,period,period_start');

  await upsert('ai_passages', buildAiPassages(userId), 'passage_id');
}

function buildStudyPoints(userId) {
  const entryIds = [
    1, 2, 3, 4, 983, 1481, 3092, 3105, 4101, 5000,
    5538, 5770, 6189, 6415, 6606, 6613, 7181, 7440,
    7767, 7879, 8092, 11549, 11550, 11551,
  ];
  const modes = ['newWord', 'review', 'mixedTest', 'wrongWordReinforcement', 'rootAffix'];
  const questionTypes = ['enToCnChoice', 'cnToEnChoice', 'enToCnInput', 'exampleToCnChoice', 'wordSkeletonInput'];
  const rows = [];
  for (let dayOffset = 0; dayOffset < 7; dayOffset += 1) {
    const date = `2026-05-${String(22 + dayOffset).padStart(2, '0')}`;
    for (let i = 0; i < 8; i += 1) {
      const entryId = entryIds[(dayOffset * 8 + i) % entryIds.length];
      const mode = modes[(dayOffset + i) % modes.length];
      const questionType = questionTypes[(dayOffset * 2 + i) % questionTypes.length];
      const attempts = 1 + ((dayOffset + i) % 4);
      const wrong = (dayOffset + i) % 3 === 0 ? 1 : 0;
      rows.push(point(
        userId,
        date,
        entryId,
        mode,
        questionType,
        attempts,
        Math.max(0, attempts - wrong),
        wrong,
        2500 + dayOffset * 700 + i * 350,
      ));
    }
  }
  return rows;
}

function buildWrongWords(userId) {
  const words = [
    [1, 3, 8.5, 'cancel 表示取消或撤销，注意不是 delay。'],
    [2, 4, 9.1, 'explosive 可以表示爆炸性的，也可以表示极易引起争论的。'],
    [3, 2, 7.0, 'numerous 强调数量多。'],
    [4, 5, 9.6, 'govern 可表示支配、占优势，不只表示治理。'],
    [983, 2, 6.8, 'fluent 强调表达流利自然。'],
    [1481, 3, 8.2, 'defect 作动词时可表示变节或叛离。'],
    [3092, 2, 7.4, 'premium 常表示优质的或高价的。'],
    [3105, 3, 8.0, 'contradiction 表示矛盾、不一致。'],
    [4101, 4, 8.8, 'dissipate 表示逐渐消散。'],
    [5770, 1, 5.5, 'architect 是建筑师，也可引申为设计者。'],
    [6189, 2, 7.7, 'treason 表示叛国、通敌。'],
    [6415, 2, 7.1, 'layman 指外行或普通信徒。'],
    [6606, 1, 5.9, 'acquaint 表示使了解、使认识。'],
    [6613, 2, 6.9, 'innumerable 表示无数的。'],
    [7181, 1, 5.8, 'ivory 表示象牙或象牙色。'],
    [7440, 3, 8.3, 'illustrate 表示说明、阐明或加插图。'],
    [7767, 2, 7.3, 'civilise 表示使文明、教化。'],
    [7879, 1, 5.7, 'tender 可表示温柔，也可表示脆弱敏感。'],
    [8092, 1, 5.6, 'axe 是斧，也可作动词表示削减。'],
    [11549, 2, 7.0, 'tude 常和状态、性质相关。'],
  ];
  return words.map(([entryId, errorCount, score, hint], index) => ({
    user_id: userId,
    entry_id: entryId,
    error_count: errorCount,
    last_wrong_at: `2026-05-${String(23 + (index % 6)).padStart(2, '0')}T0${index % 9}:20:00Z`,
    priority_score: score,
    hint_text: hint,
    hint_source: 'expanded_test_seed',
    hint_updated_at: `2026-05-${String(23 + (index % 6)).padStart(2, '0')}T10:00:00Z`,
    projection_version: 2,
  }));
}

function buildReports(userId) {
  const data = [
    ['2026-05-22', 1, 21, 30, 70.0, 10],
    ['2026-05-23', 2, 30, 42, 71.43, 18],
    ['2026-05-24', 3, 37, 50, 74.0, 27],
    ['2026-05-25', 4, 45, 59, 76.27, 35],
    ['2026-05-26', 5, 52, 67, 77.61, 43],
    ['2026-05-27', 6, 61, 78, 78.21, 50],
    ['2026-05-28', 7, 72, 92, 78.26, 58],
  ];
  return data.map(([date, days, correct, total, accuracy, learned], index) =>
    report(userId, date, days, correct, total, accuracy, learned, index),
  );
}

function buildAiPassages(userId) {
  const passages = [
    {
      id: '11111111-1111-4111-8111-111111111111',
      date: '2026-05-24',
      title: '港口新闻里的词汇线索',
      words: [
        [2, 'explosive', '爆炸的；极易引起争论的'],
        [3, 'numerous', '众多的'],
        [4101, 'dissipate', '消散，消失'],
      ],
    },
    {
      id: '22222222-2222-4222-8222-222222222222',
      date: '2026-05-25',
      title: '一场计划调整后的复盘',
      words: [
        [1, 'cancel', '取消，撤销；删去'],
        [4, 'govern', '居支配地位，占优势'],
        [983, 'fluent', '流利的，流畅的'],
      ],
    },
    {
      id: '33333333-3333-4333-8333-333333333333',
      date: '2026-05-26',
      title: '旧温室里的地图',
      words: [
        [5770, 'architect', '建筑师'],
        [7440, 'illustrate', '举例说明，阐明；图解，加插图'],
        [6415, 'layman', '门外汉，外行'],
      ],
    },
    {
      id: '44444444-4444-4444-8444-444444444444',
      date: '2026-05-27',
      title: '市场里的高级课程',
      words: [
        [3092, 'premium', '高级的，优质的；售价高的'],
        [6613, 'innumerable', '无数的，数不清的'],
        [6606, 'acquaint', '使认识，使了解'],
      ],
    },
    {
      id: '55555555-5555-4555-8555-555555555555',
      date: '2026-05-28',
      title: '雾散之后的词根课',
      words: [
        [11549, 'tude', '状态、性质相关的词根线索'],
        [11550, 'turb', '扰动、混乱相关的词根线索'],
        [11551, 'util', '使用、效用相关的词根线索'],
      ],
    },
  ];

  return passages.map((passage) => ({
    passage_id: passage.id,
    user_id: userId,
    title: passage.title,
    payload_json: passagePayload(passage),
    validation_status: 'passed',
    generated_at: `${passage.date}T10:00:00Z`,
  }));
}

function passagePayload(passage) {
  const segments = [
    { type: 'text', text: '今天的短文把几个容易混淆的词放进同一个场景里。老师先提醒大家，词义不是孤立存在的，遇到 ' },
  ];
  for (const [entryId, word, glossZh] of passage.words) {
    segments.push({ type: 'word', text: word, entryId, glossZh, highlighted: true });
    segments.push({ type: 'text', text: ' 时，要同时观察前后的语气和事件背景。' });
  }
  segments.push({
    type: 'text',
    text: '这样复习时，错词不再只是列表里的红点，而会重新回到可以理解、可以使用的语境中。',
  });

  return {
    date: passage.date,
    title: passage.title,
    passageId: `expanded_${passage.id}`,
    targetLevel: 'intermediate',
    validationStatus: 'passed',
    wordCount: 260,
    preview: `${passage.title}：今天的短文把几个容易混淆的词放进同一个场景里。`,
    wrongWords: passage.words.map(([entryId, word, primaryGloss]) => ({
      entryId,
      word,
      partOfSpeech: entryId >= 11549 ? 'root' : 'n',
      primaryGloss,
    })),
    coveredWordIds: passage.words.map(([entryId]) => entryId),
    missingWordIds: [],
    generatedAt: `${passage.date}T10:00:00Z`,
    failureReason: null,
    blocks: [
      {
        blockType: 'paragraph',
        segments,
      },
    ],
  };
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

function report(userId, date, days, correct, total, accuracy, learned, index) {
  const dailySeries = [];
  for (let offset = 0; offset <= index; offset += 1) {
    const day = 22 + offset;
    const dayTotal = 30 + offset * 10;
    const dayCorrect = Math.round(dayTotal * (0.7 + offset * 0.012));
    dailySeries.push({
      date: `2026-05-${String(day).padStart(2, '0')}`,
      studyTimeMs: 160000 + offset * 35000,
      correctCount: dayCorrect,
      totalQuestions: dayTotal,
      accuracyPercent: Number(((dayCorrect / dayTotal) * 100).toFixed(2)),
    });
  }

  return {
    user_id: userId,
    snapshot_date: date,
    payload_json: {
      last7Days: dailySeries.slice(-7),
      modeSeries: {
        newWord: dailySeries.map((row) => ({ ...row, totalQuestions: Math.ceil(row.totalQuestions * 0.35) })),
        review: dailySeries.map((row) => ({ ...row, totalQuestions: Math.ceil(row.totalQuestions * 0.32) })),
        mixedTest: dailySeries.map((row) => ({ ...row, totalQuestions: Math.ceil(row.totalQuestions * 0.18) })),
        wrongWordReinforcement: dailySeries.map((row) => ({ ...row, totalQuestions: Math.ceil(row.totalQuestions * 0.15) })),
      },
      streakInfo: { currentStreak: days, longestStreak: days, lastStudyDate: date },
      dailySeries,
      modeBreakdown: [
        { mode: 'newWord', correctCount: Math.floor(correct * 0.36), totalQuestions: Math.floor(total * 0.36), accuracyPercent: accuracy },
        { mode: 'review', correctCount: Math.floor(correct * 0.32), totalQuestions: Math.floor(total * 0.32), accuracyPercent: accuracy },
        { mode: 'mixedTest', correctCount: Math.floor(correct * 0.18), totalQuestions: Math.floor(total * 0.18), accuracyPercent: accuracy },
        { mode: 'wrongWordReinforcement', correctCount: Math.floor(correct * 0.14), totalQuestions: Math.floor(total * 0.14), accuracyPercent: accuracy },
      ],
      totalStudyDays: days,
      overallAccuracy: accuracy,
      totalWordsLearned: learned,
      totalQuestionsAnswered: total,
    },
  };
}

async function findAuthUserByEmail(email) {
  for (let page = 1; page <= 20; page += 1) {
    const response = await fetch(`${supabaseUrl}/auth/v1/admin/users?page=${page}&per_page=100`, {
      headers,
    });
    if (!response.ok) {
      const body = await response.text();
      throw new Error(`List auth users failed: ${response.status} ${body.slice(0, 500)}`);
    }
    const body = await response.json();
    const users = Array.isArray(body.users) ? body.users : [];
    const found = users.find((user) => (user.email ?? '').toLowerCase() === email);
    if (found) return found;
    if (users.length < 100) break;
  }
  return null;
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
