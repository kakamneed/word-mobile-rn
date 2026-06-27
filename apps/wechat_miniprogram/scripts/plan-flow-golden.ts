import { mockPlan } from '../src/sdk/mockData';
import { buildPlanInput } from '../src/sdk/planFlow';

const input = buildPlanInput(mockPlan, {
  newWordsPerDay: 23,
  questionTypeWeightsByMode: mockPlan.questionTypeWeightsByMode,
});

if (input.newWordsPerDay !== 20) {
  throw new Error(`Expected new-word count to round down to 20, got ${input.newWordsPerDay}`);
}

if (!input.questionTypeWeightsByMode?.review) {
  throw new Error('Expected Plan payload to preserve questionTypeWeightsByMode');
}

if (!input.sharedGrowthRule || !input.growthRulesByMode) {
  throw new Error('Expected growth rule payload fields');
}

console.log(JSON.stringify({ newWordsPerDay: input.newWordsPerDay, hasWeights: true }, null, 2));
