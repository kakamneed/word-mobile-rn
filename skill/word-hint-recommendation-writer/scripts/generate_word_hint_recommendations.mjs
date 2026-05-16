import fs from 'node:fs/promises';
import path from 'node:path';

const repoRoot = path.resolve(import.meta.dirname, '../../..');
const bookDir = path.join(repoRoot, 'apps/mobile/android/app/src/main/assets/seed-vocab/book');
const recommendationPath = path.join(
  repoRoot,
  'crates/platform-mobile/resources/word_hint_recommendations.json',
);
const overlapIndexPath = path.join(
  repoRoot,
  'crates/platform-mobile/resources/wordbook_overlap_index.json',
);
const vocabularyIndexPath = path.join(
  repoRoot,
  'crates/platform-mobile/resources/wordbook_vocabulary_index.json',
);

const books = [
  ['cet4', 'CET4_3.json'],
  ['cet6', 'CET6_3.json'],
  ['kaoyan', 'KaoYan_3.json'],
  ['medical', 'MEDICAL_RESP.json'],
];

const manualRecommendations = [
  {
    id: 'ai-effect-affect',
    word: 'effect',
    style: 'similarWord',
    label: 'AI辨析',
    text: '辨析：effect vs affect——effect 的 e 想 end result，多作名词“效果/结果”；affect 多作动词“影响”',
    wordbookCodes: ['cet4', 'cet6', 'kaoyan'],
  },
  {
    id: 'ai-language-phonetic',
    word: 'language',
    style: 'phonetic',
    label: 'AI谐音',
    text: '谐音：language南瓜哥——南瓜哥在说语言',
    wordbookCodes: ['cet4', 'cet6', 'kaoyan'],
  },
  {
    id: 'ai-medical-phonetic',
    word: 'medical',
    style: 'phonetic',
    label: 'AI谐音',
    text: '谐音：medical没得抠——身体出问题没得抠，得靠医学的办法',
    wordbookCodes: ['cet4', 'cet6', 'kaoyan'],
  },
  {
    id: 'ai-medical-root',
    word: 'medical',
    style: 'rootAffix',
    label: 'AI拆解',
    text: '拆解：medical=medic+al——medic 和医生/治疗相关，-al 表示“……的”，所以是“医学的”',
    wordbookCodes: ['cet4', 'cet6', 'kaoyan'],
  },
];

const clean = (value) =>
  String(value || '')
    .replace(/[\r\n\t]+/g, ' ')
    .replace(/\s+/g, ' ')
    .replace(/[<>A-Z]$/g, '')
    .trim();

const firstMeaning = (translations) =>
  clean(String(translations || '').split(/[；;]/)[0])
    .replace(/^[/\s]+/, '')
    .replace(/[<A-Z]+$/g, '')
    .trim();

const idPart = (value) =>
  String(value)
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '');

const sanitizeRem = (rem) =>
  clean(rem).replace(/\s*→\s*/g, '——').replace(/\s*\+\s*/g, '+').replace(/（/g, '(').replace(/）/g, ')');

const guessStyle = (rem) => {
  if (!rem) return 'phonetic';
  if (/来自/.test(rem)) return 'rootAffix';
  if (/[+＋]|\(|\)|→|根|词缀/.test(rem)) return 'rootAffix';
  return 'phonetic';
};

const labelFor = (style) =>
  ({
    phonetic: 'AI谐音',
    rootAffix: 'AI拆解',
    similarWord: 'AI辨析',
    story: 'AI典故',
  })[style] || 'AI推荐';

const weakMeaningRecommendation = (item) =>
  item?.style === 'meaning' ||
  String(item?.text || '').startsWith('提示：') ||
  String(item?.id || '').endsWith('-meaning');

const englishWords = (word) =>
  String(word || '')
    .toLowerCase()
    .match(/[a-z]+/g) || [];

const phoneticPieces = [
  ['tion', '神'],
  ['sion', '神'],
  ['civi', '细微'],
  ['civil', '细微了'],
  ['language', '南瓜哥'],
  ['govern', '狗问'],
  ['federal', '饭得肉'],
  ['cancel', '看手'],
  ['numer', '牛么'],
  ['ous', '饿死'],
  ['ate', '诶特'],
  ['ive', '爱舞'],
  ['ment', '闷特'],
  ['able', '诶包'],
  ['tional', '神脑'],
  ['pro', '破'],
  ['pre', '扑瑞'],
  ['con', '看'],
  ['com', '看'],
  ['per', '破'],
  ['sub', '萨布'],
  ['trans', '穿丝'],
  ['inter', '硬特'],
  ['anti', '安替'],
  ['ex', '爱克斯'],
  ['re', '瑞'],
  ['un', '安'],
  ['in', '因'],
  ['im', '因'],
  ['dis', '弟死'],
  ['a', '阿'],
  ['e', '一'],
  ['i', '衣'],
  ['o', '欧'],
  ['u', '优'],
  ['b', '布'],
  ['c', '克'],
  ['d', '德'],
  ['f', '夫'],
  ['g', '哥'],
  ['h', '喝'],
  ['j', '杰'],
  ['k', '克'],
  ['l', '了'],
  ['m', '么'],
  ['n', '嗯'],
  ['p', '普'],
  ['q', '丘'],
  ['r', '尔'],
  ['s', '思'],
  ['t', '特'],
  ['v', '维'],
  ['w', '屋'],
  ['x', '克斯'],
  ['y', '歪'],
  ['z', '兹'],
];

