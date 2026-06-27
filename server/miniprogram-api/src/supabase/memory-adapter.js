import { randomUUID } from 'node:crypto';
import {
  defaultPlan,
  defaultReports,
  defaultReward,
  defaultWrongWords,
  firstQuestionForMode,
  isCorrect,
  applyAnswerToReports,
  leaderboardFromReports,
  todayFromPlanAndReports,
  wrongWordDetailFromEntry,
} from '../learning/fixtures.js';

export class MemorySupabaseAdapter {
  constructor() {
    this.accounts = new Map();
    this.identitiesByProviderSubject = new Map();
    this.sessionsById = new Map();
    this.sessionsByRefreshHash = new Map();
    this.emailChallenges = new Map();
    this.mergeDecisions = new Map();
    this.planByUser = new Map();
    this.reportsByUser = new Map();
    this.rewardByUser = new Map();
    this.wrongWordsByUser = new Map();
    this.studySessions = new Map();
    this.studyEvents = [];
  }

  async findOrCreateWechatIdentity({ openid, unionid }) {
    const key = `wechat_mp:${openid}`;
    const existingInternalUserId = this.identitiesByProviderSubject.get(key);
    if (existingInternalUserId) {
      const account = this.accounts.get(existingInternalUserId);
      account.lastLoginAt = new Date().toISOString();
      return this.toAccountState(account);
    }

    const internalUserId = randomUUID();
    const account = {
      internalUserId,
      primaryIdentity: 'wechat_mp',
      status: 'active',
      hasEmailBinding: false,
      hasWechatBinding: true,
      phase: 'signed_in_active',
      needsBindDecision: false,
      cloudDataState: 'empty_or_ready',
      identities: [
        {
          provider: 'wechat_mp',
          providerSubject: openid,
          providerSubjectSecondary: unionid,
          isVerified: true,
        },
      ],
      lastLoginAt: new Date().toISOString(),
    };
    this.accounts.set(internalUserId, account);
    this.identitiesByProviderSubject.set(key, internalUserId);
    return this.toAccountState(account);
  }

  async issueMiniprogramSession({
    internalUserId,
    refreshTokenHash,
    expiresAt,
    client,
  }) {
    const session = {
      sessionId: randomUUID(),
      internalUserId,
      refreshTokenHash,
      expiresAt: expiresAt.toISOString(),
      revokedAt: undefined,
      client,
    };
    this.sessionsById.set(session.sessionId, session);
    this.sessionsByRefreshHash.set(refreshTokenHash, session);
    return session;
  }

  async refreshMiniprogramSession({ refreshTokenHash, now }) {
    const session = this.sessionsByRefreshHash.get(refreshTokenHash);
    if (!session || session.revokedAt || new Date(session.expiresAt) <= now) {
      const error = new Error('Refresh session is invalid or expired');
      error.code = 'SESSION_EXPIRED';
      error.statusCode = 401;
      throw error;
    }
    return this.getAccountState({ internalUserId: session.internalUserId });
  }

  async revokeMiniprogramSession({ sessionId, revokedAt }) {
    const session = this.sessionsById.get(sessionId);
    if (session) {
      session.revokedAt = revokedAt.toISOString();
    }
  }

  async getAccountState({ internalUserId }) {
    const account = this.accounts.get(internalUserId);
    if (!account || account.status !== 'active') {
      const error = new Error('Account is unavailable');
      error.code = 'ACCOUNT_DISABLED';
      error.statusCode = 403;
      throw error;
    }
    return this.toAccountState(account);
  }

  async startEmailBind({ internalUserId, normalizedEmail, now }) {
    await this.getAccountState({ internalUserId });
    const challenge = {
      challengeId: randomUUID(),
      internalUserId,
      normalizedEmail,
      token: '123456',
      expiresAt: new Date(now.getTime() + 10 * 60 * 1000).toISOString(),
      consumedAt: undefined,
    };
    this.emailChallenges.set(challenge.challengeId, challenge);
    return {
      challengeId: challenge.challengeId,
      expiresAt: challenge.expiresAt,
      delivery: 'email',
    };
  }

