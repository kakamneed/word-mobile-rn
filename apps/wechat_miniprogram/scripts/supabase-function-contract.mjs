import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const functionPath = path.resolve(__dirname, '..', '..', '..', 'supabase', 'functions', 'v1', 'index.ts');
const source = fs.readFileSync(functionPath, 'utf8');

const requiredSnippets = [
  "Deno.serve",
  "/v1/auth/wechat-mp/login",
  "jscode2session",
  "WECHAT_MP_APPID",
  "WECHAT_MP_SECRET",
  "createAccessToken",
  "/v1/me",
  "/v1/today",
  "/v1/rewards/today",
  "/v1/rewards/today/claim",
  "/v1/plan/active",
  "/v1/plan/apply-to-today",
  "/v1/wordbooks",
  "setActiveWordbook",
  "study_entry_payloads",
  "loadStudyEntryPayloads",
  "loadRootAffixPayloads",
  "loadMixedTestPayloads",
  "loadWrongWordPayloads",
  "questionsFromPayloads",
  "EMAIL_BIND_REQUIRED",
  "todayModeProgress",
  "resumeHintFromActiveSession",
  "daily_reward_claims",
  "getTodayRewardState",
  "claimTodayReward",
];

for (const snippet of requiredSnippets) {
  if (!source.includes(snippet)) {
    throw new Error(`Supabase function gateway missing ${snippet}`);
  }
}

if (!source.includes("await loadStudyEntryPayloads(")) {
  throw new Error("Study session startup must hydrate questions from study_entry_payloads before fallback");
}

if (!source.includes("fallbackQuestionPayloads(mode, entryCount)")) {
  throw new Error("Fallback study payloads must only use the requested entry count after cloud lookup");
}

if (source.includes("return Math.max(1, Math.min(60")) {
  throw new Error("modeQuestionTotal must preserve a plan value of 0 instead of forcing each mode to 1");
}

if (!source.includes("STUDY_MODE_DISABLED")) {
  throw new Error("Study session startup must reject disabled 0-count modes");
}

if (!source.includes("STUDY_WORD_PAYLOADS_EMPTY")) {
  throw new Error("Authenticated study sessions must expose missing word payloads instead of silently using fallback seeds");
}

if (!source.includes("wordbook_id: 'is.null'")) {
  throw new Error("Root/affix mode must load the cloud root/affix payload pool instead of the active wordbook entries");
}

if (!source.includes("stableDailySortRows(uniqueEntryRows(primaryPool), seed)")) {
  throw new Error("Mixed test mode must use a daily stable randomized selection, not the first ranked/frequent entries");
}

if (!source.includes("rank_in_book: `in.(${ids.join(',')})`")) {
  throw new Error("Wrong-word reinforcement must tolerate legacy Flutter/local entry ids by mapping them through rank_in_book");
}

if (!source.includes("loadWordbookPayloadPool(wordbookId")) {
  throw new Error("Wrong-word reinforcement must fill from the active wordbook when projected wrong ids cannot hydrate payloads");
}

if (!source.includes("legacyWordbookIds")) {
  throw new Error("Wordbook catalog must map legacy mini ids to real Supabase wordbook ids");
}

if (!source.includes("entryId: Number(payload.entry_id)")) {
  throw new Error("Study questions must carry the real payload entry_id for cloud study_word_points sync");
}

if (source.includes("question.entrySourceId.replace(/^\\D+/, '')")) {
  throw new Error("persistStudyAnswer must not derive entry_id from source_id strings like KaoYan_3_1");
}

if (!source.includes("1018327649") || !source.includes("1984431478") || !source.includes("1984433400") || !source.includes("3264070878")) {
  throw new Error("Wordbook catalog must use real study_entry_payloads.wordbook_id values");
}

if (source.includes("if (!context.supabaseUserId) return json(200, defaultPlan);")) {
  throw new Error("savePlan must not silently return the default plan when cloud binding is missing");
}

if (source.includes("modeProgress: {}")) {
  throw new Error("Today endpoint must compute modeProgress from study progress, not return an empty object");
}

if (source.includes("resumeHint: { hasResume: false }")) {
  throw new Error("Today endpoint must compute resumeHint from active sessions");
}

if (!source.includes("rewardCatalog")) {
  throw new Error("Rewards endpoint must use the migrated Flutter reward catalog, not an empty placeholder");
}

if (source.includes("rewardId: undefined, claimedAt: undefined });")) {
  throw new Error("GET /v1/rewards/today must read daily_reward_claims instead of returning an empty placeholder");
}

if (!source.includes("TODAY_TASKS_INCOMPLETE")) {
  throw new Error("Reward claiming must be gated by completed enabled today tasks");
}

if (!source.includes("on_conflict: 'user_id,reward_date'")) {
  throw new Error("Reward claiming must be idempotent per user and day");
}

console.log(JSON.stringify({ functionPath, requiredSnippets: requiredSnippets.length }, null, 2));
