import 'dart:convert';

import 'package:shared_preferences/shared_preferences.dart';

import '../sdk/sdk.dart';

typedef CrocBtiCode = String;

const _savedAnswersKey = 'croc_bti_saved_answers_v1';
const _savedDailyMinutesKey = 'croc_bti_daily_minutes_v1';
const _guestCrocBtiScope = 'guest';

String crocBtiStorageScope(String? userId) {
  final normalized = userId?.trim();
  return normalized == null || normalized.isEmpty
      ? _guestCrocBtiScope
      : normalized;
}

String _scopedKey(String baseKey, String? userId) {
  return '$baseKey.${crocBtiStorageScope(userId)}';
}

class CrocBtiQuestion {
  const CrocBtiQuestion({
    required this.id,
    required this.axis,
    required this.positiveTrait,
    required this.text,
  });

  final String id;
  final String axis;
  final String positiveTrait;
  final String text;
}

class CrocBtiAxisScore {
  const CrocBtiAxisScore({
    required this.score,
    required this.selectedTrait,
    required this.strength,
  });

  final int score;
  final String selectedTrait;
  final String strength;
}

class CrocBtiResult {
  const CrocBtiResult({
    required this.code,
    required this.title,
    required this.summary,
    required this.advice,
    required this.assetPath,
    required this.flavor,
    required this.axisScores,
    required this.weights,
    required this.planInput,
    required this.questionTypeWeightsByMode,
  });

  final CrocBtiCode code;
  final String title;
  final String summary;
  final String advice;
  final String assetPath;
  final String flavor;
  final Map<String, CrocBtiAxisScore> axisScores;
  final Map<String, int> weights;
  final Map<String, int> planInput;
  final Map<String, Map<String, int>> questionTypeWeightsByMode;
}

