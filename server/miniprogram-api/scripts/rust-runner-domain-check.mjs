import { MemorySupabaseAdapter } from '../src/supabase/memory-adapter.js';
import { MemoryStudyDomain } from '../src/study-domain/memory-domain.js';
import { RustRunnerStudyDomain } from '../src/study-domain/rust-runner-domain.js';

const adapter = new MemorySupabaseAdapter();
const fallbackDomain = new MemoryStudyDomain({ adapter });
const domain = new RustRunnerStudyDomain({ adapter, fallbackDomain });
const account = await adapter.findOrCreateWechatIdentity({
  openid: 'rust-study-openid',
  unionid: 'rust-study-unionid',
});

const payloads = [
  entry('alpha', 'alpha', 'alpha meaning'),
  entry('beta', 'beta', 'beta meaning'),
];
const start = await domain.startStudySession({
  internalUserId: account.internalUserId,
  mode: 'mixedTest',
  entrySourceIds: [],
  entryPayloads: payloads,
  distractorPayloads: [
    entry('gamma', 'gamma', 'gamma meaning'),
    entry('delta', 'delta', 'delta meaning'),
    entry('epsilon', 'epsilon', 'epsilon meaning'),
  ],
  questionTypeWeights: [{ questionType: 'enToCnChoice', weight: 100 }],
});

if (start.progress.total !== 2) {
  throw new Error(`Expected Rust runner to build 2 questions, got ${start.progress.total}`);
}
if (!start.currentQuestion.correctChoiceLabel) {
  throw new Error('Rust runner choice question missing correctChoiceLabel');
}

const first = await domain.submitStudyAnswer({
  internalUserId: account.internalUserId,
  questionId: start.currentQuestion.questionId,
  response: start.currentQuestion.correctChoiceLabel,
  responseTimeMs: 100,
});
if (first.isComplete) {
  throw new Error('First answer should advance to next Rust question, not complete session');
}
if (first.result.outcome !== 'correct') {
  throw new Error(`Expected Rust AnswerEvaluator correct outcome, got ${first.result.outcome}`);
}

const second = await domain.submitStudyAnswer({
  internalUserId: account.internalUserId,
  questionId: first.currentQuestion.questionId,
  response: first.currentQuestion.correctChoiceLabel,
  responseTimeMs: 150,
});
if (!second.isComplete) {
  throw new Error('Second answer should complete the Rust session');
}
if (second.summary.totalQuestions !== 2 || second.summary.correctCount !== 2) {
  throw new Error('Rust SessionSummaryService summary did not aggregate both answers');
}

console.log(
  JSON.stringify(
    {
      sessionId: start.session.sessionId,
      totalQuestions: start.progress.total,
      firstOutcome: first.result.outcome,
      summaryQuestions: second.summary.totalQuestions,
      summaryCorrect: second.summary.correctCount,
      nextAction: second.nextAction,
    },
    null,
    2,
  ),
);

const sourcedStart = await domain.startStudySession({
  internalUserId: account.internalUserId,
  mode: 'mixedTest',
  entrySourceIds: [],
  questionTypeWeights: [{ questionType: 'enToCnChoice', weight: 100 }],
});

if (sourcedStart.progress.total < 1 || !sourcedStart.currentQuestion.correctChoiceLabel) {
  throw new Error('Rust runner did not source payloads from adapter');
}

function entry(sourceId, word, meaning) {
  return {
    sourceId,
    word,
    partOfSpeech: 'n',
    frequency: 1,
    phoneticUs: undefined,
    phoneticUk: undefined,
    meaningDetails: [{ pos: 'n', meaningCn: meaning, meaningEn: undefined }],
    meanings: [meaning],
    exampleSentence: `${word} example`,
    exampleTranslation: `${meaning} translation`,
  };
}
