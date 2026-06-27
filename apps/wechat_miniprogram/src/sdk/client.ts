import {
  initialReports,
  initialReward,
  initialSession,
  initialToday,
  initialWrongWordDetail,
  initialWrongWords,
  leaderboardFromReports,
  mockAuthState,
  mockPlan,
  mockWordbooks,
  questionBank,
} from './mockData';
import {
  correctChoiceText,
  correctResponseForQuestion,
  normalizeChoiceToken,
  sanitizeChoices,
} from './studyChoice';
import {
  clearActiveStudySession,
  getActiveStudySession,
  saveActiveStudySession,
} from './sessionStore';
import { saveStoredRewardState, saveStoredTodayState } from './todayStore';
import {
  AuthSessionState,
  AcceptDisputedMeaningResponse,
  CompleteSessionResponse,
  CrocBtiProfile,
  LeaderboardMetric,
  LeaderboardSummary,
  MarkStudyEntryMasteredResponse,
  PlanSummary,
  PreserveStudyProgressResponse,
  ReportsOverview,
  ResumeSessionHint,
  StartSessionResponse,
  StudyMode,
  StudyQuestion,
  StudyResult,
  StudyStartOptions,
  SubmitAnswerResponse,
  TodayHomeState,
  TodayRewardState,
  WordbookSummary,
  WrongWordDetail,
  WrongWordEntry,
} from './types';

const wait = async () => {};
const mockStrictFixtureMode = true;

class MockApiError extends Error {
  readonly statusCode: number;
  readonly payload: { code: string; message: string };

  constructor(statusCode: number, payload: { code: string; message: string }) {
    super(payload.message);
    this.name = 'ApiError';
    this.statusCode = statusCode;
    this.payload = payload;
  }
}

interface MockState {
  today: TodayHomeState;
  reports: ReportsOverview;
  wrongWords: WrongWordEntry[];
  reward: TodayRewardState;
  activeSession: StartSessionResponse;
  activeSessionQuestions: StudyQuestion[];
  activePlan: PlanSummary;
  wordbooks: WordbookSummary[];
  crocBtiProfile: CrocBtiProfile | null;
}

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

function firstQuestionForMode(mode: StudyMode): StudyQuestion {
  return clone(questionBank[mode][0]);
}

function targetQuestionCountForPlan(plan: PlanSummary, mode: StudyMode): number {
  const counts: Record<StudyMode, number | undefined> = {
    newWord: plan.newWordsPerDay,
    review: plan.reviewWordsPerDay,
    mixedTest: plan.mixedTestPerDay,
    wrongWordReinforcement: plan.wrongWordTestPerDay,
    rootAffix: plan.rootAffixPerDay,
  };
  return Math.max(1, Math.round(counts[mode] ?? questionBank[mode].length));
}

function modeCountKey(mode: StudyMode): keyof PlanSummary {
  const keys: Record<StudyMode, keyof PlanSummary> = {
    newWord: 'newWordsPerDay',
    review: 'reviewWordsPerDay',
    mixedTest: 'mixedTestPerDay',
    wrongWordReinforcement: 'wrongWordTestPerDay',
    rootAffix: 'rootAffixPerDay',
  };
  return keys[mode];
}

function buildFixtureQuestionsForSession(mode: StudyMode, total: number): StudyQuestion[] {
  const bank = questionBank[mode].map((question) => ({
    ...question,
    choices: sanitizeChoices(question.choices),
  }));
  const requestedTotal = mockStrictFixtureMode ? total : Math.min(total, bank.length);
  const selected = bank.slice(0, requestedTotal);
  const sessionTotal = selected.length || total;
  const seenQuestionIds = new Set<string>();
  let previousWord = '';
  const questions = selected.map((base, index) => {
    if (seenQuestionIds.has(base.questionId)) {
      throw new Error(`Fixture questionId repeated: ${base.questionId}`);
    }
    if (previousWord && previousWord === base.word) {
      throw new Error(`Fixture word repeated consecutively: ${base.word}`);
    }
    seenQuestionIds.add(base.questionId);
    previousWord = base.word;
    return {
      ...clone(base),
      questionId: base.questionId,
      questionIndex: index + 1,
      totalQuestions: sessionTotal,
    };
  });
  if (questions.length !== requestedTotal) {
    throw new Error(`Fixture cannot provide ${requestedTotal} ${mode} questions without repetition`);
  }
  return questions;
}