const crocBtiQuestions = <CrocBtiQuestion>[
  CrocBtiQuestion(
    id: 'vc-word-volume',
    axis: 'vc',
    positiveTrait: 'V',
    text: '我认为英语学习中，单词量比语法、语感、口语等更能决定上限。',
  ),
  CrocBtiQuestion(
    id: 'vc-many-unknowns',
    axis: 'vc',
    positiveTrait: 'V',
    text: '如果一篇文章里生词很多，即使句子结构不难，我也会明显读不下去。',
  ),
  CrocBtiQuestion(
    id: 'vc-build-vocab-first',
    axis: 'vc',
    positiveTrait: 'V',
    text: '我更愿意先把核心词汇量堆起来，再谈阅读和表达。',
  ),
  CrocBtiQuestion(
    id: 'vc-context-guessing',
    axis: 'vc',
    positiveTrait: 'C',
    text: '即使不认识某些词，我也常常能通过上下文猜出大概意思。',
  ),
  CrocBtiQuestion(
    id: 'vc-learn-in-sentences',
    axis: 'vc',
    positiveTrait: 'C',
    text: '我更愿意在文章、对话或例句里学词，而不是单独背词表。',
  ),
  CrocBtiQuestion(
    id: 'vc-usage-over-meaning',
    axis: 'vc',
    positiveTrait: 'C',
    text: '我觉得掌握一个词怎么用，比单纯记住它的中文意思更重要。',
  ),
  CrocBtiQuestion(
    id: 'io-recognize-enough',
    axis: 'io',
    positiveTrait: 'I',
    text: '对我来说，看到英文能理解中文意思，比看到中文想起英文更重要。',
  ),
  CrocBtiQuestion(
    id: 'io-reading-goal',
    axis: 'io',
    positiveTrait: 'I',
    text: '我主要的英语目标是阅读、听懂、考试理解，而不是主动表达。',
  ),
  CrocBtiQuestion(
    id: 'io-passive-ok',
    axis: 'io',
    positiveTrait: 'I',
    text: '一个词我只要能认出来，就算暂时不会主动使用，也可以接受。',
  ),
  CrocBtiQuestion(
    id: 'io-not-mastered',
    axis: 'io',
    positiveTrait: 'O',
    text: '如果我知道中文意思却想不起对应英文，我会觉得这个词还没真正掌握。',
  ),
  CrocBtiQuestion(
    id: 'io-use-in-writing',
    axis: 'io',
    positiveTrait: 'O',
    text: '我希望学习后能在写作、口语或造句中主动用出新词。',
  ),
  CrocBtiQuestion(
    id: 'io-cn-to-en',
    axis: 'io',
    positiveTrait: 'O',
    text: '我更喜欢中文提示回忆英文，而不是只做英文选中文。',
  ),
  CrocBtiQuestion(
    id: 'nr-new-progress',
    axis: 'nr',
    positiveTrait: 'N',
    text: '我喜欢每天学比较多的新词，这会让我有明显进步感。',
  ),
  CrocBtiQuestion(
    id: 'nr-review-boring',
    axis: 'nr',
    positiveTrait: 'N',
    text: '如果几天没学新词，只复习旧词，我会觉得效率不高。',
  ),
  CrocBtiQuestion(
    id: 'nr-want-next',
    axis: 'nr',
    positiveTrait: 'N',
    text: '遇到熟悉词反复出现，我容易不耐烦，想尽快进入新内容。',
  ),
  CrocBtiQuestion(
    id: 'nr-slower-solid',
    axis: 'nr',
    positiveTrait: 'R',
    text: '我宁愿每天少学一点，也希望学过的词能记得更牢。',
  ),
  CrocBtiQuestion(
    id: 'nr-repeat-forgotten',
    axis: 'nr',
    positiveTrait: 'R',
    text: '如果一个词反复忘，我希望系统持续安排它，而不是很快放过。',
  ),
  CrocBtiQuestion(
    id: 'nr-review-long-term',
    axis: 'nr',
    positiveTrait: 'R',
    text: '我觉得复习和错题整理比追求新词数量更能带来长期提升。',
  ),
  CrocBtiQuestion(
    id: 'at-examples',
    axis: 'at',
    positiveTrait: 'A',
    text: '我学单词时喜欢看例句、搭配、近义词差异。',
  ),
  CrocBtiQuestion(
    id: 'at-explain-needed',
    axis: 'at',
    positiveTrait: 'A',
    text: '如果只是刷题而没有解释，我会觉得学得不踏实。',
  ),
  CrocBtiQuestion(
    id: 'at-understand-first',
    axis: 'at',
    positiveTrait: 'A',
    text: '我更喜欢先理解词义和用法，再接受测试。',
  ),
  CrocBtiQuestion(
    id: 'at-test-reveals',
    axis: 'at',
    positiveTrait: 'T',
    text: '我喜欢用测试来发现自己到底会不会，而不是凭感觉判断。',
  ),
  CrocBtiQuestion(
    id: 'at-mixed-focus',
    axis: 'at',
    positiveTrait: 'T',
    text: '错题、限时、混合测试会让我更专注，也更容易记住。',
  ),
  CrocBtiQuestion(
    id: 'at-ok-wrong',
    axis: 'at',
    positiveTrait: 'T',
    text: '我不介意一开始错很多，只要测试能帮我快速暴露薄弱点。',
  ),
];

const _axisTraits = <String, List<String>>{
  'vc': ['V', 'C'],
  'io': ['I', 'O'],
  'nr': ['N', 'R'],
  'at': ['A', 'T'],
};