const phoneticHook = (word) => {
  let rest = String(word || '').toLowerCase().replace(/[^a-z]/g, '');
  const out = [];
  while (rest && out.join('').length < 10) {
    const piece = phoneticPieces.find(([key]) => rest.startsWith(key));
    if (!piece) break;
    out.push(piece[1]);
    rest = rest.slice(piece[0].length);
  }
  return out.join('') || '按音节编一句';
};

const affixBreakdown = (word) => {
  const chunks = [];
  let core = word;
  const prefixes = [
    ['anti', '反'],
    ['inter', '在...之间'],
    ['trans', '跨越'],
    ['pre', '预先'],
    ['pro', '向前'],
    ['re', '再'],
    ['un', '不'],
    ['in', '不/向内'],
    ['im', '不/向内'],
    ['dis', '分开/否定'],
    ['con', '共同'],
    ['com', '共同'],
    ['sub', '在下'],
    ['ex', '向外'],
  ];
  const suffixes = [
    ['ization', '名词：过程/状态'],
    ['isation', '名词：过程/状态'],
    ['tion', '名词'],
    ['sion', '名词'],
    ['ment', '名词'],
    ['ity', '名词性质'],
    ['ive', '形容词'],
    ['ous', '形容词：...的'],
    ['able', '能够...的'],
    ['al', '形容词'],
    ['er', '人/物'],
  ];
  for (const [pre, gloss] of prefixes) {
    if (core.startsWith(pre) && core.length > pre.length + 3) {
      chunks.push(`${pre}(${gloss})`);
      core = core.slice(pre.length);
      break;
    }
  }
  for (const [suf, gloss] of suffixes) {
    if (core.endsWith(suf) && core.length > suf.length + 3) {
      const base = core.slice(0, -suf.length);
      if (base) chunks.push(base);
      chunks.push(`${suf}(${gloss})`);
      return chunks.join('+');
    }
  }
  if (chunks.length) {
    chunks.push(core);
    return chunks.join('+');
  }
  return '';
};

const generatedWithoutRem = (row, meaning) => {
  const words = englishWords(row.word);
  const word = row.word;
  const joined = words.join(' ');
  if (word.endsWith('isation') || word.endsWith('ization')) {
    const pair = word.endsWith('isation')
      ? word.replace(/isation$/, 'ization')
      : word.replace(/ization$/, 'isation');
    return {
      style: 'similarWord',
      text: `辨析：${word} vs ${pair}——s/z是英美拼写差别，核心都抓civil“文明”，记“${meaning}”`,
    };
  }
  if (words.length > 1) {
    return {
      style: 'rootAffix',
      text: `拆解：${word}——${words.join('+')}——先按短语块记，再回到“${meaning}”`,
    };
  }
  const breakdown = affixBreakdown(word);
  if (breakdown) {
    return {
      style: 'rootAffix',
      text: `拆解：${word}——${breakdown}——拼写块串回“${meaning}”`,
    };
  }
  const hook = phoneticHook(joined || word);
  return {
    style: 'phonetic',
    text: `谐音：${word}${hook}——${hook}出场，拉回“${meaning}”`,
  };
};

async function loadSeedBooks() {
  const byWord = new Map();
  for (const [code, file] of books) {
    const data = JSON.parse(await fs.readFile(path.join(bookDir, file), 'utf8'));
    for (const item of data) {
      const word = String(item.headWord || item.content?.word?.wordHead || '').trim().toLowerCase();
      if (!word) continue;
      const rank = Number(item.wordRank || 999999);
      const content = item.content?.word?.content || {};
      if (!byWord.has(word)) byWord.set(word, { word, codes: new Set(), ranks: {}, rem: {}, trans: {} });
      const row = byWord.get(word);
      row.codes.add(code);
      row.ranks[code] = rank;
      if (content.remMethod?.val) row.rem[code] = clean(content.remMethod.val);
      if (content.trans) row.trans[code] = content.trans.map((t) => t.tranCn).filter(Boolean).join('；');
    }
  }
  return byWord;
}

