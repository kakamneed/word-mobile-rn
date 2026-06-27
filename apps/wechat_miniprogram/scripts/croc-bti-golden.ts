import {
  clampCrocBtiAnswer,
  crocBtiPlanInputForDailyMinutes,
  crocBtiQuestions,
  evaluateCrocBti,
  hasCompleteCrocBtiAnswers,
  normalizeCrocBtiQuestionTypeWeightsByMode,
} from '../src/sdk/crocBti';
import { MiniProgramSdk } from '../src/sdk/client';

declare const require: (id: string) => any;
declare const __dirname: string;

const { existsSync } = require('fs');
const { join } = require('path');

if (hasCompleteCrocBtiAnswers({})) {
  throw new Error('Expected fresh Croc BTI state to have no complete answers');
}

const answers = Object.fromEntries(
  crocBtiQuestions.map((question) => [
    question.id,
    question.positiveTrait === 'V' ||
    question.positiveTrait === 'I' ||
    question.positiveTrait === 'N' ||
    question.positiveTrait === 'A'
      ? 2
      : -2,
  ]),
);

const result = evaluateCrocBti(answers);
const planInput = crocBtiPlanInputForDailyMinutes(result.weights, 40);

if (result.code !== 'VINA') {
  throw new Error(`Expected VINA, got ${result.code}`);
}

if (planInput.newWordsPerDay <= 0 || planInput.reviewWordsPerDay <= 0) {
  throw new Error(`Invalid plan input: ${JSON.stringify(planInput)}`);
}

if (!hasCompleteCrocBtiAnswers(answers)) {
  throw new Error('Expected complete answer set');
}

if (hasCompleteCrocBtiAnswers({ [crocBtiQuestions[0].id]: 0 })) {
  throw new Error('Expected partial Croc BTI answers not to be complete');
}

if (clampCrocBtiAnswer(100) !== 2 || clampCrocBtiAnswer(-100) !== -2) {
  throw new Error('Expected answer clamp to -2..2');
}

const totalQuestions = Object.values(planInput).reduce((sum, value) => sum + value, 0);
if (totalQuestions !== 160) {
  throw new Error(`Expected daily minutes to create 160 questions, got ${totalQuestions}`);
}

if (planInput.newWordsPerDay % 4 !== 0) {
  throw new Error(`Expected newWordsPerDay multiple of four: ${planInput.newWordsPerDay}`);
}

for (const [mode, weights] of Object.entries(result.questionTypeWeightsByMode)) {
  const total = Object.values(weights).reduce((sum, value) => sum + value, 0);
  if (total !== 100) {
    throw new Error(`Expected ${mode} weights to sum to 100, got ${total}`);
  }
}

const normalized = normalizeCrocBtiQuestionTypeWeightsByMode({
  newWord: { enToCnChoice: 100 },
  rootAffix: { glossToRootInput: 100 },
  review: { enToCnChoice: 50, cnToEnChoice: 50 },
});

if ('newWord' in normalized || 'rootAffix' in normalized) {
  throw new Error('newWord/rootAffix should be excluded from question-type customization');
}

if (JSON.stringify(result).includes('cnToEnInput')) {
  throw new Error('cnToEnInput should not be emitted');
}

const expectedAssets = [
  'scroll_master_croc.jpg',
  'cavalry_croc.jpg',
  'stele_croc.jpg',
  'armor_guard_croc.jpg',
  'chanter_croc.jpg',
  'swordsman_croc.jpg',
  'classic_croc.jpg',
  'forgemaster_croc.jpg',
  'book_guest_bu_e_ke.jpg',
  'ranger_croc.jpg',
  'hermit_croc.jpg',
  'alligator_jade.jpg',
  'bard_croc.jpg',
  'battle_mage_pencil_croc.jpg',
  'stargazer_croc.jpg',
  'correction_officer_croc.jpg',
];

for (const asset of expectedAssets) {
  const assetPath = join(__dirname, '..', 'src', 'assets', 'croc_bti_compressed', asset);
  if (!existsSync(assetPath)) {
    throw new Error(`Expected profile asset path to exist: ${assetPath}`);
  }
}

async function runSdkChecks() {
  const sdk = new MiniProgramSdk();
  const beforeToday = await sdk.today.getTodayHomeState();
  await sdk.crocBti.saveProfile({
    profile: {
      resultCode: result.code,
      title: result.title,
      summary: result.summary,
      advice: result.advice,
      assetPath: result.assetPath,
      flavor: result.flavor,
      answers,
      axisScores: result.axisScores,
      weights: result.weights,
      planInput,
      questionTypeWeightsByMode: result.questionTypeWeightsByMode,
      dailyLearningMinutes: 40,
      growthRuleEnabled: true,
      source: 'croc_bti',
      version: 1,
      evaluatedAt: new Date().toISOString(),
    },
  });
  const profile = await sdk.crocBti.getProfile();
  if (!profile || profile.resultCode !== result.code) {
    throw new Error('Expected Croc BTI profile to persist through SDK');
  }
  const plan = await sdk.plan.getActivePlan();
  await sdk.plan.savePlan({
    planId: plan.id,
    input: {
      name: result.title,
      ...planInput,
      questionTypeWeightsByMode: result.questionTypeWeightsByMode,
    },
  });
  await sdk.plan.applySavedPlanToToday();
  const afterToday = await sdk.today.getTodayHomeState();
  if (afterToday.activePlan?.name !== result.title) {
    throw new Error('Expected applying Croc BTI plan to update Today active plan');
  }
  if (afterToday.dailyProgress.completedTasks !== beforeToday.dailyProgress.completedTasks) {
    throw new Error('Expected Croc BTI plan apply not to mark Today study tasks complete');
  }
}

runSdkChecks()
  .then(() =>
    console.log(
      JSON.stringify(
        {
          code: result.code,
          title: result.title,
          planInput,
        },
        null,
        2,
      ),
    ),
  )
  .catch((error) => {
    console.error(error);
    throw error;
  });