const _profiles = <String, ({String title, String summary, String advice})>{
  'VINA': (
    title: '鳄卷师',
    summary: '你适合一边扩充词库，一边把词义、例句和搭配卷进自己的知识卷轴里。',
    advice: '每天保留稳定新词量，但别只看列表；给例句和搭配留出固定时间。',
  ),
  'VINT': (
    title: '鳄骑兵',
    summary: '你适合用新词推进获得成长感，再用测试快速确认掌握度。',
    advice: '可以保持较高新词量，但要给混合测试和错题复盘留出硬性权重。',
  ),
  'VIRA': (
    title: '鳄碑客',
    summary: '你适合慢学深记，把词义像刻碑一样沉淀下来。',
    advice: '少量新词、足量复习和例句理解会比猛冲更适合你。',
  ),
  'VIRT': (
    title: '鳄甲卫',
    summary: '你重视词汇根基，也需要稳定复习来守住已经学过的内容。',
    advice: '新词不要完全停，但复习和错题应该成为你的主防线。',
  ),
  'VONA': (
    title: '鳄咏者',
    summary: '你学词是为了说出、写出和表达出来，适合把新词变成可用语言。',
    advice: '新词学习后马上接造句、中文回忆英文和搭配练习，效果会更明显。',
  ),
  'VONT': (
    title: '鳄刃使',
    summary: '你适合把词汇当成可拔出的武器，用主动回忆检验是否真的掌握。',
    advice: '中译英、拼写和混合测试应该占比更高，避免停留在看懂。',
  ),
  'VORA': (
    title: '鳄玄匠',
    summary: '你适合精修词的用法、搭配和表达质感，走少而精的路线。',
    advice: '降低新词冲刺感，多做主动输出和用法辨析。',
  ),
  'VORT': (
    title: '鳄铸师',
    summary: '你适合通过反复锻打把词汇压实，越练越硬。',
    advice: '错题、间隔复习和主动回忆是你的主炉火，新词量需要服从复习质量。',
  ),
  'CINA': (
    title: '鳄书客',
    summary: '你适合在文章和例句里自然吸收，靠语境把词慢慢养熟。',
    advice: '用语境学习承接新词，再用轻量测试确认自己没有误读。',
  ),
  'CINT': (
    title: '鳄游侠',
    summary: '你擅长在上下文里穿行，适合阅读中认词和快速判断。',
    advice: '多做语境阅读和混合选择题，但要防止猜对后以为已经掌握。',
  ),
  'CIRA': (
    title: '鳄隐士',
    summary: '你不急着刷量，适合靠长期语境和规律复习养出稳定语感。',
    advice: '低压复习、例句理解和少量新词会让你更持久。',
  ),
  'CIRT': (
    title: '鳄谋士',
    summary: '你擅长推理、排除和语境判断，适合考试型混合训练。',
    advice: '混合测试和错题复盘能放大你的优势，同时补上主动记忆漏洞。',
  ),
  'CONA': (
    title: '鳄语师',
    summary: '你适合在真实表达和语境里学习，让词汇自然长成语言能力。',
    advice: '多做造句、复述和例句改写，新词量不必太激进。',
  ),
  'CONT': (
    title: '鳄战巫',
    summary: '你适合在场景表达里开战，用测试逼出真正能用的词。',
    advice: '场景题、主动回忆和混合测试适合你，单纯看解释容易不够刺激。',
  ),
  'CORA': (
    title: '鳄渊者',
    summary: '你适合深语境、深理解和长期沉淀，越学越能看见词背后的水脉。',
    advice: '保持低压节奏，把复习、例句和表达串起来，不必追求每天大量新词。',
  ),
  'CORT': (
    title: '鳄刑官',
    summary: '你适合用错题和复盘审出薄弱点，越错越能变强。',
    advice: '混合测试、错题复习和间隔复习应占主导，新词量保持克制。',
  ),
};

