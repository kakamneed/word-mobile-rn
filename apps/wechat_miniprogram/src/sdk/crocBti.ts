import { PlanSummary } from './types';

export type CrocBtiCode = string;
export type CrocBtiAxis = 'vc' | 'io' | 'nr' | 'at';
export type CrocBtiAnswer = -2 | -1 | 0 | 1 | 2;

export interface CrocBtiQuestion {
  id: string;
  axis: CrocBtiAxis;
  positiveTrait: string;
  negativeTrait: string;
  text: string;
}

export interface CrocBtiAxisScore {
  score: number;
  selectedTrait: string;
  strength: 'soft' | 'clear' | 'strong';
}

export interface CrocBtiResult {
  code: CrocBtiCode;
  title: string;
  summary: string;
  advice: string;
  assetPath: string;
  flavor: string;
  axisScores: Record<CrocBtiAxis, CrocBtiAxisScore>;
  weights: Record<string, number>;
  planInput: Record<string, number>;
  questionTypeWeightsByMode: Record<string, Record<string, number>>;
}

export const crocBtiQuestions: CrocBtiQuestion[] = [
  { id: 'vc-word-volume', axis: 'vc', positiveTrait: 'V', negativeTrait: 'C', text: '我认为英语学习中，单词量比语法、语感、口语等更能决定上限。' },
  { id: 'vc-many-unknowns', axis: 'vc', positiveTrait: 'V', negativeTrait: 'C', text: '如果一篇文章里生词很多，即使句子结构不难，我也会明显读不下去。' },
  { id: 'vc-build-vocab-first', axis: 'vc', positiveTrait: 'V', negativeTrait: 'C', text: '我更愿意先把核心词汇量堆起来，再谈阅读和表达。' },
  { id: 'vc-context-guessing', axis: 'vc', positiveTrait: 'C', negativeTrait: 'V', text: '即使不认识某些词，我也常常能通过上下文猜出大概意思。' },
  { id: 'vc-learn-in-sentences', axis: 'vc', positiveTrait: 'C', negativeTrait: 'V', text: '我更愿意在文章、对话或例句里学词，而不是单独背词表。' },
  { id: 'vc-usage-over-meaning', axis: 'vc', positiveTrait: 'C', negativeTrait: 'V', text: '我觉得掌握一个词怎么用，比单纯记住它的中文意思更重要。' },
  { id: 'io-recognize-enough', axis: 'io', positiveTrait: 'I', negativeTrait: 'O', text: '对我来说，看到英文能理解中文意思，比看到中文想起英文更重要。' },
  { id: 'io-reading-goal', axis: 'io', positiveTrait: 'I', negativeTrait: 'O', text: '我主要的英语目标是阅读、听懂、考试理解，而不是主动表达。' },
  { id: 'io-passive-ok', axis: 'io', positiveTrait: 'I', negativeTrait: 'O', text: '一个词我只要能认出来，就算暂时不会主动使用，也可以接受。' },
  { id: 'io-not-mastered', axis: 'io', positiveTrait: 'O', negativeTrait: 'I', text: '如果我知道中文意思却想不起对应英文，我会觉得这个词还没真正掌握。' },
  { id: 'io-use-in-writing', axis: 'io', positiveTrait: 'O', negativeTrait: 'I', text: '我希望学习后能在写作、口语或造句中主动用出新词。' },
  { id: 'io-cn-to-en', axis: 'io', positiveTrait: 'O', negativeTrait: 'I', text: '我更喜欢中文提示回忆英文，而不是只做英文选中文。' },
  { id: 'nr-new-progress', axis: 'nr', positiveTrait: 'N', negativeTrait: 'R', text: '我喜欢每天学比较多的新词，这会让我有明显进步感。' },
  { id: 'nr-review-boring', axis: 'nr', positiveTrait: 'N', negativeTrait: 'R', text: '如果几天没学新词，只复习旧词，我会觉得效率不高。' },
  { id: 'nr-want-next', axis: 'nr', positiveTrait: 'N', negativeTrait: 'R', text: '遇到熟悉词反复出现，我容易不耐烦，想尽快进入新内容。' },
  { id: 'nr-slower-solid', axis: 'nr', positiveTrait: 'R', negativeTrait: 'N', text: '我宁愿每天少学一点，也希望学过的词能记得更牢。' },
  { id: 'nr-repeat-forgotten', axis: 'nr', positiveTrait: 'R', negativeTrait: 'N', text: '如果一个词反复忘，我希望系统持续安排它，而不是很快放过。' },
  { id: 'nr-review-long-term', axis: 'nr', positiveTrait: 'R', negativeTrait: 'N', text: '我觉得复习和错题整理比追求新词数量更能带来长期提升。' },
  { id: 'at-examples', axis: 'at', positiveTrait: 'A', negativeTrait: 'T', text: '我学单词时喜欢看例句、搭配、近义词差异。' },
  { id: 'at-explain-needed', axis: 'at', positiveTrait: 'A', negativeTrait: 'T', text: '如果只是刷题而没有解释，我会觉得学得不踏实。' },
  { id: 'at-understand-first', axis: 'at', positiveTrait: 'A', negativeTrait: 'T', text: '我更喜欢先理解词义和用法，再接受测试。' },
  { id: 'at-test-reveals', axis: 'at', positiveTrait: 'T', negativeTrait: 'A', text: '我喜欢用测试来发现自己到底会不会，而不是凭感觉判断。' },
  { id: 'at-mixed-focus', axis: 'at', positiveTrait: 'T', negativeTrait: 'A', text: '错题、限时、混合测试会让我更专注，也更容易记住。' },
  { id: 'at-ok-wrong', axis: 'at', positiveTrait: 'T', negativeTrait: 'A', text: '我不介意一开始错很多，只要测试能帮我快速暴露薄弱点。' },
];

