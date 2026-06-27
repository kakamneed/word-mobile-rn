import {
  assertResponseFields,
  requiredStartSessionResponseFields,
  requiredStudyDomainMethods,
  requiredStudyQuestionFields,
  requiredSubmitAnswerResponseFields,
} from '../src/study-domain/contract.js';
import { MemorySupabaseAdapter } from '../src/supabase/memory-adapter.js';
import { MemoryStudyDomain } from '../src/study-domain/memory-domain.js';
import { RustRunnerStudyDomain } from '../src/study-domain/rust-runner-domain.js';

const adapter = new MemorySupabaseAdapter();
const domain = new MemoryStudyDomain({ adapter });
const rustDomain = new RustRunnerStudyDomain({ fallbackDomain: domain });
const account = await adapter.findOrCreateWechatIdentity({
  openid: 'study-domain-openid',
  unionid: 'study-domain-unionid',
});

for (const method of requiredStudyDomainMethods) {
  if (typeof domain[method] !== 'function') {
    throw new Error(`Current adapter missing implemented study method: ${method}`);
  }
  if (typeof rustDomain[method] !== 'function') {
    throw new Error(`Rust runner adapter missing implemented study method: ${method}`);
  }
}

const start = await domain.startStudySession({
  internalUserId: account.internalUserId,
  mode: 'newWord',
});
assertResponseFields(start, requiredStartSessionResponseFields, 'StartSessionResponse');
assertResponseFields(
  start.currentQuestion,
  requiredStudyQuestionFields,
  'StudyQuestion',
);

const submit = await domain.submitStudyAnswer({
  internalUserId: account.internalUserId,
  questionId: start.currentQuestion.questionId,
  response: 'easy to break',
  responseTimeMs: 1200,
});
assertResponseFields(submit, requiredSubmitAnswerResponseFields, 'SubmitAnswerResponse');

console.log(
  JSON.stringify(
    {
      requiredDomainMethods: requiredStudyDomainMethods.length,
      implementedNow: requiredStudyDomainMethods.length,
      startFields: requiredStartSessionResponseFields.length,
      questionFields: requiredStudyQuestionFields.length,
      submitFields: requiredSubmitAnswerResponseFields.length,
      currentGap: 'Rust runner owns start/submit/complete and adapter projections for Study; remaining work is production content import and Mini Program DevTools smoke',
    },
    null,
    2,
  ),
);