const _profileVisuals = <String, ({String title, String assetPath, String flavor})>{
  'VINA': (
    title: '鳄卷师',
    assetPath: 'assets/croc_bti/scroll_master_croc.png',
    flavor: '卷轴一摊，词义、例句和搭配都被它卷进随身小本本里。',
  ),
  'VINT': (
    title: '鳄骑兵',
    assetPath: 'assets/croc_bti/cavalry_croc.png',
    flavor: '冲锋先看新词，回头再用混测点名验收，主打一个鳄速推进。',
  ),
  'VIRA': (
    title: '鳄碑',
    assetPath: 'assets/croc_bti/stele_croc.png',
    flavor: '慢慢刻、反复看，把词义刻成不会被浪冲走的碑文。',
  ),
  'VIRT': (
    title: '鳄甲卫',
    assetPath: 'assets/croc_bti/armor_guard_croc.png',
    flavor: '先把旧词防线守住，再稳稳扩张词库边境。',
  ),
  'VONA': (
    title: '鳄咏者',
    assetPath: 'assets/croc_bti/chanter_croc.png',
    flavor: '学词不是收藏，是要念出来、写出来、用出来。',
  ),
  'VONT': (
    title: '鳄剑士',
    assetPath: 'assets/croc_bti/swordsman_croc.png',
    flavor: '拔剑就测，靠主动回忆把“看懂”削成“真会”。',
  ),
  'VORA': (
    title: '古风小鳄',
    assetPath: 'assets/croc_bti/classic_croc.png',
    flavor: '少而精地修炼用法，讲究一个词有词的身段。',
  ),
  'VORT': (
    title: '鳄铸师',
    assetPath: 'assets/croc_bti/forgemaster_croc.png',
    flavor: '错题是炉火，复习是锤炼，越敲越结实。',
  ),
  'CINA': (
    title: '不鳄客',
    assetPath: 'assets/croc_bti/book_guest_bu_e_ke.png',
    flavor: '不是恶客，是不鳄客，进语境如进茶馆，慢慢把词泡开。',
  ),
  'CINT': (
    title: '鳄游侠',
    assetPath: 'assets/croc_bti/ranger_croc.jpg',
    flavor: '在上下文里穿行，线索一闪就能嗅到答案的水汽。',
  ),
  'CIRA': (
    title: '鳄隐士',
    assetPath: 'assets/croc_bti/hermit_croc.png',
    flavor: '不争一日之功，靠长期语境和复习把语感养肥。',
  ),
  'CIRT': (
    title: '全员鳄玉',
    assetPath: 'assets/croc_bti/alligator_jade.png',
    flavor: '全员鳄玉式统筹派，线索、排除和复盘都要安排得明明白白。',
  ),
  'CONA': (
    title: '吟游鳄',
    assetPath: 'assets/croc_bti/bard_croc.jpg',
    flavor: '短笛一吹，单词就从解释里跳出来，变成能说能写的表达。',
  ),
  'CONT': (
    title: '鳄笔',
    assetPath: 'assets/croc_bti/battle_mage_pencil_croc.png',
    flavor: '笔尖开战，把场景题、主动回忆和混测都点成小火花。',
  ),
  'CORA': (
    title: '观星鳄',
    assetPath: 'assets/croc_bti/stargazer_croc.jpg',
    flavor: '抬头看星盘，低头看例句，最会从语境水脉里摸到词义。',
  ),
  'CORT': (
    title: '鳄刑官',
    assetPath: 'assets/croc_bti/correction_officer_croc.jpg',
    flavor: '戒尺轻敲错题本，薄弱点一个也别想溜走。',
  ),
};

CrocBtiResult evaluateCrocBti(Map<String, int> answers) {
  final axisScores = <String, CrocBtiAxisScore>{};
  for (final axis in _axisTraits.keys) {
    axisScores[axis] = _scoreAxis(axis, answers);
  }
  final code =
      '${axisScores['vc']!.selectedTrait}${axisScores['io']!.selectedTrait}${axisScores['nr']!.selectedTrait}${axisScores['at']!.selectedTrait}';
  final profile = _profiles[code]!;
  final visual = _profileVisuals[code]!;
  final weights = calculateCrocBtiWeights([
    axisScores['vc']!.selectedTrait,
    axisScores['io']!.selectedTrait,
    axisScores['nr']!.selectedTrait,
    axisScores['at']!.selectedTrait,
  ]);
  return CrocBtiResult(
    code: code,
    title: profile.title,
    summary: '${profile.summary}${visual.flavor}',
    advice: profile.advice,
    assetPath: visual.assetPath,
    flavor: visual.flavor,
    axisScores: axisScores,
    weights: weights,
    planInput: weightsToPlanInput(weights),
    questionTypeWeightsByMode: calculateCrocBtiQuestionTypeWeights(code),
  );
}

Future<Map<String, int>> loadSavedCrocBtiAnswers({String? userId}) async {
  final prefs = await SharedPreferences.getInstance();
  final raw = prefs.getString(_scopedKey(_savedAnswersKey, userId));
  if (raw == null || raw.trim().isEmpty) return const <String, int>{};
  try {
    final decoded = jsonDecode(raw);
    if (decoded is! Map) return const <String, int>{};
    final validQuestionIds = crocBtiQuestions
        .map((question) => question.id)
        .toSet();
    return decoded.map((key, value) {
      final answer = value is num ? value.toInt() : 0;
      return MapEntry('$key', answer.clamp(-2, 2));
    })..removeWhere((key, value) => !validQuestionIds.contains(key));
  } catch (_) {
    return const <String, int>{};
  }
}

