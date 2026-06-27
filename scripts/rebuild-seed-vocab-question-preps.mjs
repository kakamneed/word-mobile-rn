import fs from 'node:fs';
import path from 'node:path';

const args = new Map();
for (let index = 2; index < process.argv.length; index += 1) {
  const [key, value] = process.argv[index].split('=', 2);
  if (key?.startsWith('--')) args.set(key.slice(2), value ?? '');
}

const bookDir = args.get('bookDir') ?? 'apps/mobile/android/app/src/main/assets/seed-vocab/book';
const reportPath = args.get('reportPath') ?? 'docs/seed-vocab-question-preps-report.json';
const maxDistractors = Number(args.get('maxDistractors') ?? 7);
const minDistractors = Math.min(3, maxDistractors);

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, 'utf8').replace(/^\uFEFF/, ''));
}

function writeJson(filePath, value) {
  fs.writeFileSync(filePath, `${JSON.stringify(value)}\n`, 'utf8');
}

function normalizePos(pos) {
  const value = String(pos ?? '').trim().replace(/\.+$/u, '').toLowerCase();
  if (!value) return '';
  if (value.startsWith('v')) return 'v';
  if (value.startsWith('n')) return 'n';
  if (value.startsWith('adj') || value === 'a') return 'adj';
  if (value.startsWith('adv')) return 'adv';
  if (value.startsWith('prep')) return 'prep';
  if (value.startsWith('conj')) return 'conj';
  if (value.startsWith('pron')) return 'pron';
  if (value === 'int' || value.startsWith('interj')) return 'int';
  return value;
}

function isLikelyMedicalPhrasePos(pos) {
  return ['effect', 'dioxide', 'computed', 'hormone', 'failure', 'magnetic', 'concentration', 'therapy', 'index'].includes(pos);
}

function broadPos(pos) {
  const normalized = normalizePos(pos);
  if (['n', 'v', 'adj', 'adv', 'prep', 'conj', 'pron', 'int'].includes(normalized)) return normalized;
  if (isLikelyMedicalPhrasePos(normalized)) return 'n';
  return normalized || 'unknown';
}
function displayWordForEntry(bookCode, item, pos) {
  const headWord = String(item?.headWord ?? getWordNode(item)?.wordHead ?? '').trim();
  if (bookCode !== 'MEDICAL_RESP' || !headWord) return headWord;
  const normalizedPos = normalizePos(pos);
  if (!isLikelyMedicalPhrasePos(normalizedPos)) return headWord;
  const lowerHead = headWord.toLowerCase();
  if (['ct', 'mri'].includes(lowerHead)) return headWord;
  return `${headWord} ${normalizedPos}`.trim();
}

function stripExamMarkers(text) {
  return String(text ?? '')
    .split(/[;,，；]/u)
    .map((part) => part.trim().replace(/\s+[ABCD]$/u, '').replace(/[ABCD]$/u, '').trim())
    .filter(Boolean)
    .join('；');
}

function cleanMeaning(text) {
  return stripExamMarkers(
    String(text ?? '')
      .replace(/[<>]/gu, '')
      .replace(/\uFFFD/gu, '')
      .replace(/\s+/gu, ' ')
      .trim(),
  );
}

function meaningKey(text) {
  return cleanMeaning(text)
    .replace(/[\s,.;:，。；：、]/gu, '')
    .toLowerCase();
}

function normalizedText(text) {
  return cleanMeaning(text).replace(/[\s,.;:，。；：、]/gu, '');
}
function normalizedWord(text) {
  return String(text ?? '').toLowerCase().replace(/[^a-z0-9]/gu, '');
}

function meaningTooSimilar(left, right) {
  const a = normalizedText(left);
  const b = normalizedText(right);
  if (!a || !b) return false;
  if (a === b) return true;
  const shorter = Math.min(a.length, b.length);
  if (shorter >= 2 && (a.includes(b) || b.includes(a))) return true;
  return false;
}

