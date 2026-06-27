import {
  AuthSessionResponse,
  AuthSessionState,
  EmailBindStartResponse,
  LeaderboardMetric,
  LeaderboardSummary,
  PlanSummary,
  ReportsOverview,
  AcceptDisputedMeaningResponse,
  CompleteSessionResponse,
  TodayHomeState,
  TodayRewardState,
  MarkStudyEntryMasteredResponse,
  PreserveStudyProgressResponse,
  ResumeSessionHint,
  StartSessionResponse,
  StudyMode,
  StudyStartOptions,
  SubmitAnswerResponse,
  CrocBtiProfile,
  WordbookSummary,
  WrongWordDetail,
  WrongWordEntry,
} from './types';
import { apiRequest, ApiTransportConfig } from './transport';
import Taro from '@tarojs/taro';

import {
  clearActiveStudySession,
  clearSessionTokens,
  getActiveStudySession,
  getAccessToken,
  getRefreshToken,
  saveActiveStudySession,
  saveSessionTokens,
} from './sessionStore';
import { setActiveStorageNamespaceFromAuth } from './storageNamespace';

function mergeActiveSessionIntoToday(today: TodayHomeState): TodayHomeState {
  const stored = getActiveStudySession();
  if (
    !stored?.session ||
    stored.session.answeredQuestions.length >= (stored.questions?.length ?? stored.session.progress.total)
  ) {
    return today;
  }
  const mode = stored.session.session.mode;
  const total = stored.questions?.length || stored.session.progress.total || stored.session.session.totalWords;
  if (total <= 0) return today;
  const completed = Math.min(stored.session.answeredQuestions.length, total);
  const existing = today.modeProgress?.[mode];
  const nextQuestion = (stored.questions ?? stored.session.questions ?? [])[completed] ?? stored.session.currentQuestion;
  return {
    ...today,
    modeProgress: {
      ...(today.modeProgress ?? {}),
      [mode]: {
        completed: Math.max(Number(existing?.completed ?? 0), completed),
        total: Math.max(Number(existing?.total ?? 0), total),
        isComplete: completed >= total,
      },
    },
    resumeHint: {
      hasResume: true,
      sessionId: stored.session.session.sessionId,
      mode,
      current: Math.min(completed + 1, total),
      total,
      word: nextQuestion?.word,
    },
  };
}

export class HttpMiniProgramSdk {
  private config: ApiTransportConfig;
  private loginPromise?: Promise<AuthSessionResponse>;
  private loginBlockedUntil = 0;
  private activeStudySession?: StartSessionResponse;

  constructor(baseUrl: string) {
    this.config = {
      baseUrl,
      getToken: getAccessToken,
      ensureSession: async () => {
        if (getAccessToken()) return;
        await this.loginWithWechat();
      },
      refreshSession: async () => {
        const refreshToken = getRefreshToken();
        if (!refreshToken) throw new Error('No refresh token available');
        const response = await apiRequest<AuthSessionResponse>(
          { baseUrl },
          'refresh',
          { refreshToken },
        );
        saveSessionTokens(response.session);
      },
    };
  }

  bootstrapWechatLogin() {
    return this.loginWithWechat();
  }

  private async getWechatLoginCode() {
    const result = await Taro.login();
    if (!result.code) {
      throw new Error('WeChat login did not return a code');
    }
    return result.code;
  }

  private async loginWithWechat() {
    const now = Date.now();
    if (now < this.loginBlockedUntil) {
      throw new Error('WeChat login is temporarily paused after a backend failure');
    }
    if (this.loginPromise) return this.loginPromise;
    this.loginPromise = this.getWechatLoginCode()
      .then((code) => this.auth.loginWithWechatCode(code))
      .catch((error) => {
        this.loginBlockedUntil = Date.now() + 5000;
        throw error;
      })
      .finally(() => {
        this.loginPromise = undefined;
      });
    return this.loginPromise;
  }