Future<void> saveCrocBtiAnswers(
  Map<String, int> answers, {
  String? userId,
}) async {
  final prefs = await SharedPreferences.getInstance();
  final validQuestionIds = crocBtiQuestions
      .map((question) => question.id)
      .toSet();
  final normalized = <String, int>{
    for (final entry in answers.entries)
      if (validQuestionIds.contains(entry.key))
        entry.key: entry.value.clamp(-2, 2),
  };
  await prefs.setString(
    _scopedKey(_savedAnswersKey, userId),
    jsonEncode(normalized),
  );
}

Future<void> clearSavedCrocBtiAnswers({String? userId}) async {
  final prefs = await SharedPreferences.getInstance();
  await prefs.remove(_scopedKey(_savedAnswersKey, userId));
  await prefs.remove(_scopedKey(_savedDailyMinutesKey, userId));
}

bool hasCompleteCrocBtiAnswers(Map<String, int> answers) {
  return crocBtiQuestions.every((question) => answers.containsKey(question.id));
}

Future<int> loadSavedCrocBtiDailyMinutes({String? userId}) async {
  final prefs = await SharedPreferences.getInstance();
  return (prefs.getInt(_scopedKey(_savedDailyMinutesKey, userId)) ?? 40)
      .clamp(10, 240)
      .toInt();
}

Future<void> saveCrocBtiDailyMinutes(int minutes, {String? userId}) async {
  final prefs = await SharedPreferences.getInstance();
  await prefs.setInt(
    _scopedKey(_savedDailyMinutesKey, userId),
    minutes.clamp(10, 240).toInt(),
  );
}

Map<String, dynamic> crocBtiPlanInputFor(
  PlanSummary plan,
  CrocBtiResult result, {
  Map<String, int>? planInput,
  Map<String, Map<String, int>>? questionTypeWeightsByMode,
}) {
  return <String, dynamic>{
    'name': result.title,
    ...(planInput ?? result.planInput),
    'growthRuleMode': plan.growthRuleMode,
    'growthIntervalDays': plan.growthIntervalDays,
    'growthIncrement': plan.growthIncrement,
    'sharedGrowthRule':
        plan.sharedGrowthRule ??
        <String, dynamic>{
          'intervalDays': plan.growthIntervalDays,
          'increment': plan.growthIncrement,
        },
    'growthRulesByMode': plan.growthRulesByMode ?? const <String, dynamic>{},
    'questionTypeWeightsByMode': normalizeCrocBtiQuestionTypeWeightsByMode(
      questionTypeWeightsByMode ?? result.questionTypeWeightsByMode,
    ),
  };
}

Map<String, int> crocBtiPlanInputForDailyMinutes(
  Map<String, int> weights,
  int dailyMinutes,
) {
  final minutes = dailyMinutes.clamp(10, 240).toInt();
  final modeWeights = _normalizeWeights({
    'newWords': weights['newWords'] ?? 0,
    'review': weights['review'] ?? 0,
    'mixedTest':
        (weights['mixedTest'] ?? 0) +
        ((weights['activeRecall'] ?? 0) / 2).round(),
    'wrongWordReview':
        (weights['wrongWordReview'] ?? 0) +
        ((weights['activeRecall'] ?? 0) / 2).round(),
    'contextExamples': weights['contextExamples'] ?? 0,
  });
  final totalQuestions = minutes * 4;
  final counts = <String, int>{
    for (final entry in modeWeights.entries)
      entry.key: ((totalQuestions * entry.value) / 100).round(),
  };
  counts['newWords'] = _roundDownToMultipleOfFour(counts['newWords'] ?? 0);
  final diff =
      totalQuestions - counts.values.fold<int>(0, (sum, value) => sum + value);
  if (diff > 0) {
    final target = _largestCountKey(counts, exclude: 'newWords');
    counts[target] = (counts[target] ?? 0) + diff;
  } else if (diff < 0) {
    var remaining = -diff;
    final keys = counts.keys.where((key) => key != 'newWords').toList()
      ..sort((a, b) => (counts[b] ?? 0).compareTo(counts[a] ?? 0));
    for (final key in keys) {
      if (remaining == 0) break;
      final available = counts[key] ?? 0;
      final take = available < remaining ? available : remaining;
      counts[key] = available - take;
      remaining -= take;
    }
  }

  return {
    'newWordsPerDay': counts['newWords'] ?? 0,
    'reviewWordsPerDay': counts['review'] ?? 0,
    'mixedTestPerDay': counts['mixedTest'] ?? 0,
    'wrongWordTestPerDay': counts['wrongWordReview'] ?? 0,
    'rootAffixPerDay': counts['contextExamples'] ?? 0,
  };
}

