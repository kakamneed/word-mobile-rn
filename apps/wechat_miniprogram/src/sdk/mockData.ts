import {
  AuthSessionState,
  LeaderboardMetric,
  LeaderboardSummary,
  PlanSummary,
  ReportsOverview,
  StartSessionResponse,
  StudyMode,
  StudyQuestion,
  TodayHomeState,
  TodayRewardState,
  WordbookSummary,
  WrongWordDetail,
  WrongWordEntry,
} from './types';

export const modeLabels: Record<StudyMode, string> = {
  newWord: '新词学习',
  review: '复习',
  mixedTest: '混合测试',
  wrongWordReinforcement: '错词强化',
  rootAffix: '词根词缀',
};

export const mockAuthState: AuthSessionState = {
  user: {
    internalUserId: 'demo-user',
    primaryIdentity: 'wechat_mp',
    hasEmailBinding: false,
    hasWechatBinding: true,
  },
  accountState: {
    phase: 'signed_in_active',
    needsBindDecision: false,
    cloudDataState: 'empty_or_ready',
  },
};

export const mockPlan: PlanSummary = {
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
  questionTypeWeightsByMode: {
    review: {
      enToCnInput: 25,
      exampleToCnChoiceNoTranslation: 25,
      enToCnChoice: 20,
      cnToEnChoice: 15,
      wordSkeletonInput: 15,
    },
    mixedTest: {
      enToCnChoice: 25,
      exampleToCnChoice: 15,
      exampleToCnChoiceNoTranslation: 20,
      enToCnInput: 20,
      cnToEnChoice: 10,
      wordSkeletonInput: 10,
    },
    wrongWordReinforcement: {
      enToCnInput: 30,
      wordSkeletonInput: 25,
      exampleToCnChoiceNoTranslation: 20,
      cnToEnChoice: 15,
      enToCnChoice: 10,
    },
  },
};

export const mockWordbooks: WordbookSummary[] = [
  { id: 1, code: 'cet4', name: 'CET-4', category: 'exam', totalEntries: 4500, isActive: false },
  { id: 2, code: 'cet6', name: 'CET-6', category: 'exam', totalEntries: 3000, isActive: false },
  { id: 3, code: 'kaoyan', name: 'KaoYan', category: 'exam', totalEntries: 5500, isActive: true },
  { id: 4, code: 'medical', name: 'Medical English', category: 'specialized', totalEntries: 1800, isActive: false },
];