  readonly auth = {
    loginWithWechatCode: async (code: string): Promise<AuthSessionResponse> => {
      const response = await apiRequest<AuthSessionResponse>(
        this.config,
        'wechatLogin',
        {
          code,
          appVersion: '0.1.0',
          client: { platform: 'wechat_mp' },
        },
      );
      saveSessionTokens(response.session);
      setActiveStorageNamespaceFromAuth(response);
      Taro.eventCenter.trigger('auth:session-updated', response);
      return response;
    },
    refreshSession: async (): Promise<AuthSessionResponse> => {
      const response = await apiRequest<AuthSessionResponse>(
        { baseUrl: this.config.baseUrl },
        'refresh',
        { refreshToken: getRefreshToken() },
      );
      saveSessionTokens(response.session);
      setActiveStorageNamespaceFromAuth(response);
      Taro.eventCenter.trigger('auth:session-updated', response);
      return response;
    },
    logout: async (): Promise<void> => {
      await apiRequest<{ ok: boolean }>(this.config, 'logout', {});
      clearSessionTokens();
    },
    getMe: async (): Promise<AuthSessionState> =>
      apiRequest<AuthSessionState>(this.config, 'me'),
    startEmailBind: async (email: string): Promise<EmailBindStartResponse> =>
      apiRequest<EmailBindStartResponse>(this.config, 'startEmailBind', { email }),
  };

  readonly today = {
    getTodayHomeState: async (): Promise<TodayHomeState> =>
      mergeActiveSessionIntoToday(await apiRequest<TodayHomeState>(this.config, 'today')),
    getHomeState: async (): Promise<TodayHomeState> =>
      mergeActiveSessionIntoToday(await apiRequest<TodayHomeState>(this.config, 'today')),
  };

  readonly plan = {
    getActivePlan: async (): Promise<PlanSummary> =>
      apiRequest<PlanSummary>(this.config, 'activePlan'),
    getWordbooks: async (): Promise<WordbookSummary[]> =>
      apiRequest<WordbookSummary[]>(this.config, 'wordbooks'),
    toggleWordbook: async ({
      wordbookId,
      isActive,
    }: {
      wordbookId: number;
      isActive: boolean;
    }): Promise<void> => {
      await apiRequest<{ ok: boolean }>(
        this.config,
        'toggleWordbook',
        { isActive },
        { wordbookId },
      );
    },
    savePlan: async ({
      planId,
      input,
    }: {
      planId: number;
      input: Record<string, unknown>;
    }): Promise<PlanSummary> =>
      apiRequest<PlanSummary>(this.config, 'savePlan', input, { planId }),
    applySavedPlanToToday: async (): Promise<PlanSummary> =>
      apiRequest<PlanSummary>(this.config, 'applyPlan', {}),
  };

