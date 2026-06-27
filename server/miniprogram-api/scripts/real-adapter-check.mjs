import { RealSupabaseAdapter } from '../src/supabase/real-adapter.js';

class FakeClient {
  constructor() {
    this.calls = [];
    this.accounts = [
      {
        internal_user_id: 'internal-1',
        primary_identity: 'wechat_mp',
        status: 'active',
        supabase_owner_user_id: 'owner-1',
      },
    ];
    this.identities = [
      { provider: 'wechat_mp', is_verified: true },
    ];
    this.sessions = [
      {
        session_id: 'session-1',
        internal_user_id: 'internal-1',
        expires_at: '2099-01-01T00:00:00.000Z',
      },
    ];
    this.plans = [
      {
        plan_id: 'plan-1',
        name: 'Cloud Plan',
        new_words_per_day: 10,
        review_words_per_day: 20,
        mixed_test_per_day: 5,
        wrong_word_test_per_day: 4,
        root_affix_per_day: 2,
        growth_rule_mode: 'shared',
      },
    ];
    this.reports = [
      {
        snapshot_date: '2026-05-20',
        payload_json: {
          totalQuestionsAnswered: 42,
          totalWordsLearned: 12,
          totalStudyDays: 3,
          overallAccuracy: 90,
          streakInfo: { currentStreak: 3, longestStreak: 5 },
          dailySeries: [{ date: '2026-05-20', totalQuestions: 42, correctCount: 38 }],
          modeBreakdown: [{ mode: 'review', totalQuestions: 42, correctCount: 38 }],
        },
      },
    ];
    this.wrongWords = [
      {
        entry_id: 101,
        error_count: 4,
        last_wrong_at: '2026-05-20T00:00:00Z',
        priority_score: 91,
        hint_text: 'cloud hint',
      },
    ];
    this.studyPayloads = [
      {
        entry_id: 201,
        source_id: 'entry-alpha',
        word: 'alpha',
        part_of_speech: 'n',
        frequency: 10,
        phonetic_us: null,
        phonetic_uk: null,
        meanings_json: ['alpha meaning'],
        meaning_details_json: [{ pos: 'n', meaningCn: 'alpha meaning' }],
        example_sentence: 'alpha example',
        example_translation: 'alpha translation',
        wordbook_id: 1,
      },
      {
        entry_id: 202,
        source_id: 'entry-beta',
        word: 'beta',
        part_of_speech: 'n',
        frequency: 9,
        phonetic_us: null,
        phonetic_uk: null,
        meanings_json: ['beta meaning'],
        meaning_details_json: [{ pos: 'n', meaningCn: 'beta meaning' }],
        example_sentence: 'beta example',
        example_translation: 'beta translation',
        wordbook_id: 1,
      },
      {
        entry_id: 301,
        source_id: 'root_affix_shared_re',
        word: 're-',
        part_of_speech: 'root',
        frequency: 0,
        phonetic_us: null,
        phonetic_uk: null,
        meanings_json: ['again'],
        meaning_details_json: [{ pos: 'root', meaningCn: 'again' }],
        example_sentence: 'review, rebuild',
        example_translation: 'review translation',
        wordbook_id: null,
      },
    ];
  }

  async select(table, query) {
    this.calls.push({ op: 'select', table, query });
    if (table === 'miniprogram_sessions') return this.sessions;
    if (table === 'internal_accounts') return this.accounts;
    if (table === 'user_identities') return this.identities;
    if (table === 'plan_configs') return this.plans;
    if (table === 'report_snapshots') return this.reports;
    if (table === 'wrong_word_entries') return this.wrongWords;
    if (table === 'study_entry_payloads') {
      if (query.includes('source_id=like.root_affix_shared_')) {
        return this.studyPayloads.filter((row) => row.source_id.startsWith('root_affix_shared_'));
      }
      if (query.includes('source_id=like.root_affix_medical_')) {
        return this.studyPayloads.filter((row) => row.source_id.startsWith('root_affix_medical_'));
      }
      return this.studyPayloads.filter((row) => !row.source_id.startsWith('root_affix_'));
    }
    return [];
  }

  async update(table, query, patch) {
    this.calls.push({ op: 'update', table, query, patch });
    return [{ ...patch }];
  }

  async insert(table, rows) {
    this.calls.push({ op: 'insert', table, rows });
    if (table === 'internal_accounts') {
      return [{ internal_user_id: 'internal-new' }];
    }
    if (table === 'miniprogram_sessions') {
      return [{ session_id: 'session-new', internal_user_id: rows[0].internal_user_id, expires_at: rows[0].expires_at }];
    }
    return rows;
  }

  async upsert(table, rows, options) {
    this.calls.push({ op: 'upsert', table, rows, options });
    if (table === 'report_snapshots') {
      this.reports = rows.map((row) => ({
        snapshot_date: row.snapshot_date,
        payload_json: row.payload_json,
      }));
    }
    if (table === 'wrong_word_entries') {
      this.wrongWords = rows.map((row) => ({
        entry_id: row.entry_id,
        error_count: row.error_count,
        last_wrong_at: row.last_wrong_at,
        priority_score: row.priority_score,
        hint_text: row.hint_text,
      }));
    }
    return rows;
  }
}