  async verifyEmailBind({ internalUserId, challengeId, token, now }) {
    const challenge = this.emailChallenges.get(challengeId);
    if (
      !challenge ||
      challenge.internalUserId !== internalUserId ||
      challenge.token !== token ||
      challenge.consumedAt ||
      new Date(challenge.expiresAt) <= now
    ) {
      const error = new Error('Email binding challenge is invalid');
      error.code = 'EMAIL_BIND_CHALLENGE_INVALID';
      error.statusCode = 400;
      throw error;
    }

    const emailKey = `email:${challenge.normalizedEmail}`;
    const existingInternalUserId = this.identitiesByProviderSubject.get(emailKey);
    if (existingInternalUserId && existingInternalUserId !== internalUserId) {
      const mergeDecision = {
        mergeDecisionId: randomUUID(),
        sourceInternalUserId: internalUserId,
        targetInternalUserId: existingInternalUserId,
        state: 'pending',
        preview: {
          currentWechatUser: this.previewAccount(internalUserId),
          emailUser: this.previewAccount(existingInternalUserId),
          recommendedAction: 'merge_with_confirmation',
        },
      };
      this.mergeDecisions.set(mergeDecision.mergeDecisionId, mergeDecision);
      return {
        bindingState: 'merge_decision_required',
        mergeDecisionId: mergeDecision.mergeDecisionId,
        mergePreview: mergeDecision.preview,
      };
    }

    const account = this.accounts.get(internalUserId);
    account.hasEmailBinding = true;
    account.identities.push({
      provider: 'email',
      providerSubject: challenge.normalizedEmail,
      normalizedEmail: challenge.normalizedEmail,
      isVerified: true,
    });
    this.identitiesByProviderSubject.set(emailKey, internalUserId);
    challenge.consumedAt = now.toISOString();

    return {
      bindingState: 'bound',
      user: {
        internalUserId,
        hasEmailBinding: true,
        hasWechatBinding: account.hasWechatBinding,
      },
    };
  }

  async getMergePreview({ mergeDecisionId }) {
    const decision = this.mergeDecisions.get(mergeDecisionId);
    if (!decision || decision.state !== 'pending') {
      const error = new Error('Merge decision is unavailable');
      error.code = 'DOMAIN_CONFLICT';
      error.statusCode = 404;
      throw error;
    }
    return {
      mergeDecisionId,
      mergePreview: decision.preview,
    };
  }

  async getToday({ internalUserId }) {
    await this.getAccountState({ internalUserId });
    return todayFromPlanAndReports(
      this.planByUser.get(internalUserId) ?? defaultPlan,
      this.reportsByUser.get(internalUserId) ?? defaultReports,
    );
  }

  async getActivePlan({ internalUserId }) {
    await this.getAccountState({ internalUserId });
    return this.planByUser.get(internalUserId) ?? defaultPlan;
  }

  async getReportsOverview({ internalUserId }) {
    await this.getAccountState({ internalUserId });
    return this.reportsByUser.get(internalUserId) ?? defaultReports;
  }

  async getTodayReward({ internalUserId }) {
    await this.getAccountState({ internalUserId });
    return this.rewardByUser.get(internalUserId) ?? defaultReward;
  }

  async claimTodayReward({ internalUserId, now = new Date() }) {
    await this.getAccountState({ internalUserId });
    const current = this.rewardByUser.get(internalUserId) ?? defaultReward;
    if (current.claimedAt) {
      return current;
    }
    const reward = {
      todayDate: current.todayDate,
      rewardId: 'steady-star',
      claimedAt: now.toISOString(),
      asset: {
        rewardId: 'steady-star',
        title: 'Steady Star',
        imageUrl: 'https://example.invalid/rewards/steady-star.webp',
        mimeType: 'image/webp',
      },
    };
    this.rewardByUser.set(internalUserId, reward);
    return reward;
  }

  async getLeaderboard({ internalUserId, metric }) {
    await this.getAccountState({ internalUserId });
    return leaderboardFromReports(
      this.reportsByUser.get(internalUserId) ?? defaultReports,
      metric,
    );
  }

  async startStudySession({ internalUserId, mode }) {
    await this.getAccountState({ internalUserId });
    const question = structuredClone(firstQuestionForMode(mode));
    const session = {
      sessionId: randomUUID(),
      internalUserId,
      mode,
      question,
      startedAt: new Date().toISOString(),
    };
    this.studySessions.set(session.sessionId, session);
    this.latestStudySessionByUser = this.latestStudySessionByUser ?? new Map();
    this.latestStudySessionByUser.set(internalUserId, session.sessionId);
    return {
      session: {
        sessionId: session.sessionId,
        mode,
        totalWords: question.totalQuestions,
        startedAt: session.startedAt,
      },
      currentQuestion: question,
      progress: {
        current: question.questionIndex,
        total: question.totalQuestions,
      },
      answeredQuestions: [],
    };
  }

