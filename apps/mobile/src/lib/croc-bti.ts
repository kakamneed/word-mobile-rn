import type {PlanEditInput} from './mobile-bridge';

export type CrocBtiAxis = 'vc' | 'io' | 'nr' | 'at';
export type CrocBtiTrait = 'V' | 'C' | 'I' | 'O' | 'N' | 'R' | 'A' | 'T';
export type CrocBtiCode = `${'V' | 'C'}${'I' | 'O'}${'N' | 'R'}${'A' | 'T'}`;
export type CrocBtiAnswerValue = -2 | -1 | 0 | 1 | 2;

export interface CrocBtiQuestion {
  id: string;
  axis: CrocBtiAxis;
  positiveTrait: CrocBtiTrait;
  text: string;
}

export interface CrocBtiAnswer {
  questionId: string;
  value: CrocBtiAnswerValue;
}

export interface CrocBtiAxisScore {
  axis: CrocBtiAxis;
  firstTrait: CrocBtiTrait;
  secondTrait: CrocBtiTrait;
  score: number;
  selectedTrait: CrocBtiTrait;
  strength: 'balanced' | 'light' | 'clear';
}

export interface CrocBtiWeights {
  newWords: number;
  review: number;
  mixedTest: number;
  wrongWordReview: number;
  contextExamples: number;
  activeRecall: number;
}

export interface CrocBtiResult {
  code: CrocBtiCode;
  title: string;
  summary: string;
  advice: string;
  axisScores: Record<CrocBtiAxis, CrocBtiAxisScore>;
  weights: CrocBtiWeights;
  planInput: Required<
    Pick<
      PlanEditInput,
      | 'newWordsPerDay'
      | 'reviewWordsPerDay'
      | 'mixedTestPerDay'
      | 'wrongWordTestPerDay'
      | 'rootAffixPerDay'
    >
  >;
}

const AXIS_TRAITS: Record<CrocBtiAxis, [CrocBtiTrait, CrocBtiTrait]> = {
  vc: ['V', 'C'],
  io: ['I', 'O'],
  nr: ['N', 'R'],
  at: ['A', 'T'],
};

