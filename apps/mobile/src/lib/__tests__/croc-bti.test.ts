import {describe, expect, it} from '@jest/globals';
import {
  CROC_BTI_QUESTIONS,
  calculateWeights,
  evaluateCrocBti,
  type CrocBtiAnswer,
} from '../croc-bti';

describe('evaluateCrocBti', () => {
  it('maps strong VINT answers to 鳄骑兵', () => {
    const answers = answerForTraits(['V', 'I', 'N', 'T']);

    const result = evaluateCrocBti(answers);

    expect(result.code).toBe('VINT');
    expect(result.title).toBe('鳄骑兵');
    expect(result.axisScores.vc.strength).toBe('clear');
    expect(result.weights.newWords).toBeGreaterThan(result.weights.review);
    expect(result.weights.mixedTest).toBeGreaterThan(result.weights.contextExamples);
  });

  it('maps strong CORA answers to 鳄渊者', () => {
    const answers = answerForTraits(['C', 'O', 'R', 'A']);

    const result = evaluateCrocBti(answers);

    expect(result.code).toBe('CORA');
    expect(result.title).toBe('鳄渊者');
    expect(result.weights.review).toBeGreaterThan(result.weights.newWords);
    expect(result.weights.contextExamples).toBeGreaterThan(result.weights.activeRecall);
  });

  it('breaks unanswered ties toward the first trait on each axis', () => {
    const result = evaluateCrocBti([]);

    expect(result.code).toBe('VINA');
    expect(result.axisScores.vc.strength).toBe('balanced');
  });
});

describe('calculateWeights', () => {
  it('normalizes recommendations to 100 percent', () => {
    const weights = calculateWeights(['C', 'O', 'R', 'T']);

    expect(Object.values(weights).reduce((sum, value) => sum + value, 0)).toBe(100);
  });
});

function answerForTraits(traits: string[]): CrocBtiAnswer[] {
  return CROC_BTI_QUESTIONS.map(question => ({
    questionId: question.id,
    value: traits.includes(question.positiveTrait) ? 2 : -2,
  }));
}