String _largestCountKey(Map<String, int> counts, {required String exclude}) {
  return counts.keys
      .where((key) => key != exclude)
      .fold<String>(
        'review',
        (best, key) => (counts[key] ?? 0) > (counts[best] ?? 0) ? key : best,
      );
}

Map<String, Map<String, int>> normalizeCrocBtiQuestionTypeWeightsByMode(
  Map<String, Map<String, int>> weightsByMode,
) {
  return {
    for (final entry in weightsByMode.entries)
      if (entry.key != 'rootAffix' && entry.key != 'newWord')
        entry.key: _normalizeWeights(entry.value),
  };
}

Map<String, Map<String, int>> calculateCrocBtiQuestionTypeWeights(
  CrocBtiCode code,
) {
  final mixes = <String, Map<String, int>>{
    'review': {
      'enToCnInput': 25,
      'exampleToCnChoiceNoTranslation': 25,
      'enToCnChoice': 20,
      'cnToEnChoice': 15,
      'wordSkeletonInput': 15,
    },
    'mixedTest': {
      'enToCnChoice': 25,
      'exampleToCnChoice': 15,
      'exampleToCnChoiceNoTranslation': 20,
      'enToCnInput': 20,
      'cnToEnChoice': 10,
      'wordSkeletonInput': 10,
    },
    'wrongWordReinforcement': {
      'enToCnInput': 30,
      'wordSkeletonInput': 25,
      'exampleToCnChoiceNoTranslation': 20,
      'cnToEnChoice': 15,
      'enToCnChoice': 10,
    },
  };

  for (final trait in code.split('')) {
    _applyQuestionTypeDeltas(mixes, _questionTypeDeltas[trait] ?? const {});
  }

  return {
    for (final entry in mixes.entries)
      entry.key: _normalizeWeights(entry.value),
  };
}

Map<String, int> calculateCrocBtiWeights(List<String> traits) {
  final raw = <String, int>{
    'newWords': 25,
    'review': 25,
    'mixedTest': 20,
    'wrongWordReview': 15,
    'contextExamples': 10,
    'activeRecall': 5,
  };
  final deltas = <String, Map<String, int>>{
    'V': {'newWords': 10, 'activeRecall': 5},
    'C': {'contextExamples': 10, 'mixedTest': 5},
    'I': {'mixedTest': 5, 'contextExamples': 5},
    'O': {'activeRecall': 10, 'wrongWordReview': 5},
    'N': {'newWords': 15, 'review': -5},
    'R': {'review': 10, 'wrongWordReview': 10, 'newWords': -5},
    'A': {'contextExamples': 10, 'review': 5},
    'T': {'mixedTest': 10, 'wrongWordReview': 5},
  };
  for (final trait in traits) {
    for (final entry in deltas[trait]!.entries) {
      raw[entry.key] = (raw[entry.key] ?? 0) + entry.value;
    }
  }
  for (final key in raw.keys) {
    raw[key] = raw[key]!.clamp(5, 100);
  }
  return _normalizeWeights(raw);
}

Map<String, int> weightsToPlanInput(Map<String, int> weights) {
  const dailyUnits = 40;
  return {
    'newWordsPerDay': _roundDownToMultipleOfFour(
      ((dailyUnits * weights['newWords']! * 4) / 100).round().clamp(4, 180),
    ),
    'reviewWordsPerDay': ((dailyUnits * weights['review']! * 2) / 100)
        .round()
        .clamp(10, 100),
    'mixedTestPerDay': ((dailyUnits * weights['mixedTest']!) / 100)
        .round()
        .clamp(4, 30),
    'wrongWordTestPerDay': ((dailyUnits * weights['wrongWordReview']!) / 100)
        .round()
        .clamp(2, 25),
    'rootAffixPerDay': ((dailyUnits * weights['contextExamples']!) / 200)
        .round()
        .clamp(1, 10),
  };
}