export const CROC_BTI_QUESTIONS: CrocBtiQuestion[] = [
  {
    id: 'vc-word-volume',
    axis: 'vc',
    positiveTrait: 'V',
    text: '我认为英语学习中，单词量比语法、语感、口语等更能决定上限。',
  },
  {
    id: 'vc-many-unknowns',
    axis: 'vc',
    positiveTrait: 'V',
    text: '如果一篇文章里生词很多，即使句子结构不难，我也会明显读不下去。',
  },
  {
    id: 'vc-build-vocab-first',
    axis: 'vc',
    positiveTrait: 'V',
    text: '我更愿意先把核心词汇量堆起来，再谈阅读和表达。',
  },
  {
    id: 'vc-context-guessing',
    axis: 'vc',
    positiveTrait: 'C',
    text: '即使不认识某些词，我也常常能通过上下文猜出大概意思。',
  },
  {
    id: 'vc-learn-in-sentences',
    axis: 'vc',
    positiveTrait: 'C',
    text: '我更愿意在文章、对话或例句里学词，而不是单独背词表。',
  },
  {
    id: 'vc-usage-over-meaning',
    axis: 'vc',
    positiveTrait: 'C',
    text: '我觉得掌握一个词怎么用，比单纯记住它的中文意思更重要。',
  },
  {
    id: 'io-recognize-enough',
    axis: 'io',
    positiveTrait: 'I',
    text: '对我来说，看到英文能理解中文意思，比看到中文想起英文更重要。',
  },
  {
    id: 'io-reading-goal',
    axis: 'io',
    positiveTrait: 'I',
    text: '我主要的英语目标是阅读、听懂、考试理解，而不是主动表达。',
  },
  {
    id: 'io-passive-ok',
    axis: 'io',
    positiveTrait: 'I',
    text: '一个词我只要能认出来，就算暂时不会主动使用，也可以接受。',
  },
  {
    id: 'io-not-mastered',
    axis: 'io',
    positiveTrait: 'O',
    text: '如果我知道中文意思却想不起对应英文，我会觉得这个词还没真正掌握。',
  },
  {
    id: 'io-use-in-writing',
    axis: 'io',
    positiveTrait: 'O',
    text: '我希望学习后能在写作、口语或造句中主动用出新词。',
  },
  {
    id: 'io-cn-to-en',
    axis: 'io',
    positiveTrait: 'O',
    text: '我更喜欢中文提示回忆英文，而不是只做英文选中文。',
  },
  {
    id: 'nr-new-progress',
    axis: 'nr',
    positiveTrait: 'N',
    text: '我喜欢每天学比较多的新词，这会让我有明显进步感。',
  },
  {
    id: 'nr-review-boring',
    axis: 'nr',
    positiveTrait: 'N',
    text: '如果几天没学新词，只复习旧词，我会觉得效率不高。',
  },
  {
    id: 'nr-want-next',
    axis: 'nr',
    positiveTrait: 'N',
    text: '遇到熟悉词反复出现，我容易不耐烦，想尽快进入新内容。',
  },
  {
    id: 'nr-slower-solid',
    axis: 'nr',
    positiveTrait: 'R',
    text: '我宁愿每天少学一点，也希望学过的词能记得更牢。',
  },
  {
    id: 'nr-repeat-forgotten',
    axis: 'nr',
    positiveTrait: 'R',
    text: '如果一个词反复忘，我希望系统持续安排它，而不是很快放过。',
  },
  {
    id: 'nr-review-long-term',
    axis: 'nr',
    positiveTrait: 'R',
    text: '我觉得复习和错题整理比追求新词数量更能带来长期提升。',
  },
  {
    id: 'at-examples',
    axis: 'at',
    positiveTrait: 'A',
    text: '我学单词时喜欢看例句、搭配、近义词差异。',
  },
  {
    id: 'at-explain-needed',
    axis: 'at',
    positiveTrait: 'A',
    text: '如果只是刷题而没有解释，我会觉得学得不踏实。',
  },
  {
    id: 'at-understand-first',
    axis: 'at',
    positiveTrait: 'A',
    text: '我更喜欢先理解词义和用法，再接受测试。',
  },
  {
    id: 'at-test-reveals',
    axis: 'at',
    positiveTrait: 'T',
    text: '我喜欢用测试来发现自己到底会不会，而不是凭感觉判断。',
  },
  {
    id: 'at-mixed-focus',
    axis: 'at',
    positiveTrait: 'T',
    text: '错题、限时、混合测试会让我更专注，也更容易记住。',
  },
  {
    id: 'at-ok-wrong',
    axis: 'at',
    positiveTrait: 'T',
    text: '我不介意一开始错很多，只要测试能帮我快速暴露薄弱点。',
  },
];

