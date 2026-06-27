export type StudyMode =
  | 'newWord'
  | 'review'
  | 'mixedTest'
  | 'wrongWordReinforcement'
  | 'rootAffix';

export interface AuthSessionState {
  user: {
    internalUserId: string;
    primaryIdentity: 'wechat_mp' | 'email';
    hasEmailBinding: boolean;
    hasWechatBinding: boolean;
  };
  accountState: {
    phase: 'guest_local_only' | 'signed_in_active' | 'merge_decision_required';
    needsBindDecision: boolean;
    cloudDataState: 'empty_or_ready' | 'needs_merge' | 'unavailable';
  };
}

export interface AuthSessionResponse extends AuthSessionState {
  session: {
    accessToken: string;
    refreshToken: string;
    expiresAt: string;
  };
}

export interface EmailBindStartResponse {
  email: string;
  targetSupabaseUserId?: string;
  mergeDecisionId?: string;
  requiresConfirmation: boolean;
  message: string;
}

export interface PlanSummary {
  id: number;
  name: string;
  newWordsPerDay: number;
  reviewWordsPerDay: number;
  mixedTestPerDay: number;
  wrongWordTestPerDay: number;
  rootAffixPerDay?: number;
  growthRuleEnabled: boolean;
  growthRuleMode: 'shared' | 'perMode' | string;
  growthIntervalDays: number;
  growthIncrement: number;
  sharedGrowthRule?: { intervalDays: number; increment: number };
  growthRulesByMode?: Record<string, { intervalDays: number; increment: number }>;
  questionTypeWeightsByMode?: Record<string, Record<string, number>>;
}

export interface WordbookSummary {
  id: number;
  code: string;
  name: string;
  category: string;
  totalEntries: number;
  isActive: boolean;
}

export interface ResumeSessionHint {
  hasResume: boolean;
  sessionId?: string;
  mode?: StudyMode;
  current?: number;
  total?: number;
  word?: string;
}

export interface TodayHomeState {
  todayDate: string;
  activePlan?: PlanSummary;
  wordbooks: WordbookSummary[];
  dailyProgress: {
    completedTasks: number;
    totalTasks: number;
    accuracyPercent: number;
    streakDays: number;
  };
  modeProgress?: Partial<Record<StudyMode, { completed: number; total: number; isComplete?: boolean }>>;
  resumeHint?: ResumeSessionHint;
}

export interface StudyQuestion {
  questionId: string;
  questionType:
    | 'enToCnChoice'
    | 'cnToEnChoice'
    | 'exampleToCnChoice'
    | 'exampleToCnChoiceNoTranslation'
    | 'enToCnInput'
    | 'wordSkeletonInput'
    | 'glossToRootInput'
      | 'rootToGlossInput';
  entrySourceId: string;
  entryId?: number;
  word: string;
  prompt: string;
  partOfSpeech?: string;
  phoneticUs?: string;
  phoneticUk?: string;
  acceptedMeanings: string[];
  choices?: Array<{ label: string; text?: string; value?: string }>;
  correctChoiceLabel?: string;
  questionIndex: number;
  totalQuestions: number;
  exampleSentence?: string;
  exampleTranslation?: string;
  userHint?: string;
  hasHint: boolean;
}

