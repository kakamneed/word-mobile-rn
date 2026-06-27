export const defaultPlan = {
  id: 1,
  name: 'Balanced Study',
  newWordsPerDay: 12,
  reviewWordsPerDay: 24,
  mixedTestPerDay: 10,
  wrongWordTestPerDay: 8,
  rootAffixPerDay: 4,
  growthRuleEnabled: true,
};

export const defaultReports = {
  totalStudyDays: 16,
  totalWordsLearned: 184,
  totalQuestionsAnswered: 920,
  overallAccuracy: 84,
  streakInfo: {
    currentStreak: 7,
    longestStreak: 11,
  },
  dailySeries: [
    { date: '2026-05-14', totalQuestions: 42, correctCount: 34 },
    { date: '2026-05-15', totalQuestions: 50, correctCount: 45 },
    { date: '2026-05-16', totalQuestions: 46, correctCount: 39 },
    { date: '2026-05-17', totalQuestions: 54, correctCount: 47 },
    { date: '2026-05-18', totalQuestions: 44, correctCount: 36 },
    { date: '2026-05-19', totalQuestions: 58, correctCount: 50 },
    { date: '2026-05-20', totalQuestions: 18, correctCount: 16 },
  ],
  modeBreakdown: [
    { mode: 'newWord', totalQuestions: 240, correctCount: 198 },
    { mode: 'review', totalQuestions: 430, correctCount: 372 },
    { mode: 'mixedTest', totalQuestions: 160, correctCount: 128 },
    { mode: 'wrongWordReinforcement', totalQuestions: 90, correctCount: 75 },
  ],
};

export const defaultReward = {
  todayDate: '2026-05-20',
  rewardId: 'leaf-badge',
  claimedAt: '2026-05-20T05:10:00Z',
  asset: {
    rewardId: 'leaf-badge',
    title: 'Steady Leaf',
    imageUrl: 'https://example.invalid/rewards/leaf-badge.webp',
    mimeType: 'image/webp',
  },
};

export const questionBank = {
  newWord: [
    {
      questionId: 'new-1',
      questionType: 'enToCnChoice',
      entrySourceId: 'entry-resilient',
      word: 'resilient',
      prompt: 'Choose the best meaning.',
      acceptedMeanings: ['able to recover quickly'],
      choices: [
        { label: 'A', value: 'able to recover quickly' },
        { label: 'B', value: 'easy to break' },
        { label: 'C', value: 'short lived' },
        { label: 'D', value: 'unclear' },
      ],
      questionIndex: 1,
      totalQuestions: 3,
      exampleSentence: 'The team stayed resilient during the long release.',
      exampleTranslation: 'The team recovered and kept moving.',
      hasHint: true,
      userHint: 'recover quickly after stress',
    },
  ],
  review: [
    {
      questionId: 'review-1',
      questionType: 'enToCnChoice',
      entrySourceId: 'entry-meticulous',
      word: 'meticulous',
      prompt: 'Choose the best meaning.',
      acceptedMeanings: ['very careful and precise'],
      choices: [
        { label: 'A', value: 'very careful and precise' },
        { label: 'B', value: 'careless' },
        { label: 'C', value: 'temporary' },
        { label: 'D', value: 'noisy' },
      ],
      questionIndex: 1,
      totalQuestions: 3,
      exampleSentence: 'A meticulous review caught the sync bug.',
      hasHint: false,
    },
  ],
  mixedTest: [
    {
      questionId: 'mixed-1',
      questionType: 'exampleToCnChoice',
      entrySourceId: 'entry-coherent',
      word: 'coherent',
      prompt: 'Pick the word that fits the sentence.',
      acceptedMeanings: ['logical and consistent'],
      choices: [
        { label: 'A', value: 'logical and consistent' },
        { label: 'B', value: 'random' },
        { label: 'C', value: 'late' },
        { label: 'D', value: 'silent' },
      ],
      questionIndex: 1,
      totalQuestions: 3,
      exampleSentence: 'The plan became coherent after the auth boundary was named.',
      hasHint: false,
    },
  ],
  wrongWordReinforcement: [
    {
      questionId: 'wrong-1',
      questionType: 'enToCnChoice',
      entrySourceId: 'entry-resilient',
      word: 'resilient',
      prompt: 'This word is in your wrong-word queue.',
      acceptedMeanings: ['able to recover quickly'],
      choices: [
        { label: 'A', value: 'able to recover quickly' },
        { label: 'B', value: 'fragile' },
        { label: 'C', value: 'hidden' },
        { label: 'D', value: 'ordinary' },
      ],
      questionIndex: 1,
      totalQuestions: 3,
      hasHint: true,
      userHint: 'recover quickly after stress',
    },
  ],
  rootAffix: [
    {
      questionId: 'root-1',
      questionType: 'enToCnChoice',
      entrySourceId: 'root-re',
      word: 're-',
      prompt: 'Choose the root or affix meaning.',
      acceptedMeanings: ['again or back'],
      choices: [
        { label: 'A', value: 'again or back' },
        { label: 'B', value: 'before' },
        { label: 'C', value: 'against' },
        { label: 'D', value: 'between' },
      ],
      questionIndex: 1,
      totalQuestions: 3,
      hasHint: false,
    },
  ],
};

