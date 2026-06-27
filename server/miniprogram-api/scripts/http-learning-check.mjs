import { once } from 'node:events';

import { createMiniprogramApiServer } from '../src/server.js';
import { MemorySupabaseAdapter } from '../src/supabase/memory-adapter.js';
import { createFakeWechatProvider } from '../src/wechat/fake-provider.js';

const server = createMiniprogramApiServer({
  adapter: new MemorySupabaseAdapter(),
  wechatProvider: createFakeWechatProvider(),
});

server.listen(0, '127.0.0.1');
await once(server, 'listening');
const address = server.address();
const baseUrl = `http://127.0.0.1:${address.port}`;

try {
  const login = await post('/v1/auth/wechat-mp/login', {
    code: 'learning-code',
    appVersion: '1.0.0',
    client: { platform: 'wechat_mp' },
  });
  const token = login.session.accessToken;

  const today = await get('/v1/today', token);
  const plan = await get('/v1/plan/active', token);
  const reports = await get('/v1/reports/overview', token);
  const reward = await get('/v1/rewards/today', token);
  const claimed = await post('/v1/rewards/today/claim', {}, token);
  const leaderboard = await get('/v1/leaderboard/weekly', token);
  const session = await post('/v1/study/sessions', { mode: 'newWord' }, token);
  const beforeWrongWords = await get('/v1/wrong-words', token);
  const answered = await post(
    '/v1/study/answers',
    {
      questionId: session.currentQuestion.questionId,
      response: 'easy to break',
      responseTimeMs: 1500,
    },
    token,
  );
  const afterReports = await get('/v1/reports/overview', token);
  const afterWrongWords = await get('/v1/wrong-words', token);
  const wrongDetail = await get(`/v1/wrong-words/${afterWrongWords[0].entryId}`, token);
  const afterLeaderboard = await get('/v1/leaderboard/weekly', token);

  if (!today.activePlan || today.activePlan.name !== plan.name) {
    throw new Error('Today must include the active plan');
  }
  if (reports.totalQuestionsAnswered <= 0) {
    throw new Error('Reports overview must include totals');
  }
  if (!reward.rewardId || !claimed.rewardId) {
    throw new Error('Reward endpoints must return reward state');
  }
  if (!leaderboard.currentUser) {
    throw new Error('Leaderboard must include current user summary');
  }
  if (!answered.isComplete || answered.result.outcome !== 'incorrect') {
    throw new Error('Study answer should complete as incorrect for this fixture');
  }
  if (afterReports.totalQuestionsAnswered !== reports.totalQuestionsAnswered + 1) {
    throw new Error('Submit answer must update report totals');
  }
  if (afterWrongWords.length < beforeWrongWords.length || !wrongDetail.word) {
    throw new Error('Wrong-word endpoints must reflect incorrect answers');
  }
  if (afterLeaderboard.currentUser.score !== leaderboard.currentUser.score + 1) {
    throw new Error('Leaderboard must derive from updated report totals');
  }

  console.log(
    JSON.stringify(
      {
        todayDate: today.todayDate,
        planName: plan.name,
        questions: reports.totalQuestionsAnswered,
        rewardId: reward.rewardId,
        rank: leaderboard.currentUser.rank,
        answerOutcome: answered.result.outcome,
        afterQuestions: afterReports.totalQuestionsAnswered,
      },
      null,
      2,
    ),
  );
} finally {
  server.close();
}

async function get(path, accessToken) {
  const response = await fetch(`${baseUrl}${path}`, {
    headers: {
      Authorization: `Bearer ${accessToken}`,
    },
  });
  return parse(response);
}

async function post(path, body, accessToken) {
  const response = await fetch(`${baseUrl}${path}`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      ...(accessToken ? { Authorization: `Bearer ${accessToken}` } : {}),
    },
    body: JSON.stringify(body),
  });
  return parse(response);
}

async function parse(response) {
  const body = await response.json();
  if (!response.ok) {
    throw new Error(`${response.status}: ${JSON.stringify(body)}`);
  }
  return body;
}