function evaluateFixtureAnswer(
  question: StudyQuestion,
  response: string,
  skipped: boolean,
  responseTimeMs: number,
): StudyResult {
  const normalized = normalizeChoiceToken(response);
  let correct = false;
  if (question.choices?.length) {
    const correctChoice = correctResponseForQuestion(question);
    const correctTokens = new Set<string>();
    const choices = sanitizeChoices(question.choices);
    choices.forEach((choice) => {
      const labelToken = normalizeChoiceToken(choice.label);
      const valueToken = normalizeChoiceToken(choice.value);
      if (
        labelToken === normalizeChoiceToken(question.correctChoiceLabel) ||
        valueToken === normalizeChoiceToken(question.correctChoiceLabel) ||
        valueToken === normalizeChoiceToken(correctChoice)
      ) {
        correctTokens.add(labelToken);
        correctTokens.add(valueToken);
      }
    });
    correct = !skipped && correctTokens.has(normalized);
  } else {
    correct =
      !skipped &&
      question.acceptedMeanings.some((meaning) => normalizeChoiceToken(meaning) === normalized);
  }
  return {
    questionId: question.questionId,
    entrySourceId: question.entrySourceId,
    questionType: question.questionType,
    userResponse: response,
    normalizedResponse: response.trim(),
    correctAnswer: question.choices?.length ? correctChoiceText(question) : question.acceptedMeanings[0],
    outcome: skipped ? 'skipped' : correct ? 'correct' : 'incorrect',
    responseTimeMs,
    answeredAt: new Date().toISOString(),
  };
}

function summarizeSession(
  session: StartSessionResponse,
  completedAt: string,
): NonNullable<SubmitAnswerResponse['summary']> {
  const answeredQuestions = session.answeredQuestions;
  const correctCount = answeredQuestions.filter(
    (item) => item.result.outcome === 'correct' || item.result.outcome === 'fuzzyCorrect',
  ).length;
  const skippedCount = answeredQuestions.filter((item) => item.result.outcome === 'skipped').length;
  const incorrectCount = answeredQuestions.length - correctCount - skippedCount;
  return {
    sessionId: session.session.sessionId,
    totalQuestions: answeredQuestions.length,
    correctCount,
    incorrectCount,
    skippedCount,
    totalWords: session.session.totalWords,
    wrongWordCount: incorrectCount + skippedCount,
    accuracyPercent: answeredQuestions.length
      ? Math.round((correctCount / answeredQuestions.length) * 100)
      : 0,
    totalTimeMs: answeredQuestions.reduce((sum, item) => sum + item.result.responseTimeMs, 0),
    completedAt,
  };
}

function modeBreakdownIndex(reports: ReportsOverview, mode: StudyMode): number {
  return reports.modeBreakdown.findIndex((row) => row.mode === mode);
}

export class MiniProgramSdk {
  private completedModes = new Set<StudyMode>();

  private state: MockState = {
    today: clone(initialToday),
    reports: clone(initialReports),
    wrongWords: clone(initialWrongWords),
    reward: clone(initialReward),
    activeSession: clone(initialSession),
    activeSessionQuestions: [clone(initialSession.currentQuestion)],
    activePlan: clone(mockPlan),
    wordbooks: clone(mockWordbooks),
    crocBtiProfile: null,
  };

  readonly auth = {
    getMe: async (): Promise<AuthSessionState> => {
      await wait();
      return clone(mockAuthState);
    },
    startEmailBind: async (email: string) => {
      await wait();
      return {
        email,
        requiresConfirmation: false,
        message: 'Mock 模式不会绑定邮箱。',
      };
    },
  };

  readonly today = {
    getTodayHomeState: async (): Promise<TodayHomeState> => {
      await wait();
      saveStoredTodayState(this.state.today);
      return clone(this.state.today);
    },
    getHomeState: async (): Promise<TodayHomeState> => {
      await wait();
      saveStoredTodayState(this.state.today);
      return clone(this.state.today);
    },
  };