function buildOverlapIndex(byWord) {
  const overlaps = [...byWord.values()]
    .map((row) => ({
      word: row.word,
      wordbookCodes: [...row.codes].sort(),
      ranks: row.ranks,
      trans: row.trans,
      rankSum: Object.values(row.ranks).reduce((sum, rank) => sum + rank, 0),
      examBookCount: [...row.codes].filter((code) => ['cet4', 'cet6', 'kaoyan'].includes(code)).length,
    }))
    .filter((row) => row.wordbookCodes.length >= 2)
    .sort(
      (a, b) =>
        b.examBookCount - a.examBookCount ||
        a.rankSum - b.rankSum ||
        a.word.localeCompare(b.word),
    );

  return {
    generatedFrom: 'apps/mobile/android/app/src/main/assets/seed-vocab/book',
    generatedAt: new Date().toISOString().slice(0, 10),
    summary: {
      uniqueWords: byWord.size,
      multiWordbookWords: overlaps.length,
      examThreeWayWords: overlaps.filter((row) => row.examBookCount === 3).length,
      examTwoWayWords: overlaps.filter((row) => row.examBookCount === 2).length,
    },
    overlaps,
  };
}

function buildVocabularyIndex(byWord) {
  const words = [...byWord.values()]
    .map((row) => ({
      word: row.word,
      wordbookCodes: [...row.codes].sort(),
      ranks: row.ranks,
      trans: row.trans,
      rankSum: Object.values(row.ranks).reduce((sum, rank) => sum + rank, 0),
      examBookCount: [...row.codes].filter((code) => ['cet4', 'cet6', 'kaoyan'].includes(code)).length,
    }))
    .sort(
      (a, b) =>
        b.wordbookCodes.length - a.wordbookCodes.length ||
        b.examBookCount - a.examBookCount ||
        a.rankSum - b.rankSum ||
        a.word.localeCompare(b.word),
    );

  return {
    generatedFrom: 'apps/mobile/android/app/src/main/assets/seed-vocab/book',
    generatedAt: new Date().toISOString().slice(0, 10),
    summary: {
      uniqueWords: byWord.size,
      singleWordbookWords: words.filter((row) => row.wordbookCodes.length === 1).length,
      multiWordbookWords: words.filter((row) => row.wordbookCodes.length >= 2).length,
      examThreeWayWords: words.filter((row) => row.examBookCount === 3).length,
      examTwoWayWords: words.filter((row) => row.examBookCount === 2).length,
    },
    words,
  };
}

function generatedRecommendation(row) {
  const rem = row.rem.kaoyan || row.rem.cet4 || row.rem.cet6 || row.rem.medical || '';
  const translations = row.trans.kaoyan || row.trans.cet4 || row.trans.cet6 || row.trans.medical || '';
  const meaning = firstMeaning(translations) || '核心释义';
  let style = guessStyle(rem);
  let text;

  if (rem) {
    const body = sanitizeRem(rem);
    text =
      style === 'rootAffix'
        ? `拆解：${row.word}——${body}，记“${meaning}”`
        : `谐音：${row.word}——${body}，记“${meaning}”`;
  } else {
    const generated = generatedWithoutRem(row, meaning);
    style = generated.style;
    text = generated.text;
  }

  return {
    id: `ai-${idPart(row.word)}-${style === 'similarWord' ? 'similar' : style === 'rootAffix' ? 'root' : 'phonetic'}`,
    word: row.word,
    style,
    label: labelFor(style),
    text,
    wordbookCodes: [...row.codes].sort(),
  };
}

const byWord = await loadSeedBooks();
const existing = JSON.parse(await fs.readFile(recommendationPath, 'utf8'));
const existingWithManual = [...existing, ...manualRecommendations];
const existingByWord = new Map();
for (const item of existingWithManual) {
  if (!existingByWord.has(item.word)) existingByWord.set(item.word, []);
  existingByWord.get(item.word).push(item);
}
const generated = [];
const rebuilt = [];
for (const row of byWord.values()) {
  const items = existingByWord.get(row.word) || [];
  const strongItems = items.filter((item) => !weakMeaningRecommendation(item));
  if (strongItems.length > 0) {
    rebuilt.push(...strongItems);
    continue;
  }
  const replacement = generatedRecommendation(row);
  rebuilt.push(replacement);
  generated.push(replacement);
}
for (const item of existingWithManual) {
  if (!byWord.has(item.word) && !weakMeaningRecommendation(item)) {
    rebuilt.push(item);
  }
}

const recommendations = rebuilt.sort(
  (a, b) => a.word.localeCompare(b.word) || a.style.localeCompare(b.style) || a.id.localeCompare(b.id),
);

await fs.writeFile(recommendationPath, `${JSON.stringify(recommendations, null, 2)}\n`, 'utf8');
await fs.writeFile(overlapIndexPath, `${JSON.stringify(buildOverlapIndex(byWord), null, 2)}\n`, 'utf8');
await fs.writeFile(vocabularyIndexPath, `${JSON.stringify(buildVocabularyIndex(byWord), null, 2)}\n`, 'utf8');

console.log(
  JSON.stringify(
    {
      recommendations: recommendations.length,
      words: new Set(recommendations.map((item) => item.word)).size,
      generated: generated.length,
    },
    null,
    2,
  ),
);