const axisTraits: Record<CrocBtiAxis, [string, string]> = {
  vc: ['V', 'C'],
  io: ['I', 'O'],
  nr: ['N', 'R'],
  at: ['A', 'T'],
};

const profileAssets: Record<string, string> = {
  VINA: 'scroll_master_croc.jpg',
  VINT: 'cavalry_croc.jpg',
  VIRA: 'stele_croc.jpg',
  VIRT: 'armor_guard_croc.jpg',
  VONA: 'chanter_croc.jpg',
  VONT: 'swordsman_croc.jpg',
  VORA: 'classic_croc.jpg',
  VORT: 'forgemaster_croc.jpg',
  CINA: 'book_guest_bu_e_ke.jpg',
  CINT: 'ranger_croc.jpg',
  CIRA: 'hermit_croc.jpg',
  CIRT: 'alligator_jade.jpg',
  CONA: 'bard_croc.jpg',
  CONT: 'battle_mage_pencil_croc.jpg',
  CORA: 'stargazer_croc.jpg',
  CORT: 'correction_officer_croc.jpg',
};

const profileText: Record<string, { title: string; summary: string; advice: string; flavor: string }> = {
  VINA: { title: '鳄卷师', summary: '你适合一边扩充词库，一边把词义、例句和搭配卷进自己的知识卷轴里。', advice: '每天保留稳定新词量，但别只看列表；给例句和搭配留出固定时间。', flavor: '卷轴一摊，词义、例句和搭配都被它卷进随身小本本里。' },
  VINT: { title: '鳄骑兵', summary: '你适合用新词推进获得成长感，再用测试快速确认掌握度。', advice: '可以保持较高新词量，但要给混合测试和错题复盘留出硬性权重。', flavor: '冲锋先看新词，回头再用混测点名验收。' },
  VIRA: { title: '鳄碑客', summary: '你适合慢学深记，把词义像刻碑一样沉淀下来。', advice: '少量新词、足量复习和例句理解会比猛冲更适合你。', flavor: '慢慢刻、反复看，把词义刻成不会被浪冲走的碑文。' },
  VIRT: { title: '鳄甲卫', summary: '你重视词汇根基，也需要稳定复习来守住已经学过的内容。', advice: '新词不要完全停，但复习和错题应该成为你的主防线。', flavor: '先把旧词防线守住，再稳稳扩张词库边境。' },
  VONA: { title: '鳄咏者', summary: '你学词是为了说出、写出和表达出来，适合把新词变成可用语言。', advice: '新词学习后马上接造句、中文回忆英文和搭配练习，效果会更明显。', flavor: '学词不是收藏，是要念出来、写出来、用出来。' },
  VONT: { title: '鳄剑士', summary: '你适合把词汇当成可拔出的武器，用主动回忆检验是否真正掌握。', advice: '中译英、拼写和混合测试应该占比更高，避免停留在看懂。', flavor: '拔剑就测，靠主动回忆把“看懂”削成“真会”。' },
  VORA: { title: '古风小鳄', summary: '你适合精修词的用法、搭配和表达质感，走少而精的路线。', advice: '降低新词刺激感，多做主动输出和用法辨析。', flavor: '少而精地修炼用法，讲究一个词有词的身段。' },
  VORT: { title: '鳄锻师', summary: '你适合通过反复锻打把词汇压实，越练越硬。', advice: '错题、间隔复习和主动回忆是你的主炉火，新词量需要服从复习质量。', flavor: '错题是炉火，复习是锤点，越敲越结实。' },
  CINA: { title: '不鳄客', summary: '你适合在文章和例句里自然吸收，靠语境把词慢慢养熟。', advice: '用语境学习承接新词，再用轻量测试确认自己没有误读。', flavor: '进语境如进茶馆，慢慢把词泡开。' },
  CINT: { title: '鳄游侠', summary: '你擅长在上下文里穿行，适合阅读中认词和快速判断。', advice: '多做语境阅读和混合选择题，但要防止猜对后以为已经掌握。', flavor: '在上下文里穿行，线索一闪就能嗅到答案。' },
  CIRA: { title: '鳄隐士', summary: '你不急着刷量，适合靠长期语境和规律复习养出稳定语感。', advice: '低压复习、例句理解和少量新词会让你更持久。', flavor: '不争一日之功，靠长期语境和复习把语感养肥。' },
  CIRT: { title: '全员鳄玉', summary: '你擅长推理、排除和语境判断，适合考试型混合训练。', advice: '混合测试和错题复盘能放大优势，同时补上主动记忆漏洞。', flavor: '线索、排除和复盘都要安排得明明白白。' },
  CONA: { title: '吟游鳄', summary: '你适合在真实表达和语境里学习，让词汇自然长成语言能力。', advice: '多做造句、复述和例句改写，新词量不必太激进。', flavor: '单词从解释里跳出来，变成能说能写的表达。' },
  CONT: { title: '鳄笔', summary: '你适合在场景表达里开战，用测试逼出真正能用的词。', advice: '场景题、主动回忆和混合测试适合你，单纯看解释容易不够刺激。', flavor: '笔尖开战，把场景题、主动回忆和混测都点成火花。' },
  CORA: { title: '观星鳄', summary: '你适合深语境、深理解和长期沉淀，越学越能看见词背后的水脉。', advice: '保持低压节奏，把复习、例句和表达串起来，不必追求每天大量新词。', flavor: '抬头看星盘，低头看例句，最会从语境水脉里摸到词义。' },
  CORT: { title: '鳄判官', summary: '你适合用错题和复盘审出薄弱点，越错越能变强。', advice: '混合测试、错词复习和间隔复习应占主导，新词量保持克制。', flavor: '戒尺轻敲错题本，薄弱点一个也别想溜走。' },
};