  readonly study = {
    startSession: async (
      input: StudyMode | StudyStartOptions,
      legacyOptions: Omit<StudyStartOptions, 'mode'> = {},
    ): Promise<StartSessionResponse> => {
      const options =
        typeof input === 'string' ? { ...legacyOptions, mode: input } : input;
      const stored = getActiveStudySession();
      if (
        stored?.session &&
        stored.session.session.mode === options.mode &&
        stored.session.answeredQuestions.length < (stored.questions?.length ?? stored.session.progress.total)
      ) {
        this.activeStudySession = {
          ...stored.session,
          questions: stored.questions,
        };
        return this.activeStudySession;
      }
      const session = await apiRequest<StartSessionResponse>(this.config, 'startSession', {
        mode: options.mode,
        entrySourceIds: options.entrySourceIds ?? [],
        wordbookId: options.wordbookId,
        entryPayloads: options.entryPayloads,
        distractorPayloads: options.distractorPayloads,
        questionTypeWeights: options.questionTypeWeights,
      });
      this.activeStudySession = session;
      saveActiveStudySession({
        session,
        questions: session.questions ?? [session.currentQuestion],
        savedAt: new Date().toISOString(),
      });
      return session;
    },
    getResumeSessionHint: async (): Promise<ResumeSessionHint> => {
      const stored = getActiveStudySession();
      if (
        stored?.session &&
        stored.session.answeredQuestions.length < (stored.questions?.length ?? stored.session.progress.total)
      ) {
        const questions = stored.questions ?? stored.session.questions ?? [];
        const nextQuestion = questions[stored.session.answeredQuestions.length];
        return {
          hasResume: true,
          sessionId: stored.session.session.sessionId,
          mode: stored.session.session.mode,
          current: Math.min(stored.session.answeredQuestions.length + 1, questions.length || stored.session.progress.total),
          total: questions.length || stored.session.progress.total,
          word: nextQuestion?.word,
        };
      }
      return apiRequest<ResumeSessionHint>(this.config, 'resumeSessionHint');
    },
    preserveProgress: async (): Promise<PreserveStudyProgressResponse> => {
      const [resumeHint, cloudToday] = await Promise.all([
        this.study.getResumeSessionHint(),
        apiRequest<TodayHomeState>(this.config, 'today'),
      ]);
      const today = mergeActiveSessionIntoToday(cloudToday);
      return { resumeHint, today };
    },
    submitAnswer: async (
      questionId: string,
      response: string,
      responseTimeMs: number,
    ): Promise<SubmitAnswerResponse> => {
      const answer = await apiRequest<SubmitAnswerResponse>(this.config, 'submitAnswer', {
        questionId,
        response,
        responseTimeMs,
        sessionSnapshot: this.activeStudySession,
      });
      if (this.activeStudySession) {
        this.activeStudySession = {
          ...this.activeStudySession,
          currentQuestion: answer.currentQuestion ?? this.activeStudySession.currentQuestion,
          progress: answer.progress,
          answeredQuestions: answer.answeredQuestions,
          questions: this.activeStudySession.questions,
        };
        if (answer.isComplete) {
          this.activeStudySession = undefined;
          clearActiveStudySession();
        } else {
          saveActiveStudySession({
            session: this.activeStudySession,
            questions: this.activeStudySession.questions ?? [this.activeStudySession.currentQuestion],
            savedAt: new Date().toISOString(),
          });
        }
      }
      return answer;
    },
    markEntryMastered: async (
      entrySourceId: string,
      reason = 'mastered',
    ): Promise<MarkStudyEntryMasteredResponse> =>
      apiRequest<MarkStudyEntryMasteredResponse>(
        this.config,
        'markStudyEntryMastered',
        { entrySourceId, reason },
      ),
    acceptDisputedMeaning: async (
      questionId: string,
      submittedAnswer: string,
    ): Promise<AcceptDisputedMeaningResponse> =>
      apiRequest<AcceptDisputedMeaningResponse>(
        this.config,
        'acceptDisputedMeaning',
        { questionId, submittedAnswer },
      ),
    completeSession: async (sessionId: string): Promise<CompleteSessionResponse> => {
      const completion = await apiRequest<CompleteSessionResponse>(
        this.config,
        'completeSession',
        {},
        { sessionId },
      );
      this.activeStudySession = undefined;
      clearActiveStudySession();
      return completion;
    },
    cancelSession: async (sessionId: string): Promise<void> => {
      await apiRequest<{ ok: boolean }>(
        this.config,
        'cancelSession',
        {},
        { sessionId },
      );
      this.activeStudySession = undefined;
      clearActiveStudySession();
    },
  };

  readonly wrongWords = {
    list: async (): Promise<WrongWordEntry[]> =>
      apiRequest<WrongWordEntry[]>(this.config, 'wrongWords'),
    getDetail: async (entryId?: number): Promise<WrongWordDetail> =>
      apiRequest<WrongWordDetail>(
        this.config,
        'wrongWordDetail',
        undefined,
        { entryId: entryId ?? 0 },
      ),
  };

  readonly reports = {
    getOverview: async (): Promise<ReportsOverview> => {
      const overview = await apiRequest<ReportsOverview>(this.config, 'reportsOverview');
      return { ...overview, modeSeries: overview.modeSeries ?? {} };
    },
  };

  readonly crocBti = {
    getProfile: async (): Promise<CrocBtiProfile | null> =>
      apiRequest<CrocBtiProfile | null>(this.config, 'crocBtiProfile'),
    saveProfile: async ({
      profile,
    }: {
      profile: CrocBtiProfile;
    }): Promise<void> => {
      await apiRequest<{ ok: boolean }>(this.config, 'saveCrocBtiProfile', profile);
    },
  };

  readonly sync = {
    flushPendingToCloud: async (): Promise<void> => {},
  };

  readonly rewards = {
    getTodayRewardState: async (): Promise<TodayRewardState> =>
      apiRequest<TodayRewardState>(this.config, 'todayReward'),
    claimTodayReward: async (): Promise<TodayRewardState> =>
      apiRequest<TodayRewardState>(this.config, 'claimReward', {}),
  };

  readonly leaderboard = {
    getSummary: async (
      metric: LeaderboardMetric = 'weekly',
    ): Promise<LeaderboardSummary> =>
      apiRequest<LeaderboardSummary>(this.config, 'leaderboard', undefined, {
        metric,
      }),
  };
}
