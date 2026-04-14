/**
 * Word Mobile Cross-Platform Contracts
 * @version 0.1.0
 *
 * This file contains TypeScript type definitions that mirror the Rust DTOs.
 * These types are manually maintained to match the Rust contract.
 *
 * @see docs/CONTRACT.md for full contract documentation
 */

// ============================================================================
// Common Enums
// ============================================================================

/** Outcome of grading a single answer */
export type AnswerOutcome = "correct" | "fuzzyCorrect" | "incorrect" | "skipped";

/** Study session mode */
export type SessionMode = "newWord" | "review" | "mixedTest" | "wrongWordReinforcement";

/** Type of study question */
export type QuestionType = "enToCnChoice" | "exampleToCnChoice" | "cnToEnChoice" | "enToCnInput";

// ============================================================================
// Bootstrap API
// ============================================================================

/** Bootstrap state returned on app start */
export interface BootstrapState {
  appReady: boolean;
  firstRunRequired: boolean;
  databaseStatus: "ready" | "init_failed";
  snapshotStatus: "ready" | "missing_required" | "empty";
  connectivityStatus: "offline" | "online";
  aiConfigStatus: "configured" | "missing";
  settingsEntryAvailable: boolean;
  blockingReason: string | null;
}

// ============================================================================
// Today Home API
// ============================================================================

/** Full state for the today home dashboard */
export interface TodayHomeState {
  todayDate: string;
  activePlan: PlanSummary | null;
  todaySnapshot: DailySnapshot | null;
  wordbooks: WordbookSummary[];
  dailyProgress: DailyProgress;
}

/** Summary of a plan template */
export interface PlanSummary {
  id: number;
  name: string;
  newWordsPerDay: number;
  reviewWordsPerDay: number;
  mixedTestPerDay: number;
  wrongWordTestPerDay: number;
  growthIntervalDays: number;
  growthIncrement: number;
}

/** Daily snapshot for a plan */
export interface DailySnapshot {
  date: string;
  newWordsTarget: number;
  newWordsCompleted: number;
  reviewWordsTarget: number;
  reviewWordsCompleted: number;
  mixedTestTarget: number;
  mixedTestCompleted: number;
  wrongWordTestTarget: number;
  wrongWordTestCompleted: number;
}

/** Summary of a wordbook */
export interface WordbookSummary {
  id: number;
  code: string;
  name: string;
  category: string;
  totalEntries: number;
  isActive: boolean;
}

/** Daily progress summary */
export interface DailyProgress {
  totalTasks: number;
  completedTasks: number;
  nextRecommendedAction: string;
}

// ============================================================================
// Study Session API
// ============================================================================

/** Request to start a study session */
export interface StartSessionRequest {
  mode: SessionMode;
  wordbookId: number | null;
  entrySourceIds: string[];
}

/** Response from starting a study session */
export interface StartSessionResponse {
  session: StudySession;
  currentQuestion: StudyQuestion;
  progress: SessionProgress;
}

/** Study session metadata */
export interface StudySession {
  sessionId: string;
  mode: SessionMode;
  totalWords: number;
  wordbookId: number | null;
  startedAt: string;
}

/** A single study question */
export interface StudyQuestion {
  questionId: string;
  questionType: QuestionType;
  entrySourceId: string;
  word: string;
  prompt: string;
  acceptedMeanings: string[];
  exampleSentence: string | null;
  exampleTranslation: string | null;
  choices: ChoiceOption[] | null;
  correctChoiceLabel: string | null;
  questionIndex: number;
  totalQuestions: number;
}

/** A choice option in a multiple-choice question */
export interface ChoiceOption {
  text: string;
  label: string;
}

/** Session progress indicator */
export interface SessionProgress {
  current: number;
  total: number;
}

/** Request to submit an answer */
export interface SubmitAnswerRequest {
  questionId: string;
  response: string;
  responseTimeMs: number;
}

/** Response from submitting an answer */
export interface SubmitAnswerResponse {
  result: StudyResult;
  isComplete: boolean;
  currentQuestion: StudyQuestion | null;
  summary: SessionSummary | null;
  nextAction: string | null;
  progress: SessionProgress;
}

/** Result of evaluating a single answer */
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

/** Summary of a completed session */
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

/** Response from completing a session */
export interface CompleteSessionResponse {
  summary: SessionSummary;
  nextAction: string;
}

// ============================================================================
// Reports API
// ============================================================================

/** Full reports state */
export interface ReportsState {
  dailyStats: DailyStat[];
  weeklyStats: WeeklyStat[];
  monthlyStats: MonthlyStat[];
  streakInfo: StreakInfo;
}

/** Daily statistics */
export interface DailyStat {
  date: string;
  totalQuestions: number;
  correctCount: number;
  accuracyPercent: number;
  studyTimeMs: number;
}

/** Weekly statistics */
export interface WeeklyStat {
  weekStart: string;
  totalQuestions: number;
  studyDays: number;
}

/** Monthly statistics */
export interface MonthlyStat {
  month: string;
  totalQuestions: number;
  studyDays: number;
}

/** Streak information */
export interface StreakInfo {
  currentStreak: number;
  longestStreak: number;
  lastStudyDate: string | null;
}

// ============================================================================
// Wrong Words API
// ============================================================================

/** Full wrong words state */
export interface WrongWordsState {
  totalWrongWords: number;
  activeWrongWords: number;
  words: WrongWordEntry[];
}

/** Single wrong word entry */
export interface WrongWordEntry {
  entryId: number;
  word: string;
  errorCount: number;
  lastWrongAt: string;
  priorityScore: number;
}

// ============================================================================
// Vocabulary API
// ============================================================================

/** Standardized vocabulary entry */
export interface StandardizedEntry {
  sourceId: string;
  word: string;
  lemma: string;
  phoneticUs: string | null;
  phoneticUk: string | null;
  partOfSpeech: string | null;
  meaningsZh: MeaningZh[];
  examples: EntryExample[];
  tags: string[];
  difficulty: string | null;
  frequency: number;
}

/** Chinese meaning */
export interface MeaningZh {
  pos: string;
  meaningCn: string;
  meaningEn: string | null;
}

/** Example sentence */
export interface EntryExample {
  sentenceEn: string;
  sentenceCn: string;
}

/** Wordbook */
export interface Wordbook {
  id: number | null;
  code: string;
  name: string;
  category: string;
  description: string | null;
  sourceBookId: string;
  sourceVersionId: number | null;
  totalEntries: number;
  isActive: boolean;
}

// ============================================================================
// Settings API
// ============================================================================

/** Settings summary */
export interface SettingsSummary {
  aiConfigured: boolean;
  syncConfigured: boolean;
  appVersion: string;
  schemaVersion: number;
  databasePath: string;
}

/** Single setting entry */
export interface SettingEntry {
  key: string;
  valueJson: string;
  updatedAt: string;
}