function wordTooSimilar(left, right) {
  const a = normalizedWord(left);
  const b = normalizedWord(right);
  if (!a || !b) return false;
  if (a === b) return true;
  const shorter = Math.min(a.length, b.length);
  return shorter >= 3 && (a.includes(b) || b.includes(a));
}
function acronymOf(text) {
  return String(text ?? '')
    .split(/[^a-zA-Z0-9]+/u)
    .filter(Boolean)
    .map((part) => part[0])
    .join('')
    .toLowerCase();
}

function candidateLooksEquivalent(target, candidate) {
  if (meaningTooSimilar(target.meaning, candidate.meaning)) return true;
  if (wordTooSimilar(target.word, candidate.word)) return true;
  const targetWord = normalizedWord(target.word);
  const candidateWord = normalizedWord(candidate.word);
  if (targetWord.length >= 2 && targetWord.length <= 5 && acronymOf(candidate.word) === targetWord) return true;
  if (candidateWord.length >= 2 && candidateWord.length <= 5 && acronymOf(target.word) === candidateWord) return true;
  if (target.broadPos === 'n' && candidate.broadPos === 'n') {
    const targetPhrase = normalizedWord(`${target.word} ${target.pos}`);
    const candidatePhrase = normalizedWord(`${candidate.word} ${candidate.pos}`);
    if (targetPhrase && candidateWord && targetPhrase === candidateWord) return true;
    if (candidatePhrase && targetWord && candidatePhrase === targetWord) return true;
  }
  return false;
}

function overlapScore(left, right) {
  const a = normalizedText(left);
  const b = normalizedText(right);
  if (!a || !b) return 0;
  if (a === b) return -1000;
  if (a.includes(b) || b.includes(a)) return Math.min(a.length, b.length) + 50;
  const chars = new Set([...a]);
  let score = 0;
  for (const ch of b) if (chars.has(ch)) score += 1;
  return score;
}

function getWordNode(item) {
  return item?.content?.word ?? {};
}

function getContent(item) {
  return getWordNode(item)?.content ?? {};
}

function primaryMeaning(item) {
  const trans = Array.isArray(getContent(item).trans) ? getContent(item).trans : [];
  const first = trans.find((meaning) => cleanMeaning(meaning?.tranCn));
  return first ? cleanMeaning(first.tranCn) : '';
}

function primaryPos(item) {
  const trans = Array.isArray(getContent(item).trans) ? getContent(item).trans : [];
  const first = trans.find((meaning) => cleanMeaning(meaning?.tranCn));
  return normalizePos(first?.pos);
}

function setNestedPrepFields(item, cnDistractors, enDistractors, meta) {
  item.cnChoiceDistractors = cnDistractors;
  item.enChoiceDistractors = enDistractors;
  item.displayWord = meta.displayWord;
  item.questionPrepMeta = meta;
  const wordNode = getWordNode(item);
  if (wordNode && typeof wordNode === 'object') {
    wordNode.cnChoiceDistractors = cnDistractors;
    wordNode.enChoiceDistractors = enDistractors;
    wordNode.displayWord = meta.displayWord;
    wordNode.questionPrepMeta = meta;
  }
  const content = getContent(item);
  if (content && typeof content === 'object') {
    content.cnChoiceDistractors = cnDistractors;
    content.enChoiceDistractors = enDistractors;
    content.displayWord = meta.displayWord;
    content.questionPrepMeta = meta;
  }
}

function cleanEntryInPlace(item) {
  const content = getContent(item);
  if (!content || typeof content !== 'object') return;
  if (Array.isArray(content.trans)) {
    for (const entry of content.trans) {
      if (typeof entry?.tranCn === 'string') entry.tranCn = cleanMeaning(entry.tranCn);
    }
  }
  for (const phrase of content?.phrase?.phrases ?? []) {
    if (typeof phrase?.pCn === 'string') phrase.pCn = cleanMeaning(phrase.pCn);
  }
  for (const sentence of content?.sentence?.sentences ?? []) {
    if (typeof sentence?.sCn === 'string') sentence.sCn = cleanMeaning(sentence.sCn);
  }
  for (const rel of content?.relWord?.rels ?? []) {
    for (const word of rel?.words ?? []) {
      if (typeof word?.tran === 'string') word.tran = cleanMeaning(word.tran);
    }
  }
}