  async getStudyPayloads({ internalUserId, mode, wordbookId, entrySourceIds = [] }) {
    await this.getAccountState({ internalUserId });
    const primary = studyPayloadsForMode(mode, entrySourceIds, wordbookId);
    const distractors = allStudyPayloads().filter(
      (payload) => !primary.some((item) => item.sourceId === payload.sourceId),
    );
    return {
      entryPayloads: primary,
      distractorPayloads: distractors,
    };
  }

  async recordStudyEvent({ internalUserId, sessionId, eventType, payload, occurredAt }) {
    await this.getAccountState({ internalUserId });
    const timestamp = occurredAt ?? new Date().toISOString();
    this.studyEvents.push({
      internalUserId,
      sessionId,
      eventType,
      payload,
      occurredAt: timestamp,
    });
    if (eventType === 'answer_submitted') {
      this.applyStudyProjection({
        internalUserId,
        mode: payload?.mode,
        question: payload?.question,
        result: payload?.result,
        occurredAt: timestamp,
      });
    }
  }

  async getResumeSessionHint({ internalUserId }) {
    await this.getAccountState({ internalUserId });
    const latestSessionId = this.latestStudySessionByUser?.get(internalUserId);
    const session = latestSessionId
      ? this.studySessions.get(latestSessionId)
      : undefined;
    if (!session || session.completedAt || session.cancelledAt) {
      return { hasResume: false };
    }
    return {
      hasResume: true,
      mode: session.mode,
      current: session.question.questionIndex,
      total: session.question.totalQuestions,
      word: session.question.word,
    };
  }

  async submitStudyAnswer({
    internalUserId,
    questionId,
    response,
    responseTimeMs,
  }) {
    await this.getAccountState({ internalUserId });
    const session = [...this.studySessions.values()]
      .reverse()
      .find(
        (item) =>
          item.internalUserId === internalUserId &&
          item.question.questionId === questionId,
      );
    if (!session) {
      const error = new Error('Study session question is unavailable');
      error.code = 'DOMAIN_CONFLICT';
      error.statusCode = 404;
      throw error;
    }
    const question = session.question;
    const correct = isCorrect(question, response);
    const answeredAt = new Date().toISOString();
    const result = {
      questionId,
      entrySourceId: question.entrySourceId,
      questionType: question.questionType,
      userResponse: response,
      correctAnswer: question.acceptedMeanings[0],
      outcome: correct ? 'correct' : 'incorrect',
      responseTimeMs,
      answeredAt,
    };
    const reports = this.reportsByUser.get(internalUserId) ?? defaultReports;
    this.reportsByUser.set(
      internalUserId,
      applyAnswerToReports(reports, session.mode, correct),
    );
    if (!correct) {
      this.bumpWrongWord(internalUserId, question, answeredAt);
    }
    return {
      result,
      progress: { current: 1, total: 1 },
      answeredQuestions: [{ question, result }],
      isComplete: true,
      summary: {
        totalQuestions: 1,
        correctCount: correct ? 1 : 0,
        accuracyPercent: correct ? 100 : 0,
        completedAt: answeredAt,
      },
      nextAction: 'Return to today',
    };
  }

  async markStudyEntryMastered({
    internalUserId,
    entrySourceId,
    reason = 'mastered',
  }) {
    await this.getAccountState({ internalUserId });
    const session = this.findLatestStudySession(internalUserId);
    const prunedQuestionCount =
      session?.question.entrySourceId === entrySourceId ? 1 : 0;
    const completedAt = new Date().toISOString();
    if (session) {
      session.completedAt = completedAt;
    }
    return {
      entrySourceId,
      entryId: undefined,
      prunedQuestionCount,
      isComplete: true,
      currentQuestion: undefined,
      summary: {
        sessionId: session?.sessionId,
        totalQuestions: 0,
        correctCount: 0,
        fuzzyCorrectCount: 0,
        incorrectCount: 0,
        skippedCount: 0,
        totalWords: 0,
        wrongWordCount: 0,
        accuracyPercent: 0,
        totalTimeMs: 0,
        completedAt,
      },
      nextAction: reason,
      progress: { current: 0, total: 0 },
      answeredQuestions: [],
    };
  }