const studySeeds: Array<[string, string, string, string]> = [
  ['herald', '预告，先兆；传令官', 'a bowl of daffodils, the first bright heralds of spring', '一盆水仙花，春天的第一个鲜明的预兆'],
  ['resilient', '能迅速恢复的，有韧性的', 'The team stayed resilient during the release.', '团队在发布期间保持了韧性。'],
  ['meticulous', '一丝不苟的，极仔细的', 'A meticulous review caught the sync bug.', '一次细致的检查发现了同步问题。'],
  ['coherent', '连贯一致的，有条理的', 'The plan became coherent after the boundary was named.', '边界明确后计划变得连贯。'],
  ['fragile', '脆弱的，易碎的', 'The fragile cache failed after restart.', '脆弱的缓存重启后失效了。'],
  ['vivid', '生动的，鲜明的', 'The report gave a vivid picture of progress.', '报告生动展示了进展。'],
  ['subtle', '微妙的，不易察觉的', 'The subtle bug appeared only after refresh.', '这个隐蔽问题只在刷新后出现。'],
  ['robust', '强健的，可靠的', 'A robust session survives interruption.', '可靠的会话能够经受中断。'],
  ['pragmatic', '务实的，重实际的', 'The pragmatic fix preserved the old contract.', '务实的修复保留了旧契约。'],
  ['precise', '精确的，准确的', 'A precise answer avoids guesswork.', '精确答案能避免猜测。'],
  ['gradual', '逐渐的，渐进的', 'Gradual growth keeps review manageable.', '渐进增长让复习量保持可控。'],
  ['retain', '保留，保持', 'The plan must retain question weights.', '计划必须保留题型权重。'],
  ['derive', '源自，获得', 'Reports derive totals from answered questions.', '报告总数来自已答题目。'],
  ['verify', '核实，验证', 'Tests verify the learning chain.', '测试会验证学习链路。'],
  ['restore', '恢复，还原', 'The page should restore session state.', '页面应该恢复学习会话状态。'],
  ['distinct', '不同的，清楚区分的', 'Each question needs a distinct word.', '每道题都需要不同的单词。'],
  ['resolve', '解决，解析', 'The helper resolves the correct choice.', '辅助函数解析正确选项。'],
  ['stable', '稳定的', 'Stable ids keep progress accurate.', '稳定的编号让进度准确。'],
  ['native', '原生的，本地的', 'The mini app uses native WeChat events.', '小程序使用微信原生事件。'],
  ['compact', '紧凑的，简洁完整的', 'The mode chart uses a compact layout.', '模式图表使用紧凑布局。'],
  ['astronomy', '天文学', 'Astronomy appears in the wrong-word list.', '天文学出现在错词列表中。'],
  ['civilisation', '文明，文化', 'Civilisation was missed in review.', '复习时错过了文明这个词。'],
  ['cling', '紧握，坚持', 'Old progress should not cling to a new session.', '旧进度不应该附着到新会话上。'],
  ['valid', '有效的，合理的', 'Only valid options should be shown.', '只应展示有效选项。'],
  ['sanitize', '清洗，净化', 'The choices are sanitized before display.', '选项会在展示前清洗。'],
  ['distractor', '干扰项', 'A distractor must not duplicate the answer.', '干扰项不能重复答案。'],
  ['summary', '总结，概要', 'The summary appears after completion.', '完成后会出现总结。'],
  ['payload', '载荷，请求数据', 'The payload mirrors Flutter contracts.', '请求数据对齐 Flutter 契约。'],
  ['fixture', '测试数据', 'The fixture keeps local smoke tests varied.', '测试数据让本地冒烟测试保持变化。'],
  ['weighted', '加权的', 'Weighted question types affect sessions.', '题型权重会影响会话。'],
  ['segment', '片段，部分', 'The report segment shows daily data.', '报告片段展示每日数据。'],
  ['toggle', '切换，开关', 'A mode card toggles compact details.', '模式卡片切换紧凑详情。'],
  ['profile', '画像，档案', 'Croc BTI saves a profile.', '鳄bti 会保存学习画像。'],
  ['sync', '同步，使一致', 'Sync runs after applying a profile.', '应用画像后会执行同步。'],
];

const distractorMeanings = [
  '姿势，体态；看法，态度',
  '正当理由；许可证；委任状',
  '翻译；译文，译本',
  '短暂的，临时的',
  '嘈杂的，喧闹的',
  '违反规则的',
  '没有结构的，散乱的',
  '普通的，常见的',
  '提前的，在前的',
  '模糊的，不清楚的',
];

