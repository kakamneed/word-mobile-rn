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
  if (!rem) return 'meaning';
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
    meaning: 'AI推荐',
  })[style] || 'AI推荐';

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
    style = 'meaning';
    text = `提示：${row.word}——先锁定核心义“${meaning}”，再排除相近干扰项`;
  }

  return {
    id: `ai-${idPart(row.word)}-${style === 'meaning' ? 'meaning' : style === 'rootAffix' ? 'root' : 'phonetic'}`,
    word: row.word,
    style,
    label: labelFor(style),
    text,
    wordbookCodes: [...row.codes].sort(),
  };
}

const byWord = await loadSeedBooks();
const existing = JSON.parse(await fs.readFile(recommendationPath, 'utf8'));
const existingByWord = new Set(existing.map((item) => item.word));
const generated = [...byWord.values()]
  .filter((row) => !existingByWord.has(row.word))
  .map(generatedRecommendation);

const recommendations = [...existing, ...generated].sort(
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
