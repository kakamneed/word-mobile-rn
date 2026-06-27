import { request } from '@tarojs/taro';

import {
  LeaderboardMetric,
  LeaderboardSummary,
  PlanSummary,
  ReportsOverview,
  StartSessionResponse,
  StudyMode,
  SubmitAnswerResponse,
  TodayHomeState,
  TodayRewardState,
  WrongWordDetail,
  WrongWordEntry,
} from './types';
import { apiRoutes, ApiRouteKey, applyParams, publicRouteKeys } from './apiRoutes';

export { apiRoutes, applyParams };

export interface ApiErrorPayload {
  code: string;
  message: string;
  requestId?: string;
  retryable?: boolean;
  routeKey?: ApiRouteKey;
  path?: string;
}

export class ApiError extends Error {
  readonly statusCode: number;
  readonly payload: ApiErrorPayload;

  constructor(statusCode: number, payload: ApiErrorPayload) {
    super(payload.message);
    this.name = 'ApiError';
    this.statusCode = statusCode;
    this.payload = payload;
  }
}

export interface ApiTransportConfig {
  baseUrl: string;
  getToken?: () => string | undefined | Promise<string | undefined>;
  ensureSession?: () => Promise<void>;
  refreshSession?: () => Promise<void>;
}

export async function apiRequest<TResponse>(
  config: ApiTransportConfig,
  routeKey: ApiRouteKey,
  data?: unknown,
  params?: Record<string, string | number>,
  options: { hasRetried?: boolean } = {},
): Promise<TResponse> {
  const route = apiRoutes[routeKey];
  if (route.authRequired && !publicRouteKeys.has(routeKey)) {
    await config.ensureSession?.();
  }
  const token = await config.getToken?.();
  const path = applyParams(route.path, params);
  const response = await request<TResponse | ApiErrorPayload>({
    url: `${config.baseUrl}${path}`,
    method: route.method,
    data,
    header: token ? { Authorization: `Bearer ${token}` } : undefined,
    timeout: 12000,
  });

  if (
    response.statusCode === 401 &&
    route.authRequired &&
    !publicRouteKeys.has(routeKey) &&
    !options.hasRetried &&
    config.refreshSession
  ) {
    await config.refreshSession();
    return apiRequest<TResponse>(config, routeKey, data, params, { hasRetried: true });
  }

  if (response.statusCode >= 400) {
    const payload = response.data as ApiErrorPayload;
    throw new ApiError(response.statusCode, {
      code: payload.code ?? 'request_failed',
      message: payload.message ?? `${routeKey} failed with ${response.statusCode}`,
      requestId: payload.requestId,
      retryable: payload.retryable,
      routeKey,
      path,
    });
  }

  return response.data as TResponse;
}

export interface BackendStudyApiContract {
  getTodayHomeState(): Promise<TodayHomeState>;
  getActivePlan(): Promise<PlanSummary>;
  startSession(mode: StudyMode): Promise<StartSessionResponse>;
  submitAnswer(
    questionId: string,
    response: string,
    responseTimeMs: number,
  ): Promise<SubmitAnswerResponse>;
  listWrongWords(): Promise<WrongWordEntry[]>;
  getWrongWordDetail(entryId?: number): Promise<WrongWordDetail>;
  getReportsOverview(): Promise<ReportsOverview>;
  getTodayRewardState(): Promise<TodayRewardState>;
  claimTodayReward(): Promise<TodayRewardState>;
  getLeaderboard(metric: LeaderboardMetric): Promise<LeaderboardSummary>;
}