function rankCandidates(target, pool, tier) {
  return pool
    .filter((candidate) => candidate.item !== target.item)
    .filter((candidate) => candidate.key !== target.key)
    .filter((candidate) => !candidateLooksEquivalent(target, candidate))
    .map((candidate) => ({
      candidate,
      tier,
      score: overlapScore(target.meaning, candidate.meaning),
      distance: Math.abs(candidate.rank - target.rank),
    }))
    .sort((left, right) => {
      if (right.score !== left.score) return right.score - left.score;
      if (left.distance !== right.distance) return left.distance - right.distance;
      return left.candidate.word.localeCompare(right.candidate.word);
    });
}

function appendRanked(target, ranked, cn, en, seenCn, seenEn, sources) {
  for (const item of ranked) {
    const cnText = cleanMeaning(item.candidate.meaning);
    const cnKey = meaningKey(cnText);
    const enText = item.candidate.word;
    const enKey = enText.toLowerCase();
    let used = false;
    if (cn.length < maxDistractors && cnKey && !seenCn.has(cnKey)) {
      seenCn.add(cnKey);
      cn.push(cnText);
      used = true;
    }
    if (en.length < maxDistractors && enKey && !seenEn.has(enKey)) {
      seenEn.add(enKey);
      en.push(enText);
      used = true;
    }
    if (used) sources.add(item.tier);
    if (cn.length >= maxDistractors && en.length >= maxDistractors) break;
  }
}

const bookFiles = fs.readdirSync(bookDir).filter((name) => name.endsWith('.json')).sort();
const books = new Map();
const candidatesByBook = new Map();

for (const fileName of bookFiles) {
  const bookPath = path.join(bookDir, fileName);
  const entries = readJson(bookPath);
  const bookCode = path.basename(fileName, '.json');
  books.set(bookCode, { fileName, bookPath, entries });
  candidatesByBook.set(
    bookCode,
    entries
      .map((item, index) => {
        cleanEntryInPlace(item);
        const pos = primaryPos(item);
        return {
          item,
          index,
          book: bookCode,
          word: displayWordForEntry(bookCode, item, pos),
          rawWord: String(item?.headWord ?? getWordNode(item)?.wordHead ?? '').trim(),
          pos,
          broadPos: broadPos(pos),
          meaning: primaryMeaning(item),
          key: meaningKey(primaryMeaning(item)),
          rank: Number(item?.wordRank ?? index + 1),
        };
      })
      .filter((candidate) => candidate.word && candidate.meaning && candidate.key),
  );
}

const allCandidates = [...candidatesByBook.values()].flat();
const report = {
  generatedAt: new Date().toISOString(),
  maxDistractors,
  minDistractors,
  books: {},
  sparseSameBookSamePos: [],
  incomplete: [],
};