function rotateChoices(answer: string, index: number) {
  const distractors = distractorMeanings.filter((item) => item !== answer);
  const raw = [
    answer,
    distractors[index % distractors.length],
    distractors[(index + 3) % distractors.length],
    distractors[(index + 6) % distractors.length],
  ];
  const shift = index % 4;
  const ordered = raw.map((_, choiceIndex) => raw[(choiceIndex - shift + 4) % 4]);
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

function makeQuestions(
  mode: StudyMode,
  count: number,
  offset: number,
  questionType: StudyQuestion['questionType'],
): StudyQuestion[] {
  return Array.from({ length: count }, (_, index) => {
    const [word, meaning, sentence, translation] = studySeeds[(offset + index) % studySeeds.length];
    const choiceSet = rotateChoices(meaning, offset + index);
    return {
      questionId: `${mode}-${word}-${index + 1}`,
      questionType,
      entrySourceId: `entry-${word}`,
      word,
      prompt:
        questionType === 'exampleToCnChoice'
          ? '根据例句选择中文释义'
          : '选择最合适的中文释义',
      partOfSpeech: 'n',
      phoneticUk: `'${word}`,
      acceptedMeanings: [meaning],
      choices: choiceSet.choices,
      correctChoiceLabel: choiceSet.correctChoiceLabel,
      questionIndex: index + 1,
      totalQuestions: count,
      exampleSentence: sentence,
      exampleTranslation: translation,
      hasHint: index % 3 === 0,
      userHint: `${word} 的常见释义是：${meaning}`,
    };
  });
}

export interface RootAffixFallbackPayload {
  sourceId: string;
  form: string;
  meaningCn: string;
  exampleWords: string;
  exampleGlosses: string;
}

export const rootAffixFallbackPayloads: RootAffixFallbackPayload[] = [
  {
    sourceId: 'root-hyper',
    form: 'hyper-',
    meaningCn: '高、过度、超过',
    exampleWords: 'hypertension, hypercapnia, hyperactive',
    exampleGlosses: '高血压，二氧化碳过多，过度活跃',
  },
  {
    sourceId: 'root-hypo',
    form: 'hypo-',
    meaningCn: '低、不足、在下',
    exampleWords: 'hypoxia, hypoglycemia, hypothyroid',
    exampleGlosses: '缺氧，低血糖，甲状腺功能低下',
  },
  {
    sourceId: 'root-broncho',
    form: 'broncho-',
    meaningCn: '支气管',
    exampleWords: 'bronchitis, bronchoscopy, bronchodilator',
    exampleGlosses: '支气管炎，支气管镜检查，支气管扩张剂',
  },
  {
    sourceId: 'root-cardio',
    form: 'cardio-',
    meaningCn: '心脏',
    exampleWords: 'cardiology, cardiovascular, cardiomyopathy',
    exampleGlosses: '心脏病学，心血管的，心肌病',
  },
];

function makeRootAffixQuestions(payloads: RootAffixFallbackPayload[]): StudyQuestion[] {
  return payloads.map((payload, index) => ({
    questionId: `${payload.sourceId}-${index + 1}`,
    questionType: 'rootToGlossInput',
    entrySourceId: payload.sourceId,
    word: payload.form,
    prompt: '根据词根/词缀填写含义',
    acceptedMeanings: [payload.meaningCn],
    questionIndex: index + 1,
    totalQuestions: payloads.length,
    exampleSentence: payload.exampleWords,
    exampleTranslation: payload.exampleGlosses,
    hasHint: true,
    userHint: `${payload.form}：${payload.meaningCn}`,
  }));
}

export const questionBank: Record<StudyMode, StudyQuestion[]> = {
  newWord: makeQuestions('newWord', 20, 0, 'exampleToCnChoice'),
  review: makeQuestions('review', 28, 2, 'enToCnChoice'),
  mixedTest: makeQuestions('mixedTest', 34, 4, 'exampleToCnChoice'),
  wrongWordReinforcement: makeQuestions('wrongWordReinforcement', 26, 1, 'enToCnChoice'),
  rootAffix: makeRootAffixQuestions(rootAffixFallbackPayloads),
};

export const initialToday: TodayHomeState = {
  todayDate: '2026-05-21',
  activePlan: mockPlan,
  wordbooks: mockWordbooks,
  dailyProgress: {
    completedTasks: 0,
    totalTasks: 5,
    accuracyPercent: 0,
    streakDays: 0,
  },
  modeProgress: {},
  resumeHint: {
    hasResume: false,
  },
};

export const initialSession: StartSessionResponse = {
  session: {
    sessionId: 'session-demo',
    mode: 'review',
    totalWords: 28,
    startedAt: '2026-05-20T05:00:00Z',
  },
  currentQuestion: questionBank.review[0],
  progress: {
    current: 1,
    total: 28,
  },
  answeredQuestions: [],
};

export const initialWrongWords: WrongWordEntry[] = [
  {
    entryId: 101,
    word: 'resilient',
    meanings: ['able to recover quickly', '能迅速恢复的'],
    errorCount: 4,
    lastWrongAt: '2026-05-19T11:00:00Z',
    priorityScore: 92,
    hasHint: true,
    userHint: 'recover quickly after stress',
  },
  {
    entryId: 102,
    word: 'meticulous',
    meanings: ['very careful and precise', '一丝不苟的'],
    errorCount: 3,
    lastWrongAt: '2026-05-18T10:00:00Z',
    priorityScore: 77,
    hasHint: false,
  },
];

export const initialWrongWordDetail: WrongWordDetail = {
  ...initialWrongWords[0],
  examples: [
    {
      sentence: 'A resilient learner returns after mistakes.',
      translation: '一个有韧性的学习者会在犯错后回来继续尝试。',
    },
  ],
  errorHistory: [
    { outcome: 'incorrect', answeredAt: '2026-05-19T11:00:00Z' },
    { outcome: 'skipped', answeredAt: '2026-05-18T11:00:00Z' },
  ],
  relatedWords: ['recover', 'adaptable', 'persistent'],
};

export const initialReports: ReportsOverview = {
  totalStudyDays: 16,
  totalWordsLearned: 184,
  totalQuestionsAnswered: 920,
  overallAccuracy: 84,
  streakInfo: {
    currentStreak: 7,
    longestStreak: 11,
  },
  dailySeries: [
    { date: '2026-05-14', totalQuestions: 42, correctCount: 34, accuracyPercent: 81, studyTimeMs: 310000 },
    { date: '2026-05-15', totalQuestions: 50, correctCount: 45, accuracyPercent: 90, studyTimeMs: 420000 },
    { date: '2026-05-16', totalQuestions: 46, correctCount: 39, accuracyPercent: 85, studyTimeMs: 360000 },
    { date: '2026-05-17', totalQuestions: 54, correctCount: 47, accuracyPercent: 87, studyTimeMs: 480000 },
    { date: '2026-05-18', totalQuestions: 44, correctCount: 36, accuracyPercent: 82, studyTimeMs: 330000 },
    { date: '2026-05-19', totalQuestions: 58, correctCount: 50, accuracyPercent: 86, studyTimeMs: 520000 },
    { date: '2026-05-20', totalQuestions: 18, correctCount: 16, accuracyPercent: 89, studyTimeMs: 416000 },
  ],
  modeBreakdown: [
    { mode: 'newWord', totalQuestions: 240, correctCount: 198 },
    { mode: 'review', totalQuestions: 430, correctCount: 372 },
    { mode: 'mixedTest', totalQuestions: 160, correctCount: 128 },
    { mode: 'wrongWordReinforcement', totalQuestions: 90, correctCount: 75 },
    { mode: 'rootAffix', totalQuestions: 50, correctCount: 1 },
  ],
  modeSeries: {
    newWord: [
      { date: '2026-05-14', totalQuestions: 30, correctCount: 18, accuracyPercent: 60 },
      { date: '2026-05-16', totalQuestions: 42, correctCount: 35, accuracyPercent: 83 },
      { date: '2026-05-20', totalQuestions: 240, correctCount: 198, accuracyPercent: 83 },
    ],
    review: [
      { date: '2026-05-14', totalQuestions: 80, correctCount: 60, accuracyPercent: 75 },
      { date: '2026-05-17', totalQuestions: 120, correctCount: 105, accuracyPercent: 88 },
      { date: '2026-05-20', totalQuestions: 430, correctCount: 372, accuracyPercent: 87 },
    ],
    mixedTest: [
      { date: '2026-05-14', totalQuestions: 40, correctCount: 22, accuracyPercent: 55 },
      { date: '2026-05-18', totalQuestions: 80, correctCount: 64, accuracyPercent: 80 },
      { date: '2026-05-20', totalQuestions: 160, correctCount: 128, accuracyPercent: 80 },
    ],
    wrongWordReinforcement: [
      { date: '2026-05-14', totalQuestions: 20, correctCount: 8, accuracyPercent: 40 },
      { date: '2026-05-18', totalQuestions: 60, correctCount: 48, accuracyPercent: 80 },
      { date: '2026-05-20', totalQuestions: 90, correctCount: 75, accuracyPercent: 83 },
    ],
    rootAffix: [
      { date: '2026-05-14', totalQuestions: 20, correctCount: 0, accuracyPercent: 0 },
      { date: '2026-05-20', totalQuestions: 50, correctCount: 1, accuracyPercent: 2 },
    ],
  },
};

export const initialReward: TodayRewardState = {
  todayDate: '2026-05-21',
  rewardId: 'reward-001-com-hihonor-photos-20260329190156-edit-752572937',
  claimedAt: undefined,
  asset: {
    rewardId: 'reward-001-com-hihonor-photos-20260329190156-edit-752572937',
    title: 'Reward 001',
    imageUrl: '/assets/rewards/webp_q60_540/reward_001.webp',
    mimeType: 'image/webp',
  },
};

const metricLabels: Record<LeaderboardMetric, string> = {
  weekly: '周榜题数',
  monthly: '月榜题数',
  allTime: '累计词数',
  accuracy: '正确率',
  mixedAccuracy: '混测正确率',
  streak: '连续学习',
};

const baseLeaderboardRows = [
  { internalUserId: 'demo-user', displayName: '我', weekly: 86, monthly: 312, allTime: 184, accuracy: 84, mixedAccuracy: 80, streak: 7 },
  { internalUserId: 'user-river', displayName: '河岸同学', weekly: 118, monthly: 440, allTime: 290, accuracy: 88, mixedAccuracy: 83, streak: 9 },
  { internalUserId: 'user-luna', displayName: '月亮同学', weekly: 72, monthly: 355, allTime: 251, accuracy: 91, mixedAccuracy: 86, streak: 5 },
  { internalUserId: 'user-moss', displayName: '青苔同学', weekly: 64, monthly: 221, allTime: 198, accuracy: 78, mixedAccuracy: 72, streak: 12 },
];

export function leaderboardFromReports(
  reports: ReportsOverview,
  metric: LeaderboardMetric = 'weekly',
): LeaderboardSummary {
  const currentWeekQuestions = reports.dailySeries.reduce(
    (total, row) => total + (row.totalQuestions ?? row.questionsAnswered ?? 0),
    0,
  );
  const mixed = reports.modeBreakdown.find((row) => row.mode === 'mixedTest');
  const mixedAccuracy = mixed
    ? Math.round(((mixed.correctCount ?? 0) / (mixed.totalQuestions ?? mixed.questionsAnswered ?? 1)) * 100)
    : 0;

  const rows = baseLeaderboardRows.map((row) =>
    row.internalUserId === 'demo-user'
      ? {
          ...row,
          weekly: currentWeekQuestions,
          monthly: reports.totalQuestionsAnswered,
          allTime: reports.totalWordsLearned,
          accuracy: reports.overallAccuracy,
          mixedAccuracy,
          streak: reports.streakInfo.currentStreak,
        }
      : row,
  );

  const entries = rows
    .map((row) => ({
      rank: 0,
      internalUserId: row.internalUserId,
      displayName: row.displayName,
      score: row[metric],
      scoreLabel: `${row[metric]}${metric.includes('Accuracy') || metric === 'accuracy' ? '%' : metric === 'streak' ? '天' : ''}`,
      isCurrentUser: row.internalUserId === 'demo-user',
    }))
    .sort((a, b) => b.score - a.score)
    .map((entry, index) => ({ ...entry, rank: index + 1 }));

  return {
    metric,
    periodLabel: metricLabels[metric],
    generatedAt: new Date().toISOString(),
    currentUser: entries.find((entry) => entry.isCurrentUser),
    entries,
  };
}
