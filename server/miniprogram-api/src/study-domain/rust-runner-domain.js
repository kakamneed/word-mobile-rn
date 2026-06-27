import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const currentDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(currentDir, '..', '..', '..');

export class RustRunnerStudyDomain {
  constructor({
    fallbackDomain,
    adapter,
    command = 'cargo',
    args = ['run', '-p', 'word-study-domain-runner', '--quiet', '--'],
    cwd = repoRoot,
  }) {
    this.fallbackDomain = fallbackDomain;
    this.adapter = adapter;
    this.command = command;
    this.args = args;
    this.cwd = cwd;
    this.sessionsById = new Map();
    this.latestSessionByUser = new Map();
  }

  async startStudySession(input) {
    let resolvedInput = input;
    if (!hasPayloads(resolvedInput) && this.adapter?.getStudyPayloads) {
      const payloads = await this.adapter.getStudyPayloads({
        internalUserId: input.internalUserId,
        mode: input.mode,
        wordbookId: input.wordbookId,
        entrySourceIds: input.entrySourceIds ?? [],
      });
      resolvedInput = {
        ...input,
        entryPayloads: payloads.entryPayloads,
        distractorPayloads: payloads.distractorPayloads,
      };
    }
    if (!hasPayloads(resolvedInput)) {
      return this.fallbackDomain.startStudySession(input);
    }
    const runnerResponse = await this.callRunner('build-session', {
      mode: resolvedInput.mode,
      wordbookId: resolvedInput.wordbookId,
      entrySourceIds: resolvedInput.entrySourceIds ?? [],
      entryPayloads: resolvedInput.entryPayloads ?? [],
      distractorPayloads: resolvedInput.distractorPayloads ?? [],
      questionTypeWeights: resolvedInput.questionTypeWeights ?? [],
    });
    const questions = runnerResponse.questions ?? [runnerResponse.currentQuestion];
    const sessionState = {
      internalUserId: input.internalUserId,
      session: runnerResponse.session,
      questions,
      results: [],
      currentIndex: 0,
      completedAt: undefined,
      cancelledAt: undefined,
    };
    this.sessionsById.set(runnerResponse.session.sessionId, sessionState);
    this.latestSessionByUser.set(input.internalUserId, runnerResponse.session.sessionId);
    await this.adapter?.recordStudyEvent?.({
      internalUserId: input.internalUserId,
      sessionId: runnerResponse.session.sessionId,
      eventType: 'session_started',
      payload: {
        mode: runnerResponse.session.mode,
        session: runnerResponse.session,
        questionCount: questions.length,
      },
    });
    return stripRunnerQuestions(runnerResponse);
  }

  async submitStudyAnswer(input) {
    const state = this.findSessionForQuestion(input.internalUserId, input.questionId);
    if (!state) {
      return this.fallbackDomain.submitStudyAnswer(input);
    }
    if (state.cancelledAt || state.completedAt) {
      throw domainError('Study session is not active', 409);
    }

    const question = state.questions[state.currentIndex];
    if (!question || question.questionId !== input.questionId) {
      throw domainError('Study session question is unavailable', 404);
    }

    const result = await this.callRunner('evaluate-answer', {
      question,
      answer: {
        questionId: input.questionId,
        response: input.response,
        responseTimeMs: input.responseTimeMs,
      },
    });
    await this.adapter?.recordStudyEvent?.({
      internalUserId: input.internalUserId,
      sessionId: state.session.sessionId,
      eventType: 'answer_submitted',
      payload: {
        mode: state.session.mode,
        question,
        result,
      },
    });
    state.results.push(result);
    state.currentIndex += 1;
    const isComplete = state.currentIndex >= state.questions.length;
    const answeredQuestions = state.results.map((item, index) => ({
      question: state.questions[index],
      result: item,
    }));

    if (isComplete) {
      const completed = await this.completeState(state);
      return {
        result,
        isComplete: true,
        currentQuestion: undefined,
        summary: completed.summary,
        nextAction: completed.nextAction,
        progress: { current: state.questions.length, total: state.questions.length },
        answeredQuestions,
      };
    }

    return {
      result,
      isComplete: false,
      currentQuestion: state.questions[state.currentIndex],
      summary: undefined,
      nextAction: undefined,
      progress: { current: state.currentIndex + 1, total: state.questions.length },
      answeredQuestions,
    };
  }

  getResumeSessionHint(input) {
    const state = this.findLatestActiveSession(input.internalUserId);
    if (!state) {
      return this.fallbackDomain.getResumeSessionHint(input);
    }
    const question = state.questions[state.currentIndex];
    return {
      hasResume: true,
      mode: state.session.mode,
      current: state.currentIndex + 1,
      total: state.questions.length,
      word: question?.word,
    };
  }

  async completeStudySession(input) {
    const state = this.sessionsById.get(input.sessionId);
    if (!state || state.internalUserId !== input.internalUserId) {
      return this.fallbackDomain.completeStudySession(input);
    }
    return this.completeState(state);
  }

  cancelStudySession(input) {
    const state = this.sessionsById.get(input.sessionId);
    if (!state || state.internalUserId !== input.internalUserId) {
      return this.fallbackDomain.cancelStudySession(input);
    }
    state.cancelledAt = new Date().toISOString();
    return undefined;
  }