for (const [bookCode, book] of books) {
  const candidates = candidatesByBook.get(bookCode) ?? [];
  let entriesWithSevenCn = 0;
  let entriesWithSevenEn = 0;
  let entriesWithMinimumCn = 0;
  let entriesWithMinimumEn = 0;
  let usedFallbackEntries = 0;

  for (const target of candidates) {
    const sameBookSamePos = candidates.filter((candidate) => candidate.pos === target.pos);
    const sameBookBroadPos = candidates.filter((candidate) => candidate.broadPos === target.broadPos);
    const crossBookSamePos = allCandidates.filter((candidate) => candidate.pos === target.pos);
    const crossBookBroadPos = allCandidates.filter((candidate) => candidate.broadPos === target.broadPos);

    const cn = [];
    const en = [];
    const seenCn = new Set([target.key]);
    const seenEn = new Set([target.word.toLowerCase()]);
    const sources = new Set();

    appendRanked(target, rankCandidates(target, sameBookSamePos, 'same-book-same-pos'), cn, en, seenCn, seenEn, sources);
    if (cn.length < maxDistractors || en.length < maxDistractors) {
      appendRanked(target, rankCandidates(target, crossBookSamePos, 'cross-book-same-pos'), cn, en, seenCn, seenEn, sources);
    }
    if (cn.length < maxDistractors || en.length < maxDistractors) {
      appendRanked(target, rankCandidates(target, sameBookBroadPos, 'same-book-broad-pos'), cn, en, seenCn, seenEn, sources);
    }
    if (cn.length < maxDistractors || en.length < maxDistractors) {
      appendRanked(target, rankCandidates(target, crossBookBroadPos, 'cross-book-broad-pos'), cn, en, seenCn, seenEn, sources);
    }
    if (cn.length < maxDistractors || en.length < maxDistractors) {
      appendRanked(target, rankCandidates(target, candidates, 'same-book-any-pos'), cn, en, seenCn, seenEn, sources);
    }
    if (cn.length < maxDistractors || en.length < maxDistractors) {
      appendRanked(target, rankCandidates(target, allCandidates, 'cross-book-any-pos'), cn, en, seenCn, seenEn, sources);
    }

    const sameBookSamePosDistractorCount = rankCandidates(target, sameBookSamePos, 'same-book-same-pos').length;
    if (sameBookSamePosDistractorCount < minDistractors) {
      report.sparseSameBookSamePos.push({
        book: bookCode,
        word: target.word,
        pos: target.pos || 'unknown',
        broadPos: target.broadPos,
        sameBookSamePosDistractors: sameBookSamePosDistractorCount,
        finalCnDistractors: cn.length,
        finalEnDistractors: en.length,
      });
    }
    if (cn.length < maxDistractors || en.length < maxDistractors) {
      report.incomplete.push({
        book: bookCode,
        word: target.word,
        pos: target.pos || 'unknown',
        cnDistractors: cn.length,
        enDistractors: en.length,
      });
    }

    const sourceList = [...sources];
    if (sourceList.some((source) => source !== 'same-book-same-pos')) usedFallbackEntries += 1;
    setNestedPrepFields(target.item, cn, en, {
      version: 2,
      maxDistractors,
      displayWord: target.word,
      primaryPos: target.pos || 'unknown',
      broadPos: target.broadPos,
      sources: sourceList,
    });

    if (cn.length >= maxDistractors) entriesWithSevenCn += 1;
    if (en.length >= maxDistractors) entriesWithSevenEn += 1;
    if (cn.length >= minDistractors) entriesWithMinimumCn += 1;
    if (en.length >= minDistractors) entriesWithMinimumEn += 1;
  }

  writeJson(book.bookPath, book.entries);
  report.books[bookCode] = {
    entries: book.entries.length,
    candidates: candidates.length,
    entriesWithMinimumCn,
    entriesWithMinimumEn,
    entriesWithSevenCn,
    entriesWithSevenEn,
    usedFallbackEntries,
  };
}

fs.mkdirSync(path.dirname(reportPath), { recursive: true });
writeJson(reportPath, report);
console.log(
  Object.entries(report.books)
    .map(
      ([book, summary]) =>
        `${book}: entries=${summary.entries} minCn=${summary.entriesWithMinimumCn} minEn=${summary.entriesWithMinimumEn} sevenCn=${summary.entriesWithSevenCn} sevenEn=${summary.entriesWithSevenEn} fallback=${summary.usedFallbackEntries}`,
    )
    .join('\n'),
);
console.log(
  `sparseSameBookSamePos=${report.sparseSameBookSamePos.length} incomplete=${report.incomplete.length} report=${reportPath}`,
);