const PROFILE_TITLES: Record<CrocBtiCode, {title: string; summary: string; advice: string}> = {
  VINA: {
    title: '鳄卷师',
    summary: '你适合一边扩充词库，一边把词义、例句和搭配卷进自己的知识卷轴里。',
    advice: '每天保留稳定新词量，但别只看列表；给例句和搭配留出固定时间。',
  },
  VINT: {
    title: '鳄骑兵',
    summary: '你适合用新词推进获得成长感，再用测试快速确认掌握度。',
    advice: '可以保持较高新词量，但要给混合测试和错题复盘留出硬性权重。',
  },
  VIRA: {
    title: '鳄碑客',
    summary: '你适合慢学深记，把词义像刻碑一样沉淀下来。',
    advice: '少量新词、足量复习和例句理解会比猛冲更适合你。',
  },
  VIRT: {
    title: '鳄甲卫',
    summary: '你重视词汇根基，也需要稳定复习来守住已经学过的内容。',
    advice: '新词不要完全停，但复习和错题应该成为你的主防线。',
  },
  VONA: {
    title: '鳄咏者',
    summary: '你学词是为了说出、写出和表达出来，适合把新词变成可用语言。',
    advice: '新词学习后马上接造句、中文回忆英文和搭配练习，效果会更明显。',
  },
  VONT: {
    title: '鳄刃使',
    summary: '你适合把词汇当成可拔出的武器，用主动回忆检验是否真的掌握。',
    advice: '中译英、拼写和混合测试应该占比更高，避免停留在看懂。',
  },
  VORA: {
    title: '鳄玄匠',
    summary: '你适合精修词的用法、搭配和表达质感，走少而精的路线。',
    advice: '降低新词冲刺感，多做主动输出和用法辨析。',
  },
  VORT: {
    title: '鳄铸师',
    summary: '你适合通过反复锻打把词汇压实，越练越硬。',
    advice: '错题、间隔复习和主动回忆是你的主炉火，新词量需要服从复习质量。',
  },
  CINA: {
    title: '鳄书客',
    summary: '你适合在文章和例句里自然吸收，靠语境把词慢慢养熟。',
    advice: '用语境学习承接新词，再用轻量测试确认自己没有误读。',
  },
  CINT: {
    title: '鳄游侠',
    summary: '你擅长在上下文里穿行，适合阅读中认词和快速判断。',
    advice: '多做语境阅读和混合选择题，但要防止猜对后以为已经掌握。',
  },
  CIRA: {
    title: '鳄隐士',
    summary: '你不急着刷量，适合靠长期语境和规律复习养出稳定语感。',
    advice: '低压复习、例句理解和少量新词会让你更持久。',
  },
  CIRT: {
    title: '鳄谋士',
    summary: '你擅长推理、排除和语境判断，适合考试型混合训练。',
    advice: '混合测试和错题复盘能放大你的优势，同时补上主动记忆漏洞。',
  },
  CONA: {
    title: '鳄语师',
    summary: '你适合在真实表达和语境里学习，让词汇自然长成语言能力。',
    advice: '多做造句、复述和例句改写，新词量不必太激进。',
  },
  CONT: {
    title: '鳄战巫',
    summary: '你适合在场景表达里开战，用测试逼出真正能用的词。',
    advice: '场景题、主动回忆和混合测试适合你，单纯看解释容易不够刺激。',
  },
  CORA: {
    title: '鳄渊者',
    summary: '你适合深语境、深理解和长期沉淀，越学越能看见词背后的水脉。',
    advice: '保持低压节奏，把复习、例句和表达串起来，不必追求每天大量新词。',
  },
  CORT: {
    title: '鳄刑官',
    summary: '你适合用错题和复盘审出薄弱点，越错越能变强。',
    advice: '混合测试、错题复习和间隔复习应占主导，新词量保持克制。',
  },
};

const BASE_WEIGHTS: CrocBtiWeights = {
  newWords: 25,
  review: 25,
  mixedTest: 20,
  wrongWordReview: 15,
  contextExamples: 10,
  activeRecall: 5,
};

const TRAIT_WEIGHT_DELTAS: Record<CrocBtiTrait, Partial<CrocBtiWeights>> = {
  V: {newWords: 10, activeRecall: 5},
  C: {contextExamples: 10, mixedTest: 5},
  I: {mixedTest: 5, contextExamples: 5},
  O: {activeRecall: 10, wrongWordReview: 5},
  N: {newWords: 15, review: -5},
  R: {review: 10, wrongWordReview: 10, newWords: -5},
  A: {contextExamples: 10, review: 5},
  T: {mixedTest: 10, wrongWordReview: 5},
};