int _roundDownToMultipleOfFour(num value) {
  final rounded = value.round();
  return rounded - (rounded % 4);
}

const _questionTypeDeltas = <String, Map<String, Map<String, int>>>{
  'V': {
    'mixedTest': {
      'enToCnChoice': 10,
      'exampleToCnChoice': -5,
      'wordSkeletonInput': -5,
    },
  },
  'C': {
    'mixedTest': {
      'exampleToCnChoiceNoTranslation': 15,
      'exampleToCnChoice': 5,
      'enToCnChoice': -10,
    },
  },
  'I': {
    'mixedTest': {
      'enToCnChoice': 10,
      'exampleToCnChoiceNoTranslation': 5,
      'cnToEnChoice': -5,
      'wordSkeletonInput': -5,
    },
    'review': {'enToCnChoice': 5, 'enToCnInput': -5},
  },
  'O': {
    'mixedTest': {
      'wordSkeletonInput': 15,
      'cnToEnChoice': 10,
      'enToCnInput': 5,
      'enToCnChoice': -15,
    },
    'wrongWordReinforcement': {
      'wordSkeletonInput': 10,
      'cnToEnChoice': 5,
      'enToCnChoice': -5,
    },
  },
  'N': {
    'mixedTest': {'enToCnChoice': 5, 'exampleToCnChoice': 5, 'enToCnInput': -5},
  },
  'R': {
    'review': {'enToCnInput': 10, 'wordSkeletonInput': 5, 'enToCnChoice': -5},
    'wrongWordReinforcement': {
      'enToCnInput': 10,
      'wordSkeletonInput': 5,
      'enToCnChoice': -5,
    },
  },
  'A': {
    'review': {'exampleToCnChoiceNoTranslation': 10, 'cnToEnChoice': -5},
    'mixedTest': {
      'exampleToCnChoiceNoTranslation': 10,
      'exampleToCnChoice': 5,
      'enToCnChoice': -5,
    },
  },
  'T': {
    'mixedTest': {
      'enToCnInput': 10,
      'wordSkeletonInput': 10,
      'exampleToCnChoice': -5,
    },
    'wrongWordReinforcement': {
      'enToCnInput': 5,
      'wordSkeletonInput': 5,
      'exampleToCnChoiceNoTranslation': -5,
    },
  },
};

void _applyQuestionTypeDeltas(
  Map<String, Map<String, int>> mixes,
  Map<String, Map<String, int>> deltas,
) {
  for (final modeEntry in deltas.entries) {
    final modeMix = mixes[modeEntry.key];
    if (modeMix == null) continue;
    for (final delta in modeEntry.value.entries) {
      if (!modeMix.containsKey(delta.key)) continue;
      modeMix[delta.key] = ((modeMix[delta.key] ?? 0) + delta.value).clamp(
        0,
        100,
      );
    }
  }
}

CrocBtiAxisScore _scoreAxis(String axis, Map<String, int> answers) {
  final traits = _axisTraits[axis]!;
  final firstTrait = traits.first;
  var score = 0;
  for (final question in crocBtiQuestions.where((item) => item.axis == axis)) {
    final answer = answers[question.id] ?? 0;
    score += question.positiveTrait == firstTrait ? answer : -answer;
  }
  final absolute = score.abs();
  return CrocBtiAxisScore(
    score: score,
    selectedTrait: score >= 0 ? traits.first : traits.last,
    strength: absolute >= 4
        ? 'clear'
        : absolute >= 1
        ? 'light'
        : 'balanced',
  );
}

Map<String, int> _normalizeWeights(Map<String, int> raw) {
  final total = raw.values.fold<int>(0, (sum, value) => sum + value);
  if (total <= 0 || raw.isEmpty) return Map<String, int>.from(raw);
  final rounded = <String, int>{};
  for (final entry in raw.entries) {
    rounded[entry.key] = ((entry.value / total) * 100).round();
  }
  final roundedTotal = rounded.values.fold<int>(0, (sum, value) => sum + value);
  final correctionKey = rounded.containsKey('newWords')
      ? 'newWords'
      : rounded.keys.first;
  rounded[correctionKey] = rounded[correctionKey]! + (100 - roundedTotal);
  return rounded;
}
