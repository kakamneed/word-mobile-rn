import {
  CompleteSessionResponse,
  StartSessionResponse,
  StudyQuestion,
  StudyResult,
} from './types';

export type StudyFeedItem =
  | {
      kind: 'answered';
      question: StudyQuestion;
      result: StudyResult;
      responseOverride?: string;
    }
  | {
      kind: 'unanswered';
      question: StudyQuestion;
    }
  | {
      kind: 'completion';
      completion: CompleteSessionResponse;
      answeredQuestions: Array<{ question: StudyQuestion; result: StudyResult }>;
    };

export function buildStudyFeedItems(input: {
  session?: StartSessionResponse;
  lastSubmittedQuestion?: StudyQuestion;
  lastSubmittedResult?: StudyResult;
  lastSubmittedResponse?: string;
  completion?: CompleteSessionResponse;
}): StudyFeedItem[] {
  const { session, lastSubmittedQuestion, lastSubmittedResult, lastSubmittedResponse, completion } = input;
  if (!session) return [];
  const answeredQuestions = mergeLatestAnsweredQuestion(
    session.answeredQuestions,
    lastSubmittedQuestion,
    lastSubmittedResult,
  );
  const items: StudyFeedItem[] = answeredQuestions.map((answered) => ({
    kind: 'answered',
    question: answered.question,
    result: answered.result,
    responseOverride:
      lastSubmittedQuestion?.questionId === answered.question.questionId
        ? lastSubmittedResponse
        : undefined,
  }));
  const current = session.currentQuestion;
  const currentAlreadyAnswered = items.some(
    (item) => item.kind !== 'completion' && item.question.questionId === current.questionId,
  );
  if (!currentAlreadyAnswered && !completion) {
    items.push({ kind: 'unanswered', question: current });
  }
  if (completion) {
    items.push({ kind: 'completion', completion, answeredQuestions });
  }
  return items;
}

export function mergeLatestAnsweredQuestion(
  answeredQuestions: Array<{ question: StudyQuestion; result: StudyResult }>,
  question?: StudyQuestion,
  result?: StudyResult,
) {
  if (!question || !result) return answeredQuestions;
  const exists = answeredQuestions.some(
    (answered) => answered.question.questionId === question.questionId,
  );
  if (exists) {
    return answeredQuestions.map((answered) =>
      answered.question.questionId === question.questionId ? { question, result } : answered,
    );
  }
  return [...answeredQuestions, { question, result }];
}