export function clampCrocBtiAnswer(value: number): CrocBtiAnswer {
  return Math.max(-2, Math.min(2, Math.round(value))) as CrocBtiAnswer;
}

export function hasCompleteCrocBtiAnswers(answers: Record<string, number>) {
  return crocBtiQuestions.every((question) => answers[question.id] !== undefined);
}

export function evaluateCrocBti(answers: Record<string, number>): CrocBtiResult {
  const axisScores = {
    vc: scoreAxis('vc', answers),
    io: scoreAxis('io', answers),
    nr: scoreAxis('nr', answers),
    at: scoreAxis('at', answers),
  };
  const code = `${axisScores.vc.selectedTrait}${axisScores.io.selectedTrait}${axisScores.nr.selectedTrait}${axisScores.at.selectedTrait}`;
  const text = profileText[code] ?? profileText.CORA;
  const weights = calculateCrocBtiWeights(code.split(''));
  const planInput = weightsToPlanInput(weights);
  return {
    code,
    title: text.title,
    summary: `${text.summary}${text.flavor}`,
    advice: text.advice,
    assetPath: `assets/croc_bti_compressed/${profileAssets[code] ?? profileAssets.CORA}`,
    flavor: text.flavor,
    axisScores,
    weights,
    planInput,
    questionTypeWeightsByMode: calculateCrocBtiQuestionTypeWeights(code),
  };
}