  async markStudyEntryMastered(input) {
    const state = this.findLatestActiveSession(input.internalUserId);
    if (!state) {
      return this.fallbackDomain.markStudyEntryMastered(input);
    }
    const before = state.questions.length;
    state.questions = state.questions.filter(
      (question) => question.entrySourceId !== input.entrySourceId,
    );
    const prunedQuestionCount = before - state.questions.length;
    if (state.currentIndex > state.questions.length) {
      state.currentIndex = state.questions.length;
    }
    const isComplete = state.currentIndex >= state.questions.length;
    const completed = isComplete ? await this.completeState(state) : undefined;
    return {
      entrySourceId: input.entrySourceId,
      entryId: undefined,
      prunedQuestionCount,
      isComplete,
      currentQuestion: isComplete ? undefined : state.questions[state.currentIndex],
      summary: completed?.summary,
      nextAction: completed?.nextAction,
      progress: {
        current: isComplete ? state.questions.length : state.currentIndex + 1,
        total: state.questions.length,
      },
      answeredQuestions: state.results.map((item, index) => ({
        question: state.questions[index],
        result: item,
      })),
    };
  }

  async acceptDisputedMeaning(input) {
    const state = this.findSessionForQuestion(input.internalUserId, input.questionId);
    if (!state) {
      return this.fallbackDomain.acceptDisputedMeaning(input);
    }
    const question = state.questions[state.currentIndex];
    if (!question || question.questionId !== input.questionId) {
      throw domainError('Study session question is unavailable', 404);
    }
    const acceptedMeaning = input.submittedAnswer ?? '';
    const result = {
      questionId: question.questionId,
      entrySourceId: question.entrySourceId,
      questionType: question.questionType,
      userResponse: acceptedMeaning,
      normalizedResponse: acceptedMeaning.trim(),
      correctAnswer: acceptedMeaning,
      outcome: 'correct',
      responseTimeMs: 0,
      answeredAt: new Date().toISOString(),
    };
    state.results.push(result);
    state.currentIndex += 1;
    return {
      result,
      progress: {
        current: Math.min(state.currentIndex + 1, state.questions.length),
        total: state.questions.length,
      },
      answeredQuestions: state.results.map((item, index) => ({
        question: state.questions[index],
        result: item,
      })),
      entrySourceId: question.entrySourceId,
      word: question.word,
      acceptedMeaning,
    };
  }

  async completeState(state) {
    if (state.completedAt) {
      return state.completedResponse;
    }
    const completedAt = new Date().toISOString();
    const completedResponse = await this.callRunner('complete-session', {
      session: state.session,
      results: state.results,
      completedAt,
    });
    await this.adapter?.recordStudyEvent?.({
      internalUserId: state.internalUserId,
      sessionId: state.session.sessionId,
      eventType: 'session_completed',
      payload: {
        mode: state.session.mode,
        summary: completedResponse.summary,
        nextAction: completedResponse.nextAction,
      },
      occurredAt: completedAt,
    });
    state.completedAt = completedAt;
    state.completedResponse = completedResponse;
    return completedResponse;
  }

  findLatestActiveSession(internalUserId) {
    const sessionId = this.latestSessionByUser.get(internalUserId);
    const state = sessionId ? this.sessionsById.get(sessionId) : undefined;
    if (!state || state.completedAt || state.cancelledAt) {
      return undefined;
    }
    return state;
  }

  findSessionForQuestion(internalUserId, questionId) {
    const latest = this.findLatestActiveSession(internalUserId);
    if (latest?.questions[latest.currentIndex]?.questionId === questionId) {
      return latest;
    }
    return [...this.sessionsById.values()].find(
      (state) =>
        state.internalUserId === internalUserId &&
        !state.completedAt &&
        !state.cancelledAt &&
        state.questions[state.currentIndex]?.questionId === questionId,
    );
  }

  callRunner(command, payload) {
    return new Promise((resolvePromise, reject) => {
      const child = spawn(this.command, [...this.args, command], {
        cwd: this.cwd,
        stdio: ['pipe', 'pipe', 'pipe'],
        windowsHide: true,
      });
      let stdout = '';
      let stderr = '';

      child.stdout.setEncoding('utf8');
      child.stderr.setEncoding('utf8');
      child.stdout.on('data', (chunk) => {
        stdout += chunk;
      });
      child.stderr.on('data', (chunk) => {
        stderr += chunk;
      });
      child.on('error', reject);
      child.on('close', (code) => {
        if (code !== 0) {
          const error = new Error(
            `Rust study runner failed: ${stderr.trim() || `exit ${code}`}`,
          );
          error.code = 'STUDY_DOMAIN_RUNNER_FAILED';
          error.statusCode = 500;
          reject(error);
          return;
        }
        try {
          resolvePromise(JSON.parse(stdout));
        } catch (error) {
          error.code = 'STUDY_DOMAIN_RUNNER_INVALID_JSON';
          error.statusCode = 500;
          reject(error);
        }
      });
      child.stdin.end(JSON.stringify(payload));
    });
  }
}

function hasPayloads(input) {
  return Array.isArray(input.entryPayloads) && input.entryPayloads.length > 0;
}

function stripRunnerQuestions(response) {
  const { questions: _questions, ...rest } = response;
  return rest;
}

function domainError(message, statusCode) {
  const error = new Error(message);
  error.code = 'DOMAIN_CONFLICT';
  error.statusCode = statusCode;
  return error;
}
