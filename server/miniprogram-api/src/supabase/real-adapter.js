import { SupabaseRestClient } from './rest-client.js';
import {
  defaultPlan,
  defaultReports,
  defaultReward,
  defaultWrongWords,
  firstQuestionForMode,
  isCorrect,
  leaderboardFromReports,
  todayFromPlanAndReports,
  wrongWordDetailFromEntry,
} from '../learning/fixtures.js';

export class RealSupabaseAdapter {
  constructor({ client }) {
    this.client = client;
    this.studySessions = new Map();
  }

  static fromEnv(env = process.env) {
    return new RealSupabaseAdapter({
      client: new SupabaseRestClient({
        url: env.SUPABASE_URL,
        serviceRoleKey: env.SUPABASE_SERVICE_ROLE_KEY,
      }),
    });
  }

  async findOrCreateWechatIdentity({ openid, unionid, client }) {
    const identityKey = encodeURIComponent(openid);
    const existing = await this.client.select(
      'user_identities',
      `?provider=eq.wechat_mp&provider_subject=eq.${identityKey}&select=internal_user_id`,
    );
    if (existing.length > 0) {
      await this.client.update(
        'user_identities',
        `?provider=eq.wechat_mp&provider_subject=eq.${identityKey}`,
        { last_seen_at: new Date().toISOString() },
      );
      return this.getAccountState({ internalUserId: existing[0].internal_user_id });
    }

    const accountRows = await this.client.insert('internal_accounts', [
      {
        primary_identity: 'wechat_mp',
        last_login_at: new Date().toISOString(),
      },
    ]);
    const internalUserId = accountRows[0].internal_user_id;
    await this.client.insert('user_identities', [
      {
        internal_user_id: internalUserId,
        provider: 'wechat_mp',
        provider_subject: openid,
        provider_subject_secondary: unionid ?? null,
        is_verified: true,
        metadata_json: { client },
      },
    ]);
    return this.getAccountState({ internalUserId });
  }

  async issueMiniprogramSession({
    internalUserId,
    refreshTokenHash,
    expiresAt,
    client,
  }) {
    const rows = await this.client.insert('miniprogram_sessions', [
      {
        internal_user_id: internalUserId,
        refresh_token_hash: refreshTokenHash,
        expires_at: expiresAt.toISOString(),
        client_json: client ?? {},
      },
    ]);
    return {
      sessionId: rows[0].session_id,
      internalUserId: rows[0].internal_user_id,
      expiresAt: rows[0].expires_at,
    };
  }

  async getAccountState({ internalUserId }) {
    const accountRows = await this.client.select(
      'internal_accounts',
      `?internal_user_id=eq.${encodeURIComponent(internalUserId)}&select=internal_user_id,primary_identity,status`,
    );
    if (accountRows.length === 0 || accountRows[0].status !== 'active') {
      const error = new Error('Account is unavailable');
      error.code = 'ACCOUNT_DISABLED';
      error.statusCode = 403;
      throw error;
    }
    const identityRows = await this.client.select(
      'user_identities',
      `?internal_user_id=eq.${encodeURIComponent(internalUserId)}&select=provider,is_verified`,
    );
    return {
      internalUserId,
      primaryIdentity: accountRows[0].primary_identity,
      hasEmailBinding: identityRows.some(
        (row) => row.provider === 'email' && row.is_verified,
      ),
      hasWechatBinding: identityRows.some(
        (row) => row.provider === 'wechat_mp' && row.is_verified,
      ),
      phase: 'signed_in_active',
      needsBindDecision: false,
      cloudDataState: 'empty_or_ready',
      supabaseOwnerUserId: accountRows[0].supabase_owner_user_id,
    };
  }

  async getToday({ internalUserId }) {
    return todayFromPlanAndReports(
      await this.getActivePlan({ internalUserId }),
      await this.getReportsOverview({ internalUserId }),
    );
  }

  async getActivePlan({ internalUserId }) {
    const ownerId = await this.resolveSupabaseOwner({ internalUserId });
    if (!ownerId) return defaultPlan;
    const rows = await this.client.select(
      'plan_configs',
      `?user_id=eq.${encodeURIComponent(ownerId)}&select=plan_id,name,new_words_per_day,review_words_per_day,mixed_test_per_day,wrong_word_test_per_day,root_affix_per_day,growth_rule_mode`,
    );
    if (rows.length === 0) return defaultPlan;
    return mapPlanConfig(rows[0]);
  }