const client = new FakeClient();
const adapter = new RealSupabaseAdapter({ client });

const refreshed = await adapter.refreshMiniprogramSession({
  refreshTokenHash: 'hash-1',
  now: new Date('2026-05-20T00:00:00Z'),
});

if (refreshed.internalUserId !== 'internal-1' || !refreshed.hasWechatBinding) {
  throw new Error('Expected refresh to resolve account state');
}

await adapter.revokeMiniprogramSession({
  sessionId: 'session-1',
  revokedAt: new Date('2026-05-20T00:00:00Z'),
});

const plan = await adapter.getActivePlan({ internalUserId: 'internal-1' });
const reports = await adapter.getReportsOverview({ internalUserId: 'internal-1' });
const today = await adapter.getToday({ internalUserId: 'internal-1' });
const leaderboard = await adapter.getLeaderboard({
  internalUserId: 'internal-1',
  metric: 'weekly',
});
const wrongWords = await adapter.listWrongWords({ internalUserId: 'internal-1' });
const wrongDetail = await adapter.getWrongWordDetail({
  internalUserId: 'internal-1',
  entryId: 101,
});
const payloads = await adapter.getStudyPayloads({
  internalUserId: 'internal-1',
  mode: 'mixedTest',
  wordbookId: 1,
});
const rootAffixPayloads = await adapter.getStudyPayloads({
  internalUserId: 'internal-1',
  mode: 'rootAffix',
  wordbookId: 1,
});
await adapter.recordStudyEvent({
  internalUserId: 'internal-1',
  sessionId: 'rust-session-1',
  eventType: 'answer_submitted',
  payload: {
    mode: 'mixedTest',
    question: { entrySourceId: 'entry-alpha', word: 'alpha', acceptedMeanings: ['alpha meaning'] },
    result: { questionId: 'q1', entrySourceId: 'entry-alpha', outcome: 'incorrect' },
  },
  occurredAt: '2026-05-20T00:00:00Z',
});
const study = await adapter.startStudySession({
  internalUserId: 'internal-1',
  mode: 'newWord',
});
const answer = await adapter.submitStudyAnswer({
  internalUserId: 'internal-1',
  questionId: study.currentQuestion.questionId,
  response: 'easy to break',
  responseTimeMs: 1200,
});

if (!client.calls.some((call) => call.op === 'update' && call.table === 'miniprogram_sessions' && call.patch.revoked_at)) {
  throw new Error('Expected revoke to update miniprogram_sessions.revoked_at');
}

if (plan.name !== 'Cloud Plan' || reports.totalQuestionsAnswered !== 42) {
  throw new Error('Expected real adapter to map plan and report snapshots');
}

if (today.activePlan.name !== 'Cloud Plan' || !leaderboard.currentUser) {
  throw new Error('Expected real adapter to derive today and leaderboard data');
}

if (wrongWords[0].entryId !== 101 || wrongDetail.entryId !== 101) {
  throw new Error('Expected real adapter to map wrong-word rows');
}

if (payloads.entryPayloads[0].sourceId !== 'entry-alpha' || payloads.distractorPayloads.length === 0) {
  throw new Error('Expected real adapter to source study payload rows');
}

if (rootAffixPayloads.entryPayloads[0].sourceId !== 'root_affix_shared_re') {
  throw new Error('Expected real adapter to source shared root-affix payload rows');
}

if (!client.calls.some((call) => call.table === 'study_entry_payloads' && call.query.includes('source_id=like.root_affix_shared_'))) {
  throw new Error('Expected rootAffix mode to query study_entry_payloads by root_affix_shared prefix');
}

if (!client.calls.some((call) => call.op === 'insert' && call.table === 'study_events' && call.rows[0].session_id === 'rust-session-1')) {
  throw new Error('Expected real adapter to record Rust study event');
}

if (!client.calls.some((call) => call.op === 'upsert' && call.table === 'report_snapshots')) {
  throw new Error('Expected real adapter to project Rust answer into report_snapshots');
}

if (!client.calls.some((call) => call.op === 'upsert' && call.table === 'wrong_word_entries')) {
  throw new Error('Expected real adapter to project incorrect Rust answer into wrong_word_entries');
}

if (!answer.isComplete || answer.result.outcome !== 'incorrect') {
  throw new Error('Expected real adapter study submit response');
}

console.log(
  JSON.stringify(
    {
      refreshedUser: refreshed.internalUserId,
      planName: plan.name,
      questions: reports.totalQuestionsAnswered,
      wrongWords: wrongWords.length,
      payloads: payloads.entryPayloads.length,
      rootAffixPayloads: rootAffixPayloads.entryPayloads.length,
      answerOutcome: answer.result.outcome,
      callCount: client.calls.length,
      sessionUpdates: client.calls.filter((call) => call.table === 'miniprogram_sessions' && call.op === 'update').length,
    },
    null,
    2,
  ),
);