  readonly plan = {
    getActivePlan: async (): Promise<PlanSummary> => {
      await wait();
      return clone(this.state.activePlan);
    },
    getWordbooks: async (): Promise<WordbookSummary[]> => {
      await wait();
      return clone(this.state.wordbooks);
    },
    toggleWordbook: async ({
      wordbookId,
      isActive,
    }: {
      wordbookId: number;
      isActive: boolean;
    }): Promise<void> => {
      await wait();
      this.state.wordbooks = this.state.wordbooks.map((wordbook) => ({
        ...wordbook,
        isActive: isActive ? wordbook.id === wordbookId : wordbook.isActive,
      }));
      this.state.today = { ...this.state.today, wordbooks: clone(this.state.wordbooks) };
      saveStoredTodayState(this.state.today);
    },
    savePlan: async ({
      planId,
      input,
    }: {
      planId: number;
      input: Record<string, unknown>;
    }): Promise<PlanSummary> => {
      await wait();
      this.state.activePlan = {
        ...this.state.activePlan,
        id: planId,
        ...(input as Partial<PlanSummary>),
      };
      this.state.today = { ...this.state.today, activePlan: clone(this.state.activePlan) };
      saveStoredTodayState(this.state.today);
      return clone(this.state.activePlan);
    },
    applySavedPlanToToday: async (): Promise<PlanSummary> => {
      await wait();
      this.state.today = { ...this.state.today, activePlan: clone(this.state.activePlan) };
      saveStoredTodayState(this.state.today);
      return clone(this.state.activePlan);
    },
  };