  async acceptDisputedMeaning({
    internalUserId,
    questionId,
    submittedAnswer = '',
  }) {
    await this.getAccountState({ internalUserId });
    const session = this.findLatestStudySession(internalUserId);
    if (!session || session.question.questionId !== questionId) {
      const error = new Error('Study session question is unavailable');
      error.code = 'DOMAIN_CONFLICT';
      error.statusCode = 404;
      throw error;
    }
    const answeredAt = new Date().toISOString();
    const question = session.question;
    const result = {
      questionId,
      entrySourceId: question.entrySourceId,
      questionType: question.questionType,
      userResponse: submittedAnswer,
      normalizedResponse: submittedAnswer.trim(),
      correctAnswer: submittedAnswer,
      outcome: 'correct',
      responseTimeMs: 0,
      answeredAt,
    };
    return {
      result,
      progress: { current: 1, total: 1 },
      answeredQuestions: [{ question, result }],
      entrySourceId: question.entrySourceId,
      word: question.word,
      acceptedMeaning: submittedAnswer,
    };
  }

  async completeStudySession({ internalUserId, sessionId }) {
    await this.getAccountState({ internalUserId });
    const session = this.studySessions.get(sessionId);
    if (!session || session.internalUserId !== internalUserId) {
      const error = new Error('Study session is unavailable');
      error.code = 'DOMAIN_CONFLICT';
      error.statusCode = 404;
      throw error;
    }
    const completedAt = new Date().toISOString();
    session.completedAt = completedAt;
    return {
      summary: {
        sessionId,
        totalQuestions: 0,
        correctCount: 0,
        fuzzyCorrectCount: 0,
        incorrectCount: 0,
        skippedCount: 0,
        totalWords: 0,
        wrongWordCount: 0,
        accuracyPercent: 0,
        totalTimeMs: 0,
        completedAt,
      },
      nextAction: 'Return to today',
    };
  }

  async cancelStudySession({ internalUserId, sessionId }) {
    await this.getAccountState({ internalUserId });
    const session = this.studySessions.get(sessionId);
    if (session && session.internalUserId === internalUserId) {
      session.cancelledAt = new Date().toISOString();
    }
  }

  async listWrongWords({ internalUserId }) {
    await this.getAccountState({ internalUserId });
    return this.wrongWordsByUser.get(internalUserId) ?? structuredClone(defaultWrongWords);
  }

  async getWrongWordDetail({ internalUserId, entryId }) {
    const words = await this.listWrongWords({ internalUserId });
    const entry = words.find((item) => item.entryId === Number(entryId)) ?? words[0];
    return wrongWordDetailFromEntry(entry);
  }

  async confirmMerge({ mergeDecisionId }) {
    const decision = this.mergeDecisions.get(mergeDecisionId);
    if (!decision || decision.state !== 'pending') {
      const error = new Error('Merge decision is unavailable');
      error.code = 'DOMAIN_CONFLICT';
      error.statusCode = 404;
      throw error;
    }
    decision.state = 'confirmed';
    return {
      mergeDecisionId,
      state: decision.state,
      internalUserId: decision.targetInternalUserId,
    };
  }

  seedEmailAccount(normalizedEmail, overrides = {}) {
    const internalUserId = overrides.internalUserId ?? randomUUID();
    const account = {
      internalUserId,
      primaryIdentity: 'email',
      status: 'active',
      hasEmailBinding: true,
      hasWechatBinding: false,
      phase: 'signed_in_active',
      needsBindDecision: false,
      cloudDataState: 'empty_or_ready',
      identities: [
        {
          provider: 'email',
          providerSubject: normalizedEmail,
          normalizedEmail,
          isVerified: true,
        },
      ],
      hasStudyEvents: overrides.hasStudyEvents ?? true,
      hasPlanConfig: overrides.hasPlanConfig ?? true,
    };
    this.accounts.set(internalUserId, account);
    this.identitiesByProviderSubject.set(`email:${normalizedEmail}`, internalUserId);
    return this.toAccountState(account);
  }

  previewAccount(internalUserId) {
    const account = this.accounts.get(internalUserId);
    return {
      hasStudyEvents: Boolean(account?.hasStudyEvents),
      hasPlanConfig: Boolean(account?.hasPlanConfig),
    };
  }

