/**
 * Study Client
 */

import {
  startStudySession as bridgeStartStudySession,
  submitStudyAnswer as bridgeSubmitStudyAnswer,
  completeStudySession as bridgeCompleteStudySession,
  cancelStudySession as bridgeCancelStudySession,
  type StartSessionRequest,
  type StartSessionResponse,
  type SubmitAnswerRequest,
  type SubmitAnswerResponse,
  type CompleteSessionResponse,
  type SessionMode,
} from './mobile-bridge';

export type {
  StartSessionRequest,
  StartSessionResponse,
  SubmitAnswerRequest,
  SubmitAnswerResponse,
  CompleteSessionResponse,
  SessionProgress,
  SessionMode,
  StudySession,
  StudyQuestion,
  StudyResult,
  SessionSummary,
  QuestionType,
  ChoiceOption,
} from './mobile-bridge';

export async function startStudySession(
  request: StartSessionRequest,
): Promise<StartSessionResponse> {
  return bridgeStartStudySession(request);
}

export async function submitStudyAnswer(
  request: SubmitAnswerRequest,
): Promise<SubmitAnswerResponse> {
  return bridgeSubmitStudyAnswer(request);
}

export async function completeStudySession(
  sessionId: string,
): Promise<CompleteSessionResponse> {
  return bridgeCompleteStudySession(sessionId);
}

export async function cancelStudySession(sessionId: string): Promise<void> {
  return bridgeCancelStudySession(sessionId);
}

export async function checkActiveSession(
  mode: SessionMode,
  wordbookId: number | null,
  entrySourceIds: string[],
): Promise<StartSessionResponse | null> {
  try {
    return await bridgeStartStudySession({mode, wordbookId, entrySourceIds});
  } catch {
    return null;
  }
}