  readonly study = {
    startSession: async (
      input: StudyMode | StudyStartOptions,
    ): Promise<StartSessionResponse> => {
      await wait();
      const mode = typeof input === 'string' ? input : input.mode;
      const resumeHint = this.state.today.resumeHint;
      if (
        resumeHint?.hasResume &&
        resumeHint.mode === mode &&
        resumeHint.sessionId === this.state.activeSession.session.sessionId
      ) {
        this.restorePersistedActiveSession();
        return clone(this.state.activeSession);
      }
      const totalQuestions = targetQuestionCountForPlan(this.state.activePlan, mode);
      const questions = buildFixtureQuestionsForSession(mode, totalQuestions);
      const currentQuestion = questions[0] ?? firstQuestionForMode(mode);
      const sessionTotal = questions.length || totalQuestions;
      const session: StartSessionResponse = {
        session: {
          sessionId: `session-${mode}-${Date.now()}`,
          mode,
          totalWords: sessionTotal,
          startedAt: new Date().toISOString(),
        },
        currentQuestion,
        progress: {
          current: currentQuestion.questionIndex,
          total: sessionTotal,
        },
        answeredQuestions: [],
      };
      this.state.activeSession = session;
      this.state.activeSessionQuestions = questions;
      this.persistActiveSession();
      return clone(session);
    },
    getResumeSessionHint: async (): Promise<ResumeSessionHint> => {
      await wait();
      return clone(this.state.today.resumeHint ?? { hasResume: false });
    },
    preserveProgress: async (): Promise<PreserveStudyProgressResponse> => {
      await wait();
      this.syncActiveSessionResumeHint(this.state.activeSession);
      return {
        resumeHint: clone(this.state.today.resumeHint ?? { hasResume: false }),
        today: clone(this.state.today),
      };
    },
    submitAnswer: async (
      questionId: string,
      response: string,
      responseTimeMs: number,
    ): Promise<SubmitAnswerResponse> => {
      await wait();
      const session = this.state.activeSession;
      const question = session.currentQuestion;
      if (questionId !== question.questionId) {
        throw new MockApiError(409, {
          code: 'study_question_mismatch',
          message: 'Submitted questionId does not match the current question',
        });
      }
      const normalizedResponse = response.trim();
      const skipped = normalizedResponse.length === 0;
      const result = evaluateFixtureAnswer(question, response, skipped, responseTimeMs);
      const answeredAt = result.answeredAt;

      const answeredQuestions = [
        ...session.answeredQuestions,
        { question: clone(question), result: clone(result) },
      ];
      const nextIndex = question.questionIndex;
      const nextQuestion = this.state.activeSessionQuestions[nextIndex];
      const nextSession: StartSessionResponse = {
        ...session,
        currentQuestion: nextQuestion ?? question,
        progress: {
          current: nextQuestion?.questionIndex ?? question.totalQuestions,
          total: question.totalQuestions,
        },
        answeredQuestions,
      };
      this.state.activeSession = nextSession;

      const isComplete = !nextQuestion;
      this.applyStudySideEffects(session.session.mode, question, result);
      const summary = isComplete ? summarizeSession(nextSession, answeredAt) : undefined;
      if (summary) {
        this.applyCompletedSessionToToday(summary, session.session.mode);
        clearActiveStudySession();
      } else {
        this.syncActiveSessionResumeHint(nextSession);
        this.persistActiveSession();
      }
      const responsePayload: SubmitAnswerResponse = {
        result,
        currentQuestion: nextQuestion ? clone(nextQuestion) : undefined,
        progress: clone(nextSession.progress),
        answeredQuestions: clone(answeredQuestions),
        isComplete,
        summary,
        nextAction: isComplete ? 'Return to today' : 'Continue',
      };
      return clone(responsePayload);
    },
    markEntryMastered: async (
      entrySourceId: string,
      reason = 'mastered',
    ): Promise<MarkStudyEntryMasteredResponse> => {
      await wait();
      const session = this.state.activeSession;
      const currentQuestion = session.currentQuestion;
      const remainingQuestions = this.state.activeSessionQuestions.filter(
        (question) =>
          question.entrySourceId !== entrySourceId &&
          question.questionIndex > currentQuestion.questionIndex,
      );
      const nextQuestion = remainingQuestions[0];
      const nextSession: StartSessionResponse = {
        ...session,
        currentQuestion: nextQuestion ?? currentQuestion,
        progress: {
          current: nextQuestion?.questionIndex ?? session.progress.total,
          total: session.progress.total,
        },
      };
      this.state.activeSession = nextSession;
      const completedAt = new Date().toISOString();
      if (!nextQuestion) {
        const summary = summarizeSession(nextSession, completedAt);
        this.applyCompletedSessionToToday(summary, session.session.mode);
        clearActiveStudySession();
      } else {
        this.syncActiveSessionResumeHint(nextSession);
        this.persistActiveSession();
      }
      return {
        entrySourceId,
        prunedQuestionCount:
          session.currentQuestion.entrySourceId === entrySourceId ? 1 : 0,
        isComplete: !nextQuestion,
        currentQuestion: nextQuestion ? clone(nextQuestion) : undefined,
        summary: nextQuestion ? undefined : summarizeSession(nextSession, completedAt),
        nextAction: reason,
        progress: clone(nextSession.progress),
        answeredQuestions: clone(nextSession.answeredQuestions),
      };
    },
    acceptDisputedMeaning: async (
      questionId: string,
      submittedAnswer: string,
    ): Promise<AcceptDisputedMeaningResponse> => {
      await wait();
      const question = this.state.activeSession.currentQuestion;
      const result: StudyResult = {
        questionId,
        entrySourceId: question.entrySourceId,
        questionType: question.questionType,
        userResponse: submittedAnswer,
        normalizedResponse: submittedAnswer.trim(),
        correctAnswer: submittedAnswer,
        outcome: 'correct',
        responseTimeMs: 0,
        answeredAt: new Date().toISOString(),
      };
      return {
        result,
        progress: clone(this.state.activeSession.progress),
        answeredQuestions: [
          ...clone(this.state.activeSession.answeredQuestions),
          { question: clone(question), result: clone(result) },
        ],
        entrySourceId: question.entrySourceId,
        word: question.word,
        acceptedMeaning: submittedAnswer,
      };
    },
    completeSession: async (): Promise<CompleteSessionResponse> => {
      await wait();
      const completedAt = new Date().toISOString();
      return {
        summary: summarizeSession(this.state.activeSession, completedAt),
        nextAction: 'Return to today',
      };
    },
    cancelSession: async (): Promise<void> => {
      await wait();
      this.state.today = {
        ...this.state.today,
        resumeHint: { hasResume: false },
      };
      saveStoredTodayState(this.state.today);
      clearActiveStudySession();
    },
  };

