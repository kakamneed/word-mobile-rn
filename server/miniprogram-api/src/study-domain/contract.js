export const requiredStudyDomainMethods = [
  'startStudySession',
  'getResumeSessionHint',
  'submitStudyAnswer',
  'completeStudySession',
  'cancelStudySession',
  'markStudyEntryMastered',
  'acceptDisputedMeaning',
];

export const requiredStartSessionResponseFields = [
  'session',
  'currentQuestion',
  'progress',
  'answeredQuestions',
];

export const requiredSubmitAnswerResponseFields = [
  'result',
  'progress',
  'answeredQuestions',
  'isComplete',
];

export const requiredStudyQuestionFields = [
  'questionId',
  'questionType',
  'entrySourceId',
  'word',
  'prompt',
  'acceptedMeanings',
  'questionIndex',
  'totalQuestions',
  'hasHint',
];

export function assertResponseFields(response, fields, label) {
  for (const field of fields) {
    if (!(field in response)) {
      throw new Error(`${label} missing required field: ${field}`);
    }
  }
}