  async getReportsOverview({ internalUserId }) {
    const ownerId = await this.resolveSupabaseOwner({ internalUserId });
    if (!ownerId) return defaultReports;
    const rows = await this.client.select(
      'report_snapshots',
      `?user_id=eq.${encodeURIComponent(ownerId)}&select=payload_json,snapshot_date&order=snapshot_date.desc&limit=1`,
    );
    if (rows.length === 0) return defaultReports;
    return mapReportSnapshot(rows[0].payload_json);
  }

  async getTodayReward({ internalUserId }) {
    await this.getAccountState({ internalUserId });
    return defaultReward;
  }

  async claimTodayReward({ internalUserId }) {
    await this.getAccountState({ internalUserId });
    return defaultReward;
  }

  async getLeaderboard({ internalUserId, metric }) {
    return leaderboardFromReports(
      await this.getReportsOverview({ internalUserId }),
      metric,
    );
  }

  async startStudySession({ internalUserId, mode }) {
    const ownerId = await this.resolveSupabaseOwner({ internalUserId });
    const question = structuredClone(firstQuestionForMode(mode));
    const sessionId = crypto.randomUUID();
    this.studySessions.set(sessionId, {
      sessionId,
      internalUserId,
      ownerId,
      mode,
      question,
      startedAt: new Date().toISOString(),
    });
    if (ownerId) {
      await this.client.insert('study_events', [
        {
          user_id: ownerId,
          device_id: crypto.randomUUID(),
          session_id: sessionId,
          event_type: 'session_started',
          payload_json: { mode },
          occurred_at: new Date().toISOString(),
          idempotency_key: `wmp:${sessionId}:started`,
        },
      ]);
    }
    return {
      session: {
        sessionId,
        mode,
        totalWords: question.totalQuestions,
        startedAt: new Date().toISOString(),
      },
      currentQuestion: question,
      progress: {
        current: question.questionIndex,
        total: question.totalQuestions,
      },
      answeredQuestions: [],
    };
  }

  async getStudyPayloads({
    internalUserId,
    mode,
    wordbookId,
    entrySourceIds = [],
    limit = 8,
  }) {
    const ownerId = await this.resolveSupabaseOwner({ internalUserId });
    if (!ownerId) return { entryPayloads: [], distractorPayloads: [] };
    const requested = Array.isArray(entrySourceIds) ? entrySourceIds.filter(Boolean) : [];
    const entryRows = requested.length > 0
      ? await this.selectStudyPayloadRowsBySourceIds(requested)
      : mode === 'rootAffix'
        ? await this.selectRootAffixPayloadRows({ wordbookId, limit })
      : await this.selectStudyPayloadRows({ wordbookId, limit: mode === 'newWord' ? limit : Math.max(1, limit) });
    const entryPayloads = entryRows.map(mapStudyPayloadRow);
    const excluded = new Set(entryPayloads.map((payload) => payload.sourceId));
    const distractorRows = await this.selectStudyPayloadRows({
      wordbookId,
      limit: 12,
      excludeSourceIds: excluded,
    });
    return {
      entryPayloads,
      distractorPayloads: distractorRows.map(mapStudyPayloadRow),
    };
  }

  async recordStudyEvent({ internalUserId, sessionId, eventType, payload, occurredAt }) {
    const ownerId = await this.resolveSupabaseOwner({ internalUserId });
    if (!ownerId) return;
    const timestamp = occurredAt ?? new Date().toISOString();
    await this.client.insert('study_events', [
      {
        user_id: ownerId,
        device_id: crypto.randomUUID(),
        session_id: sessionId,
        event_type: eventType,
        payload_json: payload ?? {},
        occurred_at: timestamp,
        idempotency_key: `wmp:${sessionId}:${eventType}:${eventSequenceKey(payload)}`,
      },
    ]);
    if (eventType === 'answer_submitted') {
      await this.applyStudyProjection({
        ownerId,
        mode: payload?.mode,
        question: payload?.question,
        result: payload?.result,
        occurredAt: timestamp,
      });
    }
  }

  async applyStudyProjection({ ownerId, mode, question, result, occurredAt }) {
    if (!question || !result) return;
    await this.upsertReportProjection({ ownerId, mode, result, occurredAt });
    if (result.outcome === 'incorrect' || result.outcome === 'skipped') {
      await this.upsertWrongWordProjection({ ownerId, question, result, occurredAt });
    }
  }