export function crocBtiPlanInputForDailyMinutes(weights: Record<string, number>, dailyMinutes: number) {
  const minutes = Math.max(10, Math.min(240, Math.round(dailyMinutes)));
  const modeWeights = normalizeWeights({
    newWords: weights.newWords ?? 0,
    review: weights.review ?? 0,
    mixedTest: (weights.mixedTest ?? 0) + Math.round((weights.activeRecall ?? 0) / 2),
    wrongWordReview:
      (weights.wrongWordReview ?? weights.wrongWords ?? 0) +
      Math.round((weights.activeRecall ?? 0) / 2),
    contextExamples: weights.contextExamples ?? weights.rootAffix ?? 0,
  });
  const totalQuestions = minutes * 4;
  const counts = Object.fromEntries(
    Object.entries(modeWeights).map(([key, weight]) => [
      key,
      Math.round((totalQuestions * weight) / 100),
    ]),
  );
  counts.newWords = roundDownToMultipleOfFour(counts.newWords ?? 0);
  const diff = totalQuestions - Object.values(counts).reduce((sum, value) => sum + value, 0);
  if (diff !== 0) {
    const target = largestCountKey(counts, 'newWords');
    counts[target] = Math.max(0, (counts[target] ?? 0) + diff);
  }
  return {
    newWordsPerDay: counts.newWords ?? 0,
    reviewWordsPerDay: counts.review ?? 0,
    mixedTestPerDay: counts.mixedTest ?? 0,
    wrongWordTestPerDay: counts.wrongWordReview ?? 0,
    rootAffixPerDay: counts.contextExamples ?? 0,
  };
}

export function crocBtiPlanInputFor(
  plan: PlanSummary,
  result: CrocBtiResult,
  options: {
    planInput?: Record<string, number>;
    questionTypeWeightsByMode?: Record<string, Record<string, number>>;
    growthRuleEnabled?: boolean;
  } = {},
) {
  return {
    name: result.title,
    ...(options.planInput ?? result.planInput),
    growthRuleEnabled: options.growthRuleEnabled ?? plan.growthRuleEnabled,
    growthRuleMode: plan.growthRuleMode,
    growthIntervalDays: plan.growthIntervalDays,
    growthIncrement: plan.growthIncrement,
    sharedGrowthRule: plan.sharedGrowthRule ?? {
      intervalDays: plan.growthIntervalDays,
      increment: plan.growthIncrement,
    },
    growthRulesByMode: plan.growthRulesByMode ?? {},
    questionTypeWeightsByMode: normalizeCrocBtiQuestionTypeWeightsByMode(
      options.questionTypeWeightsByMode ?? result.questionTypeWeightsByMode,
    ),
  };
}

export function normalizeCrocBtiQuestionTypeWeightsByMode(
  weightsByMode: Record<string, Record<string, number>>,
) {
  return Object.fromEntries(
    Object.entries(weightsByMode)
      .filter(([mode]) => mode !== 'newWord' && mode !== 'rootAffix')
      .map(([mode, weights]) => [mode, normalizeWeights(weights)]),
  );
}

