import {
  AuthSessionResponse,
  AuthSessionState,
  LeaderboardMetric,
  LeaderboardSummary,
  PlanSummary,
  ReportsOverview,
  AcceptDisputedMeaningResponse,
  CompleteSessionResponse,
  TodayHomeState,
  TodayRewardState,
  MarkStudyEntryMasteredResponse,
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
import {
  clearSessionTokens,
  getAccessToken,
  getRefreshToken,
  saveSessionTokens,
} from './sessionStore';

export class HttpMiniProgramSdk {
  private config: ApiTransportConfig;

  constructor(baseUrl: string) {
    this.config = {
      baseUrl,
      getToken: getAccessToken,
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
      return response;
    },
    refreshSession: async (): Promise<AuthSessionResponse> => {
      const response = await apiRequest<AuthSessionResponse>(
        { baseUrl: this.config.baseUrl },
        'refresh',
        { refreshToken: getRefreshToken() },
      );
      saveSessionTokens(response.session);
      return response;
    },
    logout: async (): Promise<void> => {
      await apiRequest<{ ok: boolean }>(this.config, 'logout', {});
      clearSessionTokens();
    },
    getMe: async (): Promise<AuthSessionState> =>
      apiRequest<AuthSessionState>(this.config, 'me'),
  };

  readonly today = {
    getTodayHomeState: async (): Promise<TodayHomeState> =>
      apiRequest<TodayHomeState>(this.config, 'today'),
    getHomeState: async (): Promise<TodayHomeState> =>
      apiRequest<TodayHomeState>(this.config, 'today'),
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
      return (
      apiRequest<StartSessionResponse>(this.config, 'startSession', {
        mode: options.mode,
        entrySourceIds: options.entrySourceIds ?? [],
        wordbookId: options.wordbookId,
        entryPayloads: options.entryPayloads,
        distractorPayloads: options.distractorPayloads,
        questionTypeWeights: options.questionTypeWeights,
      })
      );
    },
    getResumeSessionHint: async (): Promise<ResumeSessionHint> =>
      apiRequest<ResumeSessionHint>(this.config, 'resumeSessionHint'),
    submitAnswer: async (
      questionId: string,
      response: string,
      responseTimeMs: number,
    ): Promise<SubmitAnswerResponse> =>
      apiRequest<SubmitAnswerResponse>(this.config, 'submitAnswer', {
        questionId,
        response,
        responseTimeMs,
      }),
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
    completeSession: async (sessionId: string): Promise<CompleteSessionResponse> =>
      apiRequest<CompleteSessionResponse>(
        this.config,
        'completeSession',
        {},
        { sessionId },
      ),
    cancelSession: async (sessionId: string): Promise<void> => {
      await apiRequest<{ ok: boolean }>(
        this.config,
        'cancelSession',
        {},
        { sessionId },
      );
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
