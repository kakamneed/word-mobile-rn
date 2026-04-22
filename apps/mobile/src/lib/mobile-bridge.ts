import {NativeModules, Platform} from 'react-native';

interface WordCoreNativeModule {
  getBootstrapState(): Promise<string>;
  markOnboardingCompleted(): Promise<void>;
  getTodayHomeState(): Promise<string>;
  getSettings(): Promise<string>;
  getAiProviderConfig(): Promise<string>;
  saveAiProviderConfig(requestJson: string): Promise<string>;
  getActivePlan(): Promise<string>;
  savePlan(requestJson: string): Promise<string>;
  applySavedPlanToToday(): Promise<string>;
  getWordbooks(): Promise<string>;
  toggleWordbook(wordbookId: number, isActive: boolean): Promise<void>;
  startStudySession(requestJson: string): Promise<string>;
  submitStudyAnswer(requestJson: string): Promise<string>;
  completeStudySession(sessionId: string): Promise<string>;
  cancelStudySession(sessionId: string): Promise<void>;
  getReportsOverview(): Promise<string>;
  getWrongWords(filter: string): Promise<string>;
  getWrongWordDetail(entryId: number): Promise<string>;
  getTodayAiPassageContext(): Promise<string>;
  generateAiPassage(requestJson: string): Promise<string>;
  getAiPassageHistory(): Promise<string>;
  getAiPassage(passageId: string): Promise<string>;
  saveAiPassage(requestJson: string): Promise<void>;
}

const {WordCoreModule} = NativeModules as {WordCoreModule?: WordCoreNativeModule};

export interface BootstrapState {
  appReady: boolean;
  firstRunRequired: boolean;
  databaseStatus: 'ready' | 'init_failed';
  snapshotStatus: 'ready' | 'missing_required' | 'empty';
  connectivityStatus: 'offline' | 'online';
  aiConfigStatus: 'configured' | 'missing';
  settingsEntryAvailable: boolean;
  blockingReason: string | null;
}

export interface PlanSummary {
  id: number;
  name: string;
  newWordsPerDay: number;
  reviewWordsPerDay: number;
  mixedTestPerDay: number;
  wrongWordTestPerDay: number;
  rootAffixPerDay?: number;
  growthIntervalDays: number;
  growthIncrement: number;
  growthRuleMode?: 'shared' | 'perMode';
  sharedGrowthRule?: GrowthRule;
  growthRulesByMode?: Partial<Record<GrowthModeKey, GrowthRule>>;
}

export type GrowthModeKey =
  | 'newWord'
  | 'review'
  | 'mixedTest'
  | 'wrongWordReinforcement'
  | 'rootAffix';

export interface GrowthRule {
  intervalDays: number;
  increment: number;
}

export interface PlanEditInput {
  name?: string;
  newWordsPerDay?: number;
  reviewWordsPerDay?: number;
  mixedTestPerDay?: number;
  wrongWordTestPerDay?: number;
  rootAffixPerDay?: number;
  growthIntervalDays?: number;
  growthIncrement?: number;
  growthRuleMode?: 'shared' | 'perMode';
  sharedGrowthRule?: GrowthRule;
  growthRulesByMode?: Partial<Record<GrowthModeKey, GrowthRule>>;
}

export interface DailySnapshot {
  date: string;
  newWordsTarget: number;
  newWordsBaseTarget?: number;
  newWordsCarryoverTarget?: number;
  newWordsCompleted: number;
  reviewWordsTarget: number;
  reviewWordsBaseTarget?: number;
  reviewWordsCarryoverTarget?: number;
  reviewWordsCompleted: number;
  mixedTestTarget: number;
  mixedTestBaseTarget?: number;
  mixedTestCarryoverTarget?: number;
  mixedTestCompleted: number;
  wrongWordTestTarget: number;
  wrongWordTestBaseTarget?: number;
  wrongWordTestCarryoverTarget?: number;
  wrongWordTestCompleted: number;
  rootAffixTarget?: number;
  rootAffixBaseTarget?: number;
  rootAffixCarryoverTarget?: number;
  rootAffixCompleted?: number;
}

export interface WordbookSummary {
  id: number;
  code: string;
  name: string;
  category: string;
  totalEntries: number;
  isActive: boolean;
}

export interface DailyProgress {
  totalTasks: number;
  completedTasks: number;
  nextRecommendedAction: string;
}

export interface TodayHomeState {
  todayDate: string;
  activePlan: PlanSummary | null;
  todaySnapshot: DailySnapshot | null;
  wordbooks: WordbookSummary[];
  dailyProgress: DailyProgress;
}

export interface SettingsSummary {
  aiConfigured: boolean;
  syncConfigured: boolean;
  appVersion: string;
  schemaVersion: number;
  databasePath: string;
}

export interface AiProviderProfile {
  provider: string;
  baseUrl: string;
  model: string;
  hasAuthToken: boolean;
  authTokenPreview: string;
}

export interface AiProviderConfig {
  primary: AiProviderProfile;
  backup: AiProviderProfile;
}