export function rebalanceCrocBtiQuestionTypeWeights(
  weights: Record<string, number>,
  changedKey: string,
  nextValue: number,
) {
  if (!Object.prototype.hasOwnProperty.call(weights, changedKey)) return normalizeWeights(weights);
  const clamped = Math.max(0, Math.min(100, Math.round(nextValue)));
  const restKeys = Object.keys(weights).filter((key) => key !== changedKey);
  const restTotal = restKeys.reduce((sum, key) => sum + Math.max(0, weights[key] ?? 0), 0);
  const remaining = Math.max(0, 100 - clamped);
  const next: Record<string, number> = { [changedKey]: clamped };
  restKeys.forEach((key, index) => {
    next[key] = restTotal === 0
      ? index === 0 ? remaining : 0
      : Math.round((Math.max(0, weights[key] ?? 0) / restTotal) * remaining);
  });
  return normalizeWeights(next);
}

export function calculateCrocBtiQuestionTypeWeights(code: string) {
  const mixes: Record<string, Record<string, number>> = {
    review: {
      enToCnInput: 25,
      exampleToCnChoiceNoTranslation: 25,
      enToCnChoice: 20,
      cnToEnChoice: 15,
      wordSkeletonInput: 15,
    },
    mixedTest: {
      enToCnChoice: 25,
      exampleToCnChoice: 15,
      exampleToCnChoiceNoTranslation: 20,
      enToCnInput: 20,
      cnToEnChoice: 10,
      wordSkeletonInput: 10,
    },
    wrongWordReinforcement: {
      enToCnInput: 30,
      wordSkeletonInput: 25,
      exampleToCnChoiceNoTranslation: 20,
      cnToEnChoice: 15,
      enToCnChoice: 10,
    },
  };
  code.split('').forEach((trait) => {
    const delta = questionTypeDeltas[trait] ?? {};
    Object.entries(delta).forEach(([mode, changes]) => {
      Object.entries(changes).forEach(([questionType, change]) => {
        mixes[mode][questionType] = (mixes[mode][questionType] ?? 0) + change;
      });
    });
  });
  return normalizeCrocBtiQuestionTypeWeightsByMode(mixes);
}

function scoreAxis(axis: CrocBtiAxis, answers: Record<string, number>): CrocBtiAxisScore {
  const [firstTrait, secondTrait] = axisTraits[axis];
  const score = crocBtiQuestions
    .filter((question) => question.axis === axis)
    .reduce((sum, question) => {
      const answer = clampCrocBtiAnswer(answers[question.id] ?? 0);
      return sum + (question.positiveTrait === firstTrait ? answer : -answer);
    }, 0);
  const selectedTrait = score >= 0 ? firstTrait : secondTrait;
  const absolute = Math.abs(score);
  return {
    score,
    selectedTrait,
    strength: absolute >= 4 ? 'strong' : absolute >= 2 ? 'clear' : 'soft',
  };
}

function calculateCrocBtiWeights(traits: string[]) {
  const raw: Record<string, number> = {
    newWords: 25,
    review: 25,
    mixedTest: 20,
    wrongWordReview: 15,
    contextExamples: 10,
    activeRecall: 5,
  };
  traits.forEach((trait) => {
    if (trait === 'V') {
      raw.newWords += 10;
      raw.activeRecall += 5;
    }
    if (trait === 'C') {
      raw.contextExamples += 10;
      raw.mixedTest += 5;
    }
    if (trait === 'I') {
      raw.mixedTest += 5;
      raw.contextExamples += 5;
    }
    if (trait === 'O') {
      raw.activeRecall += 10;
      raw.wrongWordReview += 5;
    }
    if (trait === 'N') {
      raw.newWords += 15;
      raw.review -= 5;
    }
    if (trait === 'R') {
      raw.review += 10;
      raw.wrongWordReview += 10;
      raw.newWords -= 5;
    }
    if (trait === 'A') {
      raw.contextExamples += 10;
      raw.review += 5;
    }
    if (trait === 'T') {
      raw.mixedTest += 10;
      raw.wrongWordReview += 5;
    }
  });
  Object.keys(raw).forEach((key) => {
    raw[key] = Math.max(5, raw[key]);
  });
  return normalizeWeights(raw);
}