export const defaultWrongWords = [
  {
    entryId: 101,
    word: 'resilient',
    meanings: ['able to recover quickly'],
    errorCount: 4,
    lastWrongAt: '2026-05-19T11:00:00Z',
    priorityScore: 92,
    hasHint: true,
    userHint: 'recover quickly after stress',
  },
  {
    entryId: 102,
    word: 'meticulous',
    meanings: ['very careful and precise'],
    errorCount: 3,
    lastWrongAt: '2026-05-18T10:00:00Z',
    priorityScore: 77,
    hasHint: false,
  },
];

export function wrongWordDetailFromEntry(entry = defaultWrongWords[0]) {
  return {
    ...entry,
    examples: [
      {
        sentence: `A learner reviews ${entry.word} after mistakes.`,
        translation: 'The learner returns to the difficult word.',
      },
    ],
    errorHistory: [
      { outcome: 'incorrect', answeredAt: entry.lastWrongAt ?? '2026-05-20T00:00:00Z' },
    ],
    relatedWords: ['recover', 'adaptable', 'persistent'],
  };
}

export function firstQuestionForMode(mode) {
  return questionBank[mode]?.[0] ?? questionBank.review[0];
}

export function isCorrect(question, response) {
  const normalized = String(response ?? '').trim().toLowerCase();
  return question.acceptedMeanings.some(
    (meaning) => meaning.trim().toLowerCase() === normalized,
  );
}

export function applyAnswerToReports(reports, mode, correct) {
  const nextReports = structuredClone(reports);
  const previousCorrect = Math.round(
    (nextReports.overallAccuracy / 100) * nextReports.totalQuestionsAnswered,
  );
  const nextQuestions = nextReports.totalQuestionsAnswered + 1;
  const nextCorrect = previousCorrect + (correct ? 1 : 0);
  nextReports.totalQuestionsAnswered = nextQuestions;
  nextReports.overallAccuracy = Math.round((nextCorrect / nextQuestions) * 100);
  const lastDaily = nextReports.dailySeries[nextReports.dailySeries.length - 1];
  if (lastDaily) {
    lastDaily.totalQuestions += 1;
    lastDaily.correctCount += correct ? 1 : 0;
  }
  const modeRow = nextReports.modeBreakdown.find((row) => row.mode === mode);
  if (modeRow) {
    modeRow.totalQuestions += 1;
    modeRow.correctCount += correct ? 1 : 0;
  } else {
    nextReports.modeBreakdown.push({
      mode,
      totalQuestions: 1,
      correctCount: correct ? 1 : 0,
    });
  }
  return nextReports;
}

export function todayFromPlanAndReports(plan = defaultPlan, reports = defaultReports) {
  return {
    todayDate: '2026-05-20',
    activePlan: plan,
    wordbooks: [
      {
        id: 1,
        code: 'cet4',
        name: 'CET-4 Core',
        category: 'exam',
        totalEntries: 2600,
        isActive: true,
      },
    ],
    dailyProgress: {
      completedTasks: 2,
      totalTasks: 5,
      accuracyPercent: reports.overallAccuracy,
      streakDays: reports.streakInfo.currentStreak,
    },
    resumeHint: {
      hasResume: false,
    },
  };
}

export function leaderboardFromReports(reports = defaultReports, metric = 'weekly') {
  const currentWeekQuestions = reports.dailySeries.reduce(
    (total, row) => total + row.totalQuestions,
    0,
  );
  const mixed = reports.modeBreakdown.find((row) => row.mode === 'mixedTest');
  const mixedAccuracy = mixed
    ? Math.round((mixed.correctCount / mixed.totalQuestions) * 100)
    : 0;
  const rows = [
    {
      internalUserId: 'demo-user',
      displayName: 'You',
      weekly: currentWeekQuestions,
      monthly: reports.totalQuestionsAnswered,
      allTime: reports.totalWordsLearned,
      accuracy: reports.overallAccuracy,
      mixedAccuracy,
      streak: reports.streakInfo.currentStreak,
    },
    {
      internalUserId: 'user-river',
      displayName: 'River',
      weekly: 118,
      monthly: 440,
      allTime: 290,
      accuracy: 88,
      mixedAccuracy: 83,
      streak: 9,
    },
  ];
  const entries = rows
    .map((row) => ({
      rank: 0,
      internalUserId: row.internalUserId,
      displayName: row.displayName,
      score: row[metric],
      scoreLabel: `${row[metric]}${metric.includes('Accuracy') ? '%' : ''}`,
      isCurrentUser: row.internalUserId === 'demo-user',
    }))
    .sort((a, b) => b.score - a.score)
    .map((entry, index) => ({ ...entry, rank: index + 1 }));

  return {
    metric,
    periodLabel: metric,
    generatedAt: new Date().toISOString(),
    currentUser: entries.find((entry) => entry.isCurrentUser),
    entries,
  };
}