  readonly wrongWords = {
    list: async (): Promise<WrongWordEntry[]> => {
      await wait();
      return clone(this.state.wrongWords);
    },
    getDetail: async (entryId?: number): Promise<WrongWordDetail> => {
      await wait();
      const entry =
        this.state.wrongWords.find((word) => word.entryId === entryId) ??
        this.state.wrongWords[0];
      return {
        ...clone(initialWrongWordDetail),
        ...clone(entry),
      };
    },
  };

  readonly reports = {
    getOverview: async (): Promise<ReportsOverview> => {
      await wait();
      return clone({ ...this.state.reports, modeSeries: this.state.reports.modeSeries ?? {} });
    },
  };

  readonly crocBti = {
    getProfile: async (): Promise<CrocBtiProfile | null> => {
      await wait();
      return clone(this.state.crocBtiProfile);
    },
    saveProfile: async ({ profile }: { profile: CrocBtiProfile }): Promise<void> => {
      await wait();
      this.state.crocBtiProfile = clone(profile);
    },
  };

  readonly rewards = {
    getTodayRewardState: async (): Promise<TodayRewardState> => {
      await wait();
      saveStoredRewardState(this.state.reward);
      return clone(this.state.reward);
    },
    claimTodayReward: async (): Promise<TodayRewardState> => {
      await wait();
      if (!this.state.reward.claimedAt) {
        this.state.reward = {
          todayDate: this.state.today.todayDate,
          rewardId: 'reward-001-com-hihonor-photos-20260329190156-edit-752572937',
          claimedAt: new Date().toISOString(),
          asset: {
            rewardId: 'reward-001-com-hihonor-photos-20260329190156-edit-752572937',
            title: 'Reward 001',
            imageUrl: '/assets/rewards/webp_q60_540/reward_001.webp',
            mimeType: 'image/webp',
          },
        };
      }
      saveStoredRewardState(this.state.reward);
      return clone(this.state.reward);
    },
  };

  readonly leaderboard = {
    getSummary: async (
      metric: LeaderboardMetric = 'weekly',
    ): Promise<LeaderboardSummary> => {
      await wait();
      return clone(leaderboardFromReports(this.state.reports, metric));
    },
  };

  readonly sync = {
    flushPendingToCloud: async (): Promise<void> => {
      await wait();
    },
  };

  private applyStudySideEffects(
    mode: StudyMode,
    question: StudyQuestion,
    result: StudyResult,
  ) {
    const correct = result.outcome === 'correct' || result.outcome === 'fuzzyCorrect';
    const nextQuestions = this.state.reports.totalQuestionsAnswered + 1;
    const previousCorrect = Math.round(
      (this.state.reports.overallAccuracy / 100) *
        this.state.reports.totalQuestionsAnswered,
    );
    const nextCorrect = previousCorrect + (correct ? 1 : 0);

    this.state.reports = {
      ...this.state.reports,
      totalQuestionsAnswered: nextQuestions,
      overallAccuracy: Math.round((nextCorrect / nextQuestions) * 100),
      dailySeries: this.state.reports.dailySeries.map((row, index, rows) =>
        index === rows.length - 1
          ? {
              ...row,
              totalQuestions: (row.totalQuestions ?? row.questionsAnswered ?? 0) + 1,
              correctCount: (row.correctCount ?? 0) + (correct ? 1 : 0),
              accuracyPercent: Math.round(
                (((row.correctCount ?? 0) + (correct ? 1 : 0)) /
                  ((row.totalQuestions ?? row.questionsAnswered ?? 0) + 1)) *
                  100,
              ),
            }
          : row,
      ),
      modeBreakdown: this.nextModeBreakdown(mode, correct),
      modeSeries: this.nextModeSeries(mode, correct),
    };

    if (!correct) {
      this.bumpWrongWord(question, result.answeredAt);
    }
  }

  private applyCompletedSessionToToday(
    summary: NonNullable<SubmitAnswerResponse['summary']>,
    mode: StudyMode,
  ) {
    const progress = this.state.today.dailyProgress;
    const completedTasks = this.completedModes.has(mode)
      ? progress.completedTasks
      : Math.min(progress.totalTasks, progress.completedTasks + 1);
    this.completedModes.add(mode);
    this.state.today = {
      ...this.state.today,
      dailyProgress: {
        ...progress,
        completedTasks,
        accuracyPercent: summary.accuracyPercent,
      },
      modeProgress: {
        ...(this.state.today.modeProgress ?? {}),
        [mode]: {
          completed: summary.totalQuestions,
          total: summary.totalWords ?? summary.totalQuestions,
          isComplete: true,
        },
      },
      resumeHint: { hasResume: false },
    };
    saveStoredTodayState(this.state.today);
  }