function weightsToPlanInput(weights: Record<string, number>) {
  const dailyUnits = 40;
  return {
    newWordsPerDay: roundDownToMultipleOfFour(
      Math.min(180, Math.max(4, Math.round((dailyUnits * (weights.newWords ?? 0) * 4) / 100))),
    ),
    reviewWordsPerDay: Math.min(100, Math.max(10, Math.round((dailyUnits * (weights.review ?? 0) * 2) / 100))),
    mixedTestPerDay: Math.min(30, Math.max(4, Math.round((dailyUnits * (weights.mixedTest ?? 0)) / 100))),
    wrongWordTestPerDay: Math.min(25, Math.max(2, Math.round((dailyUnits * (weights.wrongWordReview ?? 0)) / 100))),
    rootAffixPerDay: Math.min(10, Math.max(1, Math.round((dailyUnits * (weights.contextExamples ?? 0)) / 200))),
  };
}

const questionTypeDeltas: Record<string, Record<string, Record<string, number>>> = {
  V: { mixedTest: { enToCnChoice: 10, exampleToCnChoice: -5, wordSkeletonInput: -5 } },
  C: {
    mixedTest: { exampleToCnChoiceNoTranslation: 15, exampleToCnChoice: 5, enToCnChoice: -10 },
  },
  I: {
    mixedTest: { enToCnChoice: 10, exampleToCnChoiceNoTranslation: 5, cnToEnChoice: -5, wordSkeletonInput: -5 },
    review: { enToCnChoice: 5, enToCnInput: -5 },
  },
  O: {
    mixedTest: { wordSkeletonInput: 15, cnToEnChoice: 10, enToCnInput: 5, enToCnChoice: -15 },
    wrongWordReinforcement: { wordSkeletonInput: 10, cnToEnChoice: 5, enToCnChoice: -5 },
  },
  N: {
    mixedTest: { enToCnChoice: 5, exampleToCnChoice: 5, enToCnInput: -5 },
  },
  R: {
    review: { enToCnInput: 10, wordSkeletonInput: 5, enToCnChoice: -5 },
    wrongWordReinforcement: { enToCnInput: 10, wordSkeletonInput: 5, enToCnChoice: -5 },
  },
  A: {
    review: { exampleToCnChoiceNoTranslation: 10, cnToEnChoice: -5 },
    mixedTest: { exampleToCnChoiceNoTranslation: 10, exampleToCnChoice: 5, enToCnChoice: -5 },
  },
  T: {
    mixedTest: { enToCnInput: 10, wordSkeletonInput: 10, exampleToCnChoice: -5 },
    wrongWordReinforcement: { enToCnInput: 5, wordSkeletonInput: 5, exampleToCnChoiceNoTranslation: -5 },
  },
};

function normalizeWeights(weights: Record<string, number>) {
  const entries = Object.entries(weights);
  const total = entries.reduce((sum, [, value]) => sum + Math.max(0, value), 0) || 1;
  const normalized = Object.fromEntries(
    entries.map(([key, value]) => [key, Math.round((Math.max(0, value) / total) * 100)]),
  );
  const diff = 100 - Object.values(normalized).reduce((sum, value) => sum + value, 0);
  const target = Object.keys(normalized).sort((a, b) => normalized[b] - normalized[a])[0];
  if (target) normalized[target] += diff;
  return normalized;
}

function roundDownToMultipleOfFour(value: number) {
  return Math.max(0, value - (value % 4));
}

function largestCountKey(counts: Record<string, number>, exclude: string) {
  return Object.keys(counts)
    .filter((key) => key !== exclude)
    .reduce((best, key) => ((counts[key] ?? 0) > (counts[best] ?? 0) ? key : best), 'review');
}