  async upsertReportProjection({ ownerId, mode, result, occurredAt }) {
    const today = occurredAt.slice(0, 10);
    const current = await this.getReportsOverviewByOwner({ ownerId });
    const positive = result.outcome === 'correct' || result.outcome === 'fuzzyCorrect';
    const next = projectReportAnswer(current, mode ?? 'review', positive, today);
    await this.client.upsert(
      'report_snapshots',
      [
        {
          user_id: ownerId,
          snapshot_date: today,
          payload_json: next,
          updated_at: new Date().toISOString(),
        },
      ],
      { onConflict: 'user_id,snapshot_date' },
    );
  }

  async upsertWrongWordProjection({ ownerId, question, result, occurredAt }) {
    const entry = await this.resolveStudyPayloadEntry(question.entrySourceId);
    const entryId = entry?.entry_id ?? Number(question.entrySourceId);
    if (!Number.isFinite(entryId)) return;
    const existingRows = await this.client.select(
      'wrong_word_entries',
      `?user_id=eq.${encodeURIComponent(ownerId)}&entry_id=eq.${encodeURIComponent(entryId)}&select=entry_id,error_count,priority_score,hint_text,hint_source,hint_updated_at`,
    );
    const existing = existingRows[0];
    const errorCount = (existing?.error_count ?? 0) + 1;
    const priorityScore = Math.min(
      100,
      Number(existing?.priority_score ?? 60) + (result.outcome === 'skipped' ? 8 : 5),
    );
    await this.client.upsert(
      'wrong_word_entries',
      [
        {
          user_id: ownerId,
          entry_id: entryId,
          error_count: errorCount,
          last_wrong_at: occurredAt,
          priority_score: priorityScore,
          hint_text: existing?.hint_text ?? '',
          hint_source: existing?.hint_source ?? '',
          hint_updated_at: existing?.hint_updated_at ?? null,
          projection_version: 1,
          updated_at: new Date().toISOString(),
        },
      ],
      { onConflict: 'user_id,entry_id' },
    );
  }

  async getReportsOverviewByOwner({ ownerId }) {
    const rows = await this.client.select(
      'report_snapshots',
      `?user_id=eq.${encodeURIComponent(ownerId)}&select=payload_json,snapshot_date&order=snapshot_date.desc&limit=1`,
    );
    if (rows.length === 0) return defaultReports;
    return mapReportSnapshot(rows[0].payload_json);
  }

  async resolveStudyPayloadEntry(sourceId) {
    const rows = await this.client.select(
      'study_entry_payloads',
      `?source_id=eq.${encodeURIComponent(sourceId)}&select=entry_id,word,meanings_json&limit=1`,
    );
    return rows[0];
  }