export interface StudyResult {
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

export interface StartSessionEntryPayload {
  sourceId: string;
  word: string;
  partOfSpeech?: string;
  frequency?: number;
  phoneticUs?: string;
  phoneticUk?: string;
  meanings?: string[];
  exampleSentence?: string;
  exampleTranslation?: string;
}

export interface StartSessionResponse {
  session: {
    sessionId: string;
    mode: StudyMode;
    totalWords: number;
    startedAt: string;
  };
  currentQuestion: StudyQuestion;
  progress: {
    current: number;
    total: number;
  };
  answeredQuestions: Array<{ question: StudyQuestion; result: StudyResult }>;
  questions?: StudyQuestion[];
}

export interface StudyStartOptions {
  mode: StudyMode;
  wordbookId?: number;
  entrySourceIds?: string[];
  entryPayloads?: StartSessionEntryPayload[];
  distractorPayloads?: StartSessionEntryPayload[];
  questionTypeWeights?: Array<Record<string, unknown>> | null;
}

export interface SubmitAnswerResponse {
  result: StudyResult;
  currentQuestion?: StudyQuestion;
  progress: {
    current: number;
    total: number;
  };
  answeredQuestions: Array<{ question: StudyQuestion; result: StudyResult }>;
  isComplete: boolean;
  summary?: {
    sessionId?: string;
    totalQuestions: number;
    correctCount: number;
    fuzzyCorrectCount?: number;
    incorrectCount?: number;
    skippedCount?: number;
    totalWords?: number;
    wrongWordCount?: number;
    accuracyPercent: number;
    totalTimeMs?: number;
    completedAt: string;
  };
  nextAction?: string;
  hintPrompt?: {
    entryId: number;
    word: string;
    errorCount: number;
    triggerOutcome: string;
    suggestions: Array<{ id?: string; text: string; source?: string }>;
  };
}

export interface MarkStudyEntryMasteredResponse {
  entrySourceId: string;
  entryId?: number;
  prunedQuestionCount: number;
  isComplete: boolean;
  currentQuestion?: StudyQuestion;
  summary?: SubmitAnswerResponse['summary'];
  nextAction?: string;
  progress: {
    current: number;
    total: number;
  };
  answeredQuestions: Array<{ question: StudyQuestion; result: StudyResult }>;
}

export interface AcceptDisputedMeaningResponse {
  result: StudyResult;
  progress: {
    current: number;
    total: number;
  };
  answeredQuestions: Array<{ question: StudyQuestion; result: StudyResult }>;
  entrySourceId: string;
  word: string;
  acceptedMeaning: string;
}

export interface CompleteSessionResponse {
  summary: NonNullable<SubmitAnswerResponse['summary']>;
  nextAction: string;
}

export interface PreserveStudyProgressResponse {
  resumeHint: ResumeSessionHint;
  today: TodayHomeState;
}

export interface StoredStudySessionSnapshot {
  session: StartSessionResponse;
  questions: StudyQuestion[];
  savedAt: string;
}

export interface WrongWordEntry {
  entryId: number;
  word: string;
  meanings: string[];
  errorCount: number;
  lastWrongAt?: string;
  priorityScore: number;
  hasHint: boolean;
  userHint?: string;
}

export interface WrongWordDetail extends WrongWordEntry {
  examples: Array<{ sentence: string; translation?: string }>;
  errorHistory: Array<{ outcome: string; answeredAt: string }>;
  relatedWords: string[];
  rootsAffixes?: string[];
}

export interface ReportsOverview {
  totalStudyDays: number;
  totalWordsLearned: number;
  totalQuestionsAnswered: number;
  overallAccuracy: number;
  streakInfo: {
    currentStreak: number;
    longestStreak: number;
  };
  dailySeries: Array<{
    date: string;
    totalQuestions?: number;
    questionsAnswered?: number;
    correctCount?: number;
    accuracyPercent?: number;
    accuracy?: number;
    studyTimeMs?: number;
    totalTimeMs?: number;
  }>;
  modeBreakdown: Array<{
    mode: StudyMode;
    totalQuestions?: number;
    questionsAnswered?: number;
    correctCount?: number;
    accuracyPercent?: number;
    accuracy?: number;
  }>;
  modeSeries: Partial<
    Record<
      StudyMode,
      Array<{
        date: string;
        totalQuestions?: number;
        questionsAnswered?: number;
        correctCount?: number;
        accuracyPercent?: number;
        accuracy?: number;
        studyTimeMs?: number;
        totalTimeMs?: number;
      }>
    >
  >;
}

export interface CrocBtiProfile {
  resultCode?: string;
  title?: string;
  summary?: string;
  advice?: string;
  assetPath?: string;
  flavor?: string;
  answers?: Record<string, number>;
  axisScores?: Record<string, unknown>;
  weights?: Record<string, number>;
  planInput?: Record<string, number>;
  questionTypeWeightsByMode?: Record<string, Record<string, number>>;
  dailyLearningMinutes?: number;
  growthRuleEnabled?: boolean;
  source?: string;
  version?: number;
  evaluatedAt?: string;
}

export interface TodayRewardState {
  todayDate: string;
  rewardId?: string;
  claimedAt?: string;
  canClaim?: boolean;
  asset?: {
    rewardId: string;
    title?: string;
    imageUrl: string;
    mimeType: string;
  };
}

export type LeaderboardMetric =
  | 'weekly'
  | 'monthly'
  | 'allTime'
  | 'accuracy'
  | 'mixedAccuracy'
  | 'streak';

export interface LeaderboardEntry {
  rank: number;
  internalUserId: string;
  displayName: string;
  avatarUrl?: string;
  score: number;
  scoreLabel: string;
  isCurrentUser: boolean;
}

export interface LeaderboardSummary {
  metric: LeaderboardMetric;
  periodLabel: string;
  generatedAt: string;
  currentUser?: LeaderboardEntry;
  entries: LeaderboardEntry[];
}

export interface SyncStatus {
  phase: 'idle' | 'syncing' | 'offline';
  pendingItems: number;
  lastSyncedAt?: string;
}