  private syncActiveSessionResumeHint(session: StartSessionResponse) {
    const mode = session.session.mode;
    const answered = session.answeredQuestions.length;
    const total =
      session.progress.total ||
      Number(this.state.activePlan[modeCountKey(mode)] ?? session.session.totalWords);
    this.state.today = {
      ...this.state.today,
      modeProgress: {
        ...(this.state.today.modeProgress ?? {}),
        [mode]: {
          completed: answered,
          total,
          isComplete: false,
        },
      },
      resumeHint: {
        hasResume: true,
        sessionId: session.session.sessionId,
        mode,
        word: session.currentQuestion.word,
        current: session.progress.current,
        total,
      },
    };
    saveStoredTodayState(this.state.today);
  }

  private persistActiveSession() {
    saveActiveStudySession({
      session: clone(this.state.activeSession),
      questions: clone(this.state.activeSessionQuestions),
      savedAt: new Date().toISOString(),
    });
  }

  private restorePersistedActiveSession() {
    const snapshot = getActiveStudySession();
    if (
      !snapshot ||
      snapshot.session.session.sessionId !== this.state.today.resumeHint?.sessionId
    ) {
      return;
    }
    this.state.activeSession = clone(snapshot.session);
    this.state.activeSessionQuestions = clone(snapshot.questions);
  }

  private nextModeBreakdown(mode: StudyMode, correct: boolean) {
    const rows = [...this.state.reports.modeBreakdown];
    const index = modeBreakdownIndex(this.state.reports, mode);
    if (index < 0) {
      rows.push({
        mode,
        totalQuestions: 1,
        correctCount: correct ? 1 : 0,
        accuracyPercent: correct ? 100 : 0,
      });
      return rows;
    }
    const totalQuestions = (rows[index].totalQuestions ?? rows[index].questionsAnswered ?? 0) + 1;
    const correctCount = (rows[index].correctCount ?? 0) + (correct ? 1 : 0);
    rows[index] = {
      ...rows[index],
      totalQuestions,
      correctCount,
      accuracyPercent: Math.round((correctCount / totalQuestions) * 100),
    };
    return rows;
  }

  private nextModeSeries(mode: StudyMode, correct: boolean) {
    const series = { ...(this.state.reports.modeSeries ?? {}) };
    const todayDate = this.state.today.todayDate;
    const rows = [...(series[mode] ?? [])];
    const index = rows.findIndex((row) => row.date === todayDate);
    if (index < 0) {
      rows.push({
        date: todayDate,
        totalQuestions: 1,
        correctCount: correct ? 1 : 0,
        accuracyPercent: correct ? 100 : 0,
      });
    } else {
      const totalQuestions = (rows[index].totalQuestions ?? rows[index].questionsAnswered ?? 0) + 1;
      const correctCount = (rows[index].correctCount ?? 0) + (correct ? 1 : 0);
      rows[index] = {
        ...rows[index],
        totalQuestions,
        correctCount,
        accuracyPercent: Math.round((correctCount / totalQuestions) * 100),
      };
    }
    series[mode] = rows;
    return series;
  }

  private bumpWrongWord(question: StudyQuestion, answeredAt: string) {
    const index = this.state.wrongWords.findIndex(
      (word) => word.word === question.word,
    );
    if (index < 0) {
      this.state.wrongWords = [
        {
          entryId: Date.now(),
          word: question.word,
          meanings: question.acceptedMeanings,
          errorCount: 1,
          lastWrongAt: answeredAt,
          priorityScore: 65,
          hasHint: question.hasHint,
          userHint: question.userHint,
        },
        ...this.state.wrongWords,
      ];
      return;
    }
    const current = this.state.wrongWords[index];
    this.state.wrongWords[index] = {
      ...current,
      errorCount: current.errorCount + 1,
      lastWrongAt: answeredAt,
      priorityScore: Math.min(100, current.priorityScore + 5),
    };
  }
}

export const sdk = new MiniProgramSdk();