export type SessionMode =
  | 'newWord'
  | 'review'
  | 'mixedTest'
  | 'wrongWordReinforcement'
  | 'rootAffix';

export type QuestionType =
  | 'enToCnChoice'
  | 'exampleToCnChoice'
  | 'cnToEnChoice'
  | 'enToCnInput'
  | 'glossToRootInput'
  | 'rootToGlossInput';

export interface ChoiceOption {
  text: string;
  label: string;
}

export interface StudyQuestion {
  questionId: string;
  questionType: QuestionType;
  entrySourceId: string;
  word: string;
  partOfSpeech?: string | null;
  phoneticUs?: string | null;
  phoneticUk?: string | null;
  prompt: string;
  acceptedMeanings: string[];
  exampleSentence: string | null;
  exampleTranslation: string | null;
  choices: ChoiceOption[] | null;
  correctChoiceLabel: string | null;
  questionIndex: number;
  totalQuestions: number;
}

export interface StudySession {
  sessionId: string;
  mode: SessionMode;
  totalWords: number;
  wordbookId: number | null;
  startedAt: string;
}

export interface SessionProgress {
  current: number;
  total: number;
}

export interface StartSessionRequest {
  mode: SessionMode;
  wordbookId: number | null;
  entrySourceIds: string[];
}

export interface StartSessionResponse {
  session: StudySession;
  currentQuestion: StudyQuestion;
  progress: SessionProgress;
}

export type AnswerOutcome = 'correct' | 'fuzzyCorrect' | 'incorrect' | 'skipped';

export interface StudyResult {
  questionId: string;
  entrySourceId: string;
  questionType: QuestionType;
  userResponse: string;
  normalizedResponse: string | null;
  correctAnswer: string;
  outcome: AnswerOutcome;
  responseTimeMs: number;
  answeredAt: string;
}

export interface SessionSummary {
  sessionId: string;
  totalQuestions: number;
  correctCount: number;
  fuzzyCorrectCount: number;
  incorrectCount: number;
  skippedCount: number;
  totalWords: number;
  wrongWordCount: number;
  accuracyPercent: number;
  totalTimeMs: number;
  completedAt: string;
}

export interface SubmitAnswerRequest {
  questionId: string;
  response: string;
  responseTimeMs: number;
}

export interface SubmitAnswerResponse {
  result: StudyResult;
  isComplete: boolean;
  currentQuestion: StudyQuestion | null;
  summary: SessionSummary | null;
  nextAction: string | null;
  progress: SessionProgress;
}

export interface CompleteSessionResponse {
  summary: SessionSummary;
  nextAction: string;
}

export interface DailyStat {
  date: string;
  totalQuestions: number;
  correctCount: number;
  accuracyPercent: number;
  studyTimeMs: number;
}

export interface StreakInfo {
  currentStreak: number;
  longestStreak: number;
  lastStudyDate: string | null;
}

export interface ModeBreakdown {
  mode: 'newWord' | 'review' | 'mixedTest' | 'wrongWordReinforcement' | 'rootAffix';
  totalQuestions: number;
  correctCount: number;
  accuracyPercent: number;
}

export interface ReportsOverview {
  totalStudyDays: number;
  totalWordsLearned: number;
  totalQuestionsAnswered: number;
  overallAccuracy: number;
  streakInfo: StreakInfo;
  modeBreakdown: ModeBreakdown[];
  last7Days: DailyStat[];
  dailySeries: DailyStat[];
  modeSeries: Partial<Record<ModeBreakdown['mode'], DailyStat[]>>;
}

export interface WrongWordEntry {
  entryId: number;
  word: string;
  phoneticUs: string | null;
  phoneticUk: string | null;
  meanings: string[];
  errorCount: number;
  lastWrongAt: string;
  priorityScore: number;
  isActive: boolean;
}

export interface WrongWordDetail {
  entryId: number;
  word: string;
  lemma: string;
  phoneticUs: string | null;
  phoneticUk: string | null;
  partOfSpeech: string;
  meanings: {pos: string; meaningCn: string; meaningEn?: string}[];
  examples: {sentenceEn: string; sentenceCn: string}[];
  errorHistory: {date: string; context: string}[];
  riskBreakdown: {
    questionType: QuestionType;
    attempts: number;
    incorrect: number;
    skipped: number;
    fuzzyCorrect: number;
    correct: number;
  }[];
  relatedWords: string[];
}

export interface WrongWordInput {
  entryId: number;
  word: string;
  primaryGloss: string;
  partOfSpeech: string | null;
}

export interface TodayAiPassageContext {
  date: string;
  tasksComplete: boolean;
  wrongWords: WrongWordInput[];
}

export interface PassageSegment {
  type: 'text' | 'word';
  text: string;
  entryId?: number;
  glossZh?: string;
  highlighted?: boolean;
}

export interface PassageBlock {
  blockType: string;
  segments: PassageSegment[];
}

export interface AIPassage {
  passageId: string;
  title: string;
  blocks: PassageBlock[];
  wrongWords: WrongWordInput[];
  coveredWordIds: number[];
  missingWordIds: number[];
  validationStatus: 'passed' | 'failed' | 'pending';
  failureReason: string | null;
  wordCount: number;
  targetLevel: string;
  generatedAt: string;
  preview: string;
}