export function evaluateCrocBti(answers: CrocBtiAnswer[]): CrocBtiResult {
  const answerMap = new Map(answers.map(answer => [answer.questionId, answer.value]));
  const axisScores = (Object.keys(AXIS_TRAITS) as CrocBtiAxis[]).reduce(
    (acc, axis) => {
      acc[axis] = scoreAxis(axis, answerMap);
      return acc;
    },
    {} as Record<CrocBtiAxis, CrocBtiAxisScore>,
  );

  const code = `${axisScores.vc.selectedTrait}${axisScores.io.selectedTrait}${axisScores.nr.selectedTrait}${axisScores.at.selectedTrait}` as CrocBtiCode;
  const profile = PROFILE_TITLES[code];
  const weights = calculateWeights([
    axisScores.vc.selectedTrait,
    axisScores.io.selectedTrait,
    axisScores.nr.selectedTrait,
    axisScores.at.selectedTrait,
  ]);

  return {
    code,
    title: profile.title,
    summary: profile.summary,
    advice: profile.advice,
    axisScores,
    weights,
    planInput: weightsToPlanInput(weights),
  };
}

export function calculateWeights(traits: CrocBtiTrait[]): CrocBtiWeights {
  const raw = {...BASE_WEIGHTS};
  for (const trait of traits) {
    const deltas = TRAIT_WEIGHT_DELTAS[trait];
    for (const key of Object.keys(deltas) as (keyof CrocBtiWeights)[]) {
      raw[key] += deltas[key] ?? 0;
    }
  }

  for (const key of Object.keys(raw) as (keyof CrocBtiWeights)[]) {
    raw[key] = Math.max(5, raw[key]);
  }

  return normalizeWeights(raw);
}

export function weightsToPlanInput(weights: CrocBtiWeights): CrocBtiResult['planInput'] {
  const dailyUnits = 40;
  return {
    newWordsPerDay: clamp(Math.round((dailyUnits * weights.newWords) / 100), 5, 45),
    reviewWordsPerDay: clamp(Math.round((dailyUnits * weights.review * 2) / 100), 10, 100),
    mixedTestPerDay: clamp(Math.round((dailyUnits * weights.mixedTest) / 100), 4, 30),
    wrongWordTestPerDay: clamp(
      Math.round((dailyUnits * weights.wrongWordReview) / 100),
      2,
      25,
    ),
    rootAffixPerDay: clamp(Math.round((dailyUnits * weights.contextExamples) / 200), 1, 10),
  };
}

function scoreAxis(
  axis: CrocBtiAxis,
  answerMap: Map<string, CrocBtiAnswerValue>,
): CrocBtiAxisScore {
  const [firstTrait, secondTrait] = AXIS_TRAITS[axis];
  const score = CROC_BTI_QUESTIONS.filter(question => question.axis === axis).reduce(
    (total, question) => {
      const answer = answerMap.get(question.id) ?? 0;
      return total + (question.positiveTrait === firstTrait ? answer : -answer);
    },
    0,
  );

  const selectedTrait = score >= 0 ? firstTrait : secondTrait;
  const absolute = Math.abs(score);
  const strength = absolute >= 4 ? 'clear' : absolute >= 1 ? 'light' : 'balanced';

  return {
    axis,
    firstTrait,
    secondTrait,
    score,
    selectedTrait,
    strength,
  };
}

function normalizeWeights(weights: CrocBtiWeights): CrocBtiWeights {
  const total = Object.values(weights).reduce((sum, value) => sum + value, 0);
  const entries = Object.entries(weights) as [keyof CrocBtiWeights, number][];
  const rounded = entries.reduce(
    (acc, [key, value]) => {
      acc[key] = Math.round((value / total) * 100);
      return acc;
    },
    {} as CrocBtiWeights,
  );
  const diff = 100 - Object.values(rounded).reduce((sum, value) => sum + value, 0);
  rounded.newWords += diff;
  return rounded;
}

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}
