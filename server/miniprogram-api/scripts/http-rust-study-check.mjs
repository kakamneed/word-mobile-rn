import { once } from 'node:events';

import { AuthService } from '../src/auth/service.js';
import { createRouter } from '../src/http/router.js';
import { MemorySupabaseAdapter } from '../src/supabase/memory-adapter.js';
import { MemoryStudyDomain } from '../src/study-domain/memory-domain.js';
import { RustRunnerStudyDomain } from '../src/study-domain/rust-runner-domain.js';
import { createFakeWechatProvider } from '../src/wechat/fake-provider.js';
import { createServer } from 'node:http';

const adapter = new MemorySupabaseAdapter();
const authService = new AuthService({
  adapter,
  wechatProvider: createFakeWechatProvider(),
});
const fallbackDomain = new MemoryStudyDomain({ adapter });
const studyDomain = new RustRunnerStudyDomain({ adapter, fallbackDomain });
const server = createServer(createRouter({ authService, adapter, studyDomain }));
server.listen(0);
await once(server, 'listening');
const port = server.address().port;
const baseUrl = `http://127.0.0.1:${port}`;

try {
  const login = await post('/v1/auth/wechat-mp/login', {
    code: 'http-rust-study-code',
    client: { platform: 'wechat_mp' },
  });
  const token = login.session.accessToken;
  const start = await post(
    '/v1/study/sessions',
    {
      mode: 'mixedTest',
      entrySourceIds: [],
      questionTypeWeights: [{ questionType: 'enToCnChoice', weight: 100 }],
    },
    token,
  );
  const beforeReports = await get('/v1/reports/overview', token);
  let currentQuestion = start.currentQuestion;
  let submit;
  let answerCount = 0;
  while (currentQuestion) {
    answerCount += 1;
    submit = await post(
      '/v1/study/answers',
      {
        questionId: currentQuestion.questionId,
        response: currentQuestion.correctChoiceLabel,
        responseTimeMs: 100 + answerCount,
      },
      token,
    );
    currentQuestion = submit.currentQuestion;
  }

  if (start.progress.total < 1) {
    throw new Error(`Expected HTTP Rust study session to have questions, got ${start.progress.total}`);
  }
  if (answerCount !== start.progress.total) {
    throw new Error(`Expected to answer ${start.progress.total} questions, answered ${answerCount}`);
  }
  if (!submit.isComplete || submit.summary.totalQuestions !== answerCount) {
    throw new Error('HTTP Rust study did not complete with Rust summary');
  }
  const eventTypes = adapter.studyEvents.map((event) => event.eventType);
  if (
    !eventTypes.includes('session_started') ||
    !eventTypes.includes('answer_submitted') ||
    !eventTypes.includes('session_completed')
  ) {
    throw new Error(`Expected Rust study flow to record events, got ${eventTypes.join(',')}`);
  }
  const afterReports = await get('/v1/reports/overview', token);
  if (
    afterReports.totalQuestionsAnswered !==
    beforeReports.totalQuestionsAnswered + answerCount
  ) {
    throw new Error('Expected Rust answer events to update report projection');
  }

  console.log(
    JSON.stringify(
      {
        sessionId: start.session.sessionId,
        totalQuestions: start.progress.total,
        lastOutcome: submit.result.outcome,
        completed: submit.isComplete,
        summaryQuestions: submit.summary.totalQuestions,
        events: eventTypes.length,
        reportQuestions: afterReports.totalQuestionsAnswered,
      },
      null,
      2,
    ),
  );
} finally {
  server.close();
}

async function get(path, token) {
  const response = await fetch(`${baseUrl}${path}`, {
    headers: {
      ...(token ? { authorization: `Bearer ${token}` } : {}),
    },
  });
  const json = await response.json();
  if (!response.ok) {
    throw new Error(`${path} failed: ${JSON.stringify(json)}`);
  }
  return json;
}

async function post(path, body, token) {
  const response = await fetch(`${baseUrl}${path}`, {
    method: 'POST',
    headers: {
      'content-type': 'application/json',
      ...(token ? { authorization: `Bearer ${token}` } : {}),
    },
    body: JSON.stringify(body),
  });
  const json = await response.json();
  if (!response.ok) {
    throw new Error(`${path} failed: ${JSON.stringify(json)}`);
  }
  return json;
}