export interface AIPassageHistoryItem {
  passageId: string;
  title: string;
  preview: string;
  wordCount: number;
  generatedAt: string;
  validationStatus: 'passed' | 'failed' | 'pending';
}

function ensureModule(): WordCoreNativeModule {
  if (!WordCoreModule) {
    throw new Error(`WordCoreModule is not available. Platform: ${Platform.OS}`);
  }
  return WordCoreModule;
}

export async function getBootstrapState(): Promise<BootstrapState> {
  const json = await ensureModule().getBootstrapState();
  return JSON.parse(json) as BootstrapState;
}

export async function markOnboardingCompleted(): Promise<void> {
  return ensureModule().markOnboardingCompleted();
}

export async function getTodayHomeState(): Promise<TodayHomeState> {
  const json = await ensureModule().getTodayHomeState();
  return JSON.parse(json) as TodayHomeState;
}

export async function getSettings(): Promise<SettingsSummary> {
  const json = await ensureModule().getSettings();
  return JSON.parse(json) as SettingsSummary;
}

export async function getAiProviderConfig(): Promise<AiProviderConfig> {
  const json = await ensureModule().getAiProviderConfig();
  return JSON.parse(json) as AiProviderConfig;
}

export async function saveAiProviderConfig(
  request: {
    primary: {provider: string; baseUrl: string; model: string; authToken: string};
    backup: {provider: string; baseUrl: string; model: string; authToken: string};
  },
): Promise<AiProviderConfig> {
  const json = await ensureModule().saveAiProviderConfig(JSON.stringify(request));
  return JSON.parse(json) as AiProviderConfig;
}

export async function getActivePlan(): Promise<PlanSummary | null> {
  const json = await ensureModule().getActivePlan();
  return JSON.parse(json) as PlanSummary | null;
}

export async function savePlan(
  planId: number,
  input: PlanEditInput,
): Promise<PlanSummary> {
  const json = await ensureModule().savePlan(
    JSON.stringify({
      planId,
      input,
    }),
  );
  return JSON.parse(json) as PlanSummary;
}

export async function applySavedPlanToToday(): Promise<PlanSummary> {
  const json = await ensureModule().applySavedPlanToToday();
  return JSON.parse(json) as PlanSummary;
}

export async function getWordbooks(): Promise<WordbookSummary[]> {
  const json = await ensureModule().getWordbooks();
  return JSON.parse(json) as WordbookSummary[];
}

export async function toggleWordbook(
  wordbookId: number,
  isActive: boolean,
): Promise<void> {
  return ensureModule().toggleWordbook(wordbookId, isActive);
}

export async function startStudySession(
  request: StartSessionRequest,
): Promise<StartSessionResponse> {
  const json = await ensureModule().startStudySession(JSON.stringify(request));
  return JSON.parse(json) as StartSessionResponse;
}

export async function submitStudyAnswer(
  request: SubmitAnswerRequest,
): Promise<SubmitAnswerResponse> {
  const json = await ensureModule().submitStudyAnswer(JSON.stringify(request));
  return JSON.parse(json) as SubmitAnswerResponse;
}

export async function completeStudySession(
  sessionId: string,
): Promise<CompleteSessionResponse> {
  const json = await ensureModule().completeStudySession(sessionId);
  return JSON.parse(json) as CompleteSessionResponse;
}

export async function cancelStudySession(sessionId: string): Promise<void> {
  return ensureModule().cancelStudySession(sessionId);
}

export async function getReportsOverview(): Promise<ReportsOverview> {
  const json = await ensureModule().getReportsOverview();
  return JSON.parse(json) as ReportsOverview;
}

export async function getWrongWords(filter: string): Promise<WrongWordEntry[]> {
  const json = await ensureModule().getWrongWords(filter);
  return JSON.parse(json) as WrongWordEntry[];
}

export async function getWrongWordDetail(entryId: number): Promise<WrongWordDetail> {
  const json = await ensureModule().getWrongWordDetail(entryId);
  return JSON.parse(json) as WrongWordDetail;
}

export async function getTodayAiPassageContext(): Promise<TodayAiPassageContext> {
  const json = await ensureModule().getTodayAiPassageContext();
  return JSON.parse(json) as TodayAiPassageContext;
}

export async function generateAiPassage(request: {
  targetWords: string[];
  level: string;
}): Promise<AIPassage> {
  const json = await ensureModule().generateAiPassage(JSON.stringify(request));
  return JSON.parse(json) as AIPassage;
}

export async function getAiPassageHistory(): Promise<AIPassageHistoryItem[]> {
  const json = await ensureModule().getAiPassageHistory();
  return JSON.parse(json) as AIPassageHistoryItem[];
}

export async function getAiPassage(passageId: string): Promise<AIPassage | null> {
  const json = await ensureModule().getAiPassage(passageId);
  return JSON.parse(json) as AIPassage | null;
}

export async function saveAiPassage(passage: AIPassage): Promise<void> {
  return ensureModule().saveAiPassage(JSON.stringify(passage));
}