  async submitStudyAnswer({
    internalUserId,
    questionId,
    response,
    responseTimeMs,
  }) {
    const session = [...this.studySessions.values()]
      .reverse()
      .find(
        (item) =>
          item.internalUserId === internalUserId &&
          item.question.questionId === questionId,
      );
    const question = session?.question ?? firstQuestionForMode('review');
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
    if (session?.ownerId) {
      await this.client.insert('study_events', [
        {
          user_id: session.ownerId,
          device_id: crypto.randomUUID(),
          session_id: session.sessionId,
          event_type: 'answer_submitted',
          payload_json: {
            mode: session.mode,
            question,
            result,
          },
          occurred_at: answeredAt,
          idempotency_key: `wmp:${session.sessionId}:${questionId}:answer`,
        },
      ]);
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

  async listWrongWords({ internalUserId }) {
    const ownerId = await this.resolveSupabaseOwner({ internalUserId });
    if (!ownerId) return defaultWrongWords;
    const rows = await this.client.select(
      'wrong_word_entries',
      `?user_id=eq.${encodeURIComponent(ownerId)}&select=entry_id,error_count,last_wrong_at,priority_score,hint_text&order=priority_score.desc&limit=50`,
    );
    if (rows.length === 0) return defaultWrongWords;
    return rows.map(mapWrongWordEntry);
  }

  async getWrongWordDetail({ internalUserId, entryId }) {
    const words = await this.listWrongWords({ internalUserId });
    return wrongWordDetailFromEntry(
      words.find((item) => item.entryId === Number(entryId)) ?? words[0],
    );
  }

  async selectStudyPayloadRowsBySourceIds(sourceIds) {
    const encoded = sourceIds.map((id) => `"${String(id).replaceAll('"', '\\"')}"`).join(',');
    return this.client.select(
      'study_entry_payloads',
      `?source_id=in.(${encoded})&is_active=eq.true&select=${studyPayloadSelect()}`,
    );
  }

  async selectStudyPayloadRows({ wordbookId, limit, excludeSourceIds = new Set() }) {
    const filters = ['is_active=eq.true'];
    if (wordbookId != null) {
      filters.push(`wordbook_id=eq.${encodeURIComponent(wordbookId)}`);
    }
    if (excludeSourceIds.size > 0) {
      const excluded = [...excludeSourceIds]
        .map((id) => `"${String(id).replaceAll('"', '\\"')}"`)
        .join(',');
      filters.push(`source_id=not.in.(${excluded})`);
    }
    filters.push(`select=${studyPayloadSelect()}`);
    filters.push('order=rank_in_book.asc,frequency.desc');
    filters.push(`limit=${limit}`);
    return this.client.select('study_entry_payloads', `?${filters.join('&')}`);
  }

  async selectRootAffixPayloadRows({ wordbookId, limit }) {
    const prefixes = rootAffixPrefixesForWordbook(wordbookId);
    const rows = [];
    for (const prefix of prefixes) {
      const filters = [
        'is_active=eq.true',
        `source_id=like.${encodeURIComponent(`${prefix}%`)}`,
        `select=${studyPayloadSelect()}`,
        'order=rank_in_book.asc,source_id.asc',
        `limit=${limit}`,
      ];
      rows.push(...await this.client.select('study_entry_payloads', `?${filters.join('&')}`));
    }
    return dedupeRowsBySourceId(rows).slice(0, limit);
  }

  async resolveSupabaseOwner({ internalUserId }) {
    const accountRows = await this.client.select(
      'internal_accounts',
      `?internal_user_id=eq.${encodeURIComponent(internalUserId)}&select=supabase_owner_user_id`,
    );
    return accountRows[0]?.supabase_owner_user_id;
  }

  async refreshMiniprogramSession({ refreshTokenHash, now }) {
    const rows = await this.client.select(
      'miniprogram_sessions',
      `?refresh_token_hash=eq.${encodeURIComponent(refreshTokenHash)}&revoked_at=is.null&select=session_id,internal_user_id,expires_at`,
    );
    if (rows.length === 0 || new Date(rows[0].expires_at) <= now) {
      const error = new Error('Refresh session is invalid or expired');
      error.code = 'SESSION_EXPIRED';
      error.statusCode = 401;
      throw error;
    }
    await this.client.update(
      'miniprogram_sessions',
      `?session_id=eq.${encodeURIComponent(rows[0].session_id)}`,
      { last_seen_at: now.toISOString() },
    );
    return this.getAccountState({ internalUserId: rows[0].internal_user_id });
  }

  async revokeMiniprogramSession({ sessionId, revokedAt }) {
    await this.client.update(
      'miniprogram_sessions',
      `?session_id=eq.${encodeURIComponent(sessionId)}`,
      { revoked_at: revokedAt.toISOString() },
    );
  }

  async startEmailBind() {
    throw new Error('RealSupabaseAdapter.startEmailBind pending');
  }

  async verifyEmailBind() {
    throw new Error('RealSupabaseAdapter.verifyEmailBind pending');
  }

  async getMergePreview() {
    throw new Error('RealSupabaseAdapter.getMergePreview pending');
  }

  async confirmMerge() {
    throw new Error('RealSupabaseAdapter.confirmMerge pending');
  }
}

function mapWrongWordEntry(row) {
  return {
    entryId: row.entry_id,
    word: `entry-${row.entry_id}`,
    meanings: ['Review this word'],
    errorCount: row.error_count ?? 0,
    lastWrongAt: row.last_wrong_at,
    priorityScore: Number(row.priority_score ?? 0),
    hasHint: Boolean(row.hint_text),
    userHint: row.hint_text || undefined,
  };
}

function studyPayloadSelect() {
  return [
    'entry_id',
    'source_id',
    'word',
    'part_of_speech',
    'frequency',
    'phonetic_us',
    'phonetic_uk',
    'meanings_json',
    'meaning_details_json',
    'example_sentence',
    'example_translation',
    'wordbook_id',
  ].join(',');
}

function mapStudyPayloadRow(row) {
  return {
    sourceId: row.source_id,
    word: row.word,
    partOfSpeech: row.part_of_speech ?? undefined,
    frequency: Number(row.frequency ?? 0),
    phoneticUs: row.phonetic_us ?? undefined,
    phoneticUk: row.phonetic_uk ?? undefined,
    meaningDetails: Array.isArray(row.meaning_details_json)
      ? row.meaning_details_json.map((meaning) => ({
          pos: meaning.pos ?? '',
          meaningCn: meaning.meaningCn ?? meaning.meaning_cn ?? '',
          meaningEn: meaning.meaningEn ?? meaning.meaning_en ?? undefined,
        }))
      : [],
    meanings: Array.isArray(row.meanings_json) ? row.meanings_json : [],
    exampleSentence: row.example_sentence ?? undefined,
    exampleTranslation: row.example_translation ?? undefined,
    wordbookId: row.wordbook_id ?? undefined,
  };
}

function rootAffixPrefixesForWordbook(wordbookId) {
  if (Number(wordbookId) === 4) return ['root_affix_medical_'];
  if ([1, 2, 3].includes(Number(wordbookId))) return ['root_affix_shared_'];
  return ['root_affix_shared_', 'root_affix_medical_'];
}

function dedupeRowsBySourceId(rows) {
  const seen = new Set();
  return rows.filter((row) => {
    if (seen.has(row.source_id)) return false;
    seen.add(row.source_id);
    return true;
  });
}

function eventSequenceKey(payload) {
  if (payload?.result?.questionId) return payload.result.questionId;
  if (payload?.summary?.completedAt) return payload.summary.completedAt;
  return 'event';
}

function mapPlanConfig(row) {
  return {
    id: row.plan_id,
    name: row.name ?? 'Default plan',
    newWordsPerDay: row.new_words_per_day ?? 0,
    reviewWordsPerDay: row.review_words_per_day ?? 0,
    mixedTestPerDay: row.mixed_test_per_day ?? 0,
    wrongWordTestPerDay: row.wrong_word_test_per_day ?? 0,
    rootAffixPerDay: row.root_affix_per_day ?? 0,
    growthRuleEnabled: Boolean(row.growth_rule_mode),
  };
}

function mapReportSnapshot(payload) {
  return {
    totalStudyDays: payload.totalStudyDays ?? payload.total_study_days ?? 0,
    totalWordsLearned: payload.totalWordsLearned ?? payload.total_words_learned ?? 0,
    totalQuestionsAnswered:
      payload.totalQuestionsAnswered ?? payload.total_questions_answered ?? 0,
    overallAccuracy: payload.overallAccuracy ?? payload.overall_accuracy ?? 0,
    streakInfo: payload.streakInfo ?? payload.streak_info ?? {
      currentStreak: 0,
      longestStreak: 0,
    },
    dailySeries: payload.dailySeries ?? payload.daily_series ?? [],
    modeBreakdown: payload.modeBreakdown ?? payload.mode_breakdown ?? [],
  };
}

function projectReportAnswer(reports, mode, positive, today) {
  const next = structuredClone(reports);
  const previousQuestions = Number(next.totalQuestionsAnswered ?? 0);
  const previousCorrect = Math.round(
    (Number(next.overallAccuracy ?? 0) / 100) * previousQuestions,
  );
  const nextQuestions = previousQuestions + 1;
  const nextCorrect = previousCorrect + (positive ? 1 : 0);
  next.totalQuestionsAnswered = nextQuestions;
  next.overallAccuracy = nextQuestions > 0
    ? Math.round((nextCorrect / nextQuestions) * 100)
    : 0;

  const dailySeries = Array.isArray(next.dailySeries) ? [...next.dailySeries] : [];
  const dailyIndex = dailySeries.findIndex((row) => row.date === today);
  if (dailyIndex >= 0) {
    dailySeries[dailyIndex] = {
      ...dailySeries[dailyIndex],
      totalQuestions: Number(dailySeries[dailyIndex].totalQuestions ?? 0) + 1,
      correctCount: Number(dailySeries[dailyIndex].correctCount ?? 0) + (positive ? 1 : 0),
    };
  } else {
    dailySeries.push({
      date: today,
      totalQuestions: 1,
      correctCount: positive ? 1 : 0,
    });
  }
  next.dailySeries = dailySeries;

  const modeBreakdown = Array.isArray(next.modeBreakdown) ? [...next.modeBreakdown] : [];
  const modeIndex = modeBreakdown.findIndex((row) => row.mode === mode);
  if (modeIndex >= 0) {
    modeBreakdown[modeIndex] = {
      ...modeBreakdown[modeIndex],
      totalQuestions: Number(modeBreakdown[modeIndex].totalQuestions ?? 0) + 1,
      correctCount: Number(modeBreakdown[modeIndex].correctCount ?? 0) + (positive ? 1 : 0),
    };
  } else {
    modeBreakdown.push({
      mode,
      totalQuestions: 1,
      correctCount: positive ? 1 : 0,
    });
  }
  next.modeBreakdown = modeBreakdown;
  return next;
}