  bumpWrongWord(internalUserId, question, answeredAt) {
    const words = structuredClone(
      this.wrongWordsByUser.get(internalUserId) ?? defaultWrongWords,
    );
    const existing = words.find((item) => item.word === question.word);
    if (existing) {
      existing.errorCount += 1;
      existing.lastWrongAt = answeredAt;
      existing.priorityScore = Math.min(100, existing.priorityScore + 5);
    } else {
      words.unshift({
        entryId: Date.now(),
        word: question.word,
        meanings: question.acceptedMeanings,
        errorCount: 1,
        lastWrongAt: answeredAt,
        priorityScore: 65,
        hasHint: question.hasHint,
        userHint: question.userHint,
      });
    }
    this.wrongWordsByUser.set(internalUserId, words);
  }

  applyStudyProjection({ internalUserId, mode, question, result, occurredAt }) {
    if (!question || !result) return;
    const positive = result.outcome === 'correct' || result.outcome === 'fuzzyCorrect';
    const reports = this.reportsByUser.get(internalUserId) ?? defaultReports;
    this.reportsByUser.set(
      internalUserId,
      applyAnswerToReports(reports, mode ?? 'review', positive),
    );
    if (result.outcome === 'incorrect' || result.outcome === 'skipped') {
      this.bumpWrongWord(internalUserId, question, occurredAt);
    }
  }

  findLatestStudySession(internalUserId) {
    const latestSessionId = this.latestStudySessionByUser?.get(internalUserId);
    if (latestSessionId) {
      return this.studySessions.get(latestSessionId);
    }
    return [...this.studySessions.values()]
      .reverse()
      .find((item) => item.internalUserId === internalUserId);
  }

  toAccountState(account) {
    return {
      internalUserId: account.internalUserId,
      primaryIdentity: account.primaryIdentity,
      hasEmailBinding: account.hasEmailBinding,
      hasWechatBinding: account.hasWechatBinding,
      phase: account.phase,
      needsBindDecision: account.needsBindDecision,
      cloudDataState: account.cloudDataState,
    };
  }
}

function studyPayloadsForMode(mode, entrySourceIds, wordbookId) {
  const payloads = allStudyPayloads();
  if (entrySourceIds.length > 0) {
    const selected = payloads.filter((payload) =>
      entrySourceIds.includes(payload.sourceId),
    );
    if (selected.length > 0) return selected;
  }
  if (wordbookId) {
    const selected = payloads.filter((payload) => payload.wordbookId === wordbookId);
    if (selected.length > 0) return selected.slice(0, 4);
  }
  if (mode === 'rootAffix') {
    return allRootAffixPayloads().slice(0, 3);
  }
  if (mode === 'wrongWordReinforcement') {
    return payloads.slice(0, 2);
  }
  return payloads.slice(0, 3);
}

function allStudyPayloads() {
  return [
    payload('entry-resilient', 'resilient', 'able to recover quickly', 1),
    payload('entry-meticulous', 'meticulous', 'very careful and precise', 1),
    payload('entry-coherent', 'coherent', 'logical and consistent', 1),
    payload('entry-adapt', 'adapt', 'change to fit new conditions', 1),
    payload('entry-persistent', 'persistent', 'continuing despite difficulty', 1),
  ];
}

function allRootAffixPayloads() {
  return [
    rootPayload('root_affix_shared_re', 're-', '再；重新', 'review, rebuild', '复习, 重建'),
    rootPayload('root_affix_shared_un', 'un-', '不；相反', 'unknown, unable', '未知, 不能'),
    rootPayload('root_affix_medical_cardi', 'cardi-', '心脏', 'cardiology, cardiopulmonary', '心脏病学, 心肺的'),
  ];
}

function payload(sourceId, word, meaning, wordbookId) {
  return {
    sourceId,
    word,
    partOfSpeech: 'adj.',
    frequency: 1,
    phoneticUs: undefined,
    phoneticUk: undefined,
    meaningDetails: [{ pos: 'adj.', meaningCn: meaning, meaningEn: undefined }],
    meanings: [meaning],
    exampleSentence: `The learner used ${word} in a sentence.`,
    exampleTranslation: `${word} example translation`,
    wordbookId,
  };
}

function rootPayload(sourceId, word, meaning, exampleSentence, exampleTranslation) {
  return {
    sourceId,
    word,
    partOfSpeech: 'root',
    frequency: 0,
    phoneticUs: undefined,
    phoneticUk: undefined,
    meaningDetails: [{ pos: 'root', meaningCn: meaning, meaningEn: undefined }],
    meanings: [meaning],
    exampleSentence,
    exampleTranslation,
    wordbookId: undefined,
  };
}
