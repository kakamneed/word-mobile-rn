import { requireAuth } from './context.js';
import { readJsonBody, sendError, sendJson } from './json.js';

export function createRouter({ authService, adapter, studyDomain }) {
  return async function handle(request, response) {
    try {
      const url = new URL(request.url, 'http://localhost');
      const method = request.method;

      if (method === 'POST' && url.pathname === '/v1/auth/wechat-mp/login') {
        const body = await readJsonBody(request);
        return sendJson(response, 200, await authService.loginWithWechatCode(body));
      }

      if (method === 'POST' && url.pathname === '/v1/auth/refresh') {
        const body = await readJsonBody(request);
        return sendJson(response, 200, await authService.refreshSession(body.refreshToken));
      }

      if (method === 'POST' && url.pathname === '/v1/auth/logout') {
        const context = requireAuth(request.headers);
        return sendJson(response, 200, await authService.logout(context.sessionId));
      }

      if (method === 'GET' && url.pathname === '/v1/me') {
        const context = requireAuth(request.headers);
        return sendJson(response, 200, await authService.getMe(context.internalUserId));
      }

      if (method === 'POST' && url.pathname === '/v1/account/email-bind/start') {
        const context = requireAuth(request.headers);
        const body = await readJsonBody(request);
        return sendJson(
          response,
          200,
          await authService.startEmailBind(context.internalUserId, body),
        );
      }

      if (method === 'POST' && url.pathname === '/v1/account/email-bind/verify') {
        const context = requireAuth(request.headers);
        const body = await readJsonBody(request);
        return sendJson(
          response,
          200,
          await authService.verifyEmailBind(context.internalUserId, body),
        );
      }

      const mergePreviewMatch = url.pathname.match(
        /^\/v1\/account\/merge-preview\/([^/]+)$/,
      );
      if (method === 'GET' && mergePreviewMatch) {
        const context = requireAuth(request.headers);
        return sendJson(
          response,
          200,
          await authService.getMergePreview(
            context.internalUserId,
            decodeURIComponent(mergePreviewMatch[1]),
          ),
        );
      }

      if (method === 'POST' && url.pathname === '/v1/account/merge-confirm') {
        const context = requireAuth(request.headers);
        const body = await readJsonBody(request);
        return sendJson(
          response,
          200,
          await authService.confirmMerge(context.internalUserId, body),
        );
      }

      if (method === 'GET' && url.pathname === '/v1/today') {
        const context = requireAuth(request.headers);
        return sendJson(
          response,
          200,
          await adapter.getToday({ internalUserId: context.internalUserId }),
        );
      }

      if (method === 'GET' && url.pathname === '/v1/plan/active') {
        const context = requireAuth(request.headers);
        return sendJson(
          response,
          200,
          await adapter.getActivePlan({ internalUserId: context.internalUserId }),
        );
      }

      if (method === 'GET' && url.pathname === '/v1/reports/overview') {
        const context = requireAuth(request.headers);
        return sendJson(
          response,
          200,
          await adapter.getReportsOverview({ internalUserId: context.internalUserId }),
        );
      }

      if (method === 'POST' && url.pathname === '/v1/study/sessions') {
        const context = requireAuth(request.headers);
        const body = await readJsonBody(request);
        return sendJson(
          response,
          200,
          await studyDomain.startStudySession({
            internalUserId: context.internalUserId,
            mode: body.mode,
            wordbookId: body.wordbookId,
            entrySourceIds: body.entrySourceIds,
            entryPayloads: body.entryPayloads,
            distractorPayloads: body.distractorPayloads,
            questionTypeWeights: body.questionTypeWeights,
          }),
        );
      }

      if (method === 'GET' && url.pathname === '/v1/study/resume-hint') {
        const context = requireAuth(request.headers);
        return sendJson(
          response,
          200,
          await studyDomain.getResumeSessionHint({
            internalUserId: context.internalUserId,
          }),
        );
      }

      if (method === 'POST' && url.pathname === '/v1/study/answers') {
        const context = requireAuth(request.headers);
        const body = await readJsonBody(request);
        return sendJson(
          response,
          200,
          await studyDomain.submitStudyAnswer({
            internalUserId: context.internalUserId,
            questionId: body.questionId,
            response: body.response,
            responseTimeMs: body.responseTimeMs,
          }),
        );
      }

      if (method === 'POST' && url.pathname === '/v1/study/mastered') {
        const context = requireAuth(request.headers);
        const body = await readJsonBody(request);
        return sendJson(
          response,
          200,
          await studyDomain.markStudyEntryMastered({
            internalUserId: context.internalUserId,
            entrySourceId: body.entrySourceId,
            reason: body.reason,
          }),
        );
      }

      if (method === 'POST' && url.pathname === '/v1/study/disputed-meaning/accept') {
        const context = requireAuth(request.headers);
        const body = await readJsonBody(request);
        return sendJson(
          response,
          200,
          await studyDomain.acceptDisputedMeaning({
            internalUserId: context.internalUserId,
            questionId: body.questionId,
            submittedAnswer: body.submittedAnswer,
          }),
        );
      }

      const completeSessionMatch = url.pathname.match(
        /^\/v1\/study\/sessions\/([^/]+)\/complete$/,
      );
      if (method === 'POST' && completeSessionMatch) {
        const context = requireAuth(request.headers);
        return sendJson(
          response,
          200,
          await studyDomain.completeStudySession({
            internalUserId: context.internalUserId,
            sessionId: decodeURIComponent(completeSessionMatch[1]),
          }),
        );
      }

      const cancelSessionMatch = url.pathname.match(
        /^\/v1\/study\/sessions\/([^/]+)\/cancel$/,
      );
      if (method === 'POST' && cancelSessionMatch) {
        const context = requireAuth(request.headers);
        await studyDomain.cancelStudySession({
          internalUserId: context.internalUserId,
          sessionId: decodeURIComponent(cancelSessionMatch[1]),
        });
        return sendJson(response, 200, { ok: true });
      }

      if (method === 'GET' && url.pathname === '/v1/wrong-words') {
        const context = requireAuth(request.headers);
        return sendJson(
          response,
          200,
          await studyDomain.listWrongWords({ internalUserId: context.internalUserId }),
        );
      }

      const wrongWordMatch = url.pathname.match(/^\/v1\/wrong-words\/([^/]+)$/);
      if (method === 'GET' && wrongWordMatch) {
        const context = requireAuth(request.headers);
        return sendJson(
          response,
          200,
          await studyDomain.getWrongWordDetail({
            internalUserId: context.internalUserId,
            entryId: decodeURIComponent(wrongWordMatch[1]),
          }),
        );
      }

      if (method === 'GET' && url.pathname === '/v1/rewards/today') {
        const context = requireAuth(request.headers);
        return sendJson(
          response,
          200,
          await adapter.getTodayReward({ internalUserId: context.internalUserId }),
        );
      }

      if (method === 'POST' && url.pathname === '/v1/rewards/today/claim') {
        const context = requireAuth(request.headers);
        return sendJson(
          response,
          200,
          await adapter.claimTodayReward({
            internalUserId: context.internalUserId,
            now: new Date(),
          }),
        );
      }

      const leaderboardMatch = url.pathname.match(/^\/v1\/leaderboard\/([^/]+)$/);
      if (method === 'GET' && leaderboardMatch) {
        const context = requireAuth(request.headers);
        return sendJson(
          response,
          200,
          await adapter.getLeaderboard({
            internalUserId: context.internalUserId,
            metric: decodeURIComponent(leaderboardMatch[1]),
          }),
        );
      }

      return sendJson(response, 404, {
        error: {
          code: 'ROUTE_NOT_FOUND',
          message: 'Route not found',
        },
      });
    } catch (error) {
      return sendError(response, error);
    }
  };
}
