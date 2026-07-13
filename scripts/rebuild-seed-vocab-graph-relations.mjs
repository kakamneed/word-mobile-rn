import fs from 'node:fs';
import path from 'node:path';

const args = new Map();
for (let index = 2; index < process.argv.length; index += 1) {
  const [key, value] = process.argv[index].split('=', 2);
  if (key?.startsWith('--')) args.set(key.slice(2), value ?? '');
}

const bookDir = args.get('bookDir') ?? 'apps/mobile/android/app/src/main/assets/seed-vocab/book';
const reportPath = args.get('reportPath') ?? 'docs/seed-vocab-graph-relations-report.json';
const maxRootFamilyWords = Number(args.get('maxRootFamilyWords') ?? 12);
const maxSimilarFormWords = Number(args.get('maxSimilarFormWords') ?? 12);
const maxMeaningOverlapWords = Number(args.get('maxMeaningOverlapWords') ?? 12);
const maxRootAffixes = Number(args.get('maxRootAffixes') ?? 8);

const STOP_MEANING_TOKENS = new Set([
  '的', '地', '得', '使', '被', '把', '和', '与', '及', '或', '等', '在', '有', '无', '不', '非',
  '某', '一种', '一个', '东西', '事情', '人', '物', '进行', '表示', '用于', '关于', '…', 'n', 'v',
  'adj', 'adv', 'vi', 'vt', 'prep', 'conj', 'pron', 'int', 'a', 'the', 'of', 'to', 'and', 'or',
]);
const ROOT_STOP_FORMS = new Set([
  'a', 'an', 'the', 'e', 'i', 'o', 'en', 'er', 'or', 'on', 'al', 'ly', 'y', 's', 'es', 'ed', 'ing',
  'ous', 'ious', 'tion', 'sion', 'ment', 'ness', 'able', 'ible', 'ance', 'ence', 'ive', 'ize', 'ise',
  'word', 'words', 'root', 'look', 'see', 'space', 'like', 'as', 'to', 'do', 'make', 'take', 'one',
]);
const GRAPH_ROOT_STOP_FORMS = new Set([
  'ate', 'ation', 'tion', 'sion', 'ment', 'ness', 'able', 'ible', 'ance', 'ence', 'ive', 'ous',
  'ious', 'ical', 'al', 'er', 'or', 'ic', 'ty', 'ity', 'ize', 'ise', 're', 'un', 'dis', 'de',
  'con', 'com', 'co', 'pro', 'pre', 'sub', 'inter', 'trans',
]);
const COMMON_PREFIXES = [
  ['anti', '反；抗'], ['auto', '自己；自动'], ['bio', '生命'], ['circum', '周围'], ['co', '共同'],
  ['com', '共同'], ['con', '共同；加强'], ['contra', '反对'], ['de', '向下；除去'], ['dis', '否定；分离'],
  ['extra', '额外'], ['geo', '地球；土地'], ['inter', '在…之间'], ['macro', '大'], ['micro', '小'],
  ['multi', '多'], ['post', '后'], ['pre', '前'], ['pro', '向前；支持'], ['re', '再；回'], ['semi', '半'],
  ['sub', '下；次'], ['super', '上；超'], ['tele', '远'], ['trans', '横过；转移'], ['tri', '三'], ['un', '不；反'],
];
const COMMON_SUFFIXES = [
  ['able', '可…的'], ['ible', '可…的'], ['al', '…的'], ['ance', '性质；状态'], ['ence', '性质；状态'],
  ['er', '人；物'], ['or', '人；物'], ['ic', '…的'], ['ical', '…的'], ['ise', '使；变成'], ['ize', '使；变成'],
  ['less', '无…的'], ['ment', '行为；结果'], ['ness', '性质；状态'], ['ous', '…的'], ['sion', '动作；状态'],
  ['tion', '动作；状态'], ['tive', '…的'], ['ty', '性质'], ['ity', '性质'],
];

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, 'utf8').replace(/^\uFEFF/, ''));
}
function writeJson(filePath, value) {
  fs.writeFileSync(filePath, `${JSON.stringify(value)}\n`, 'utf8');
}
function getWordNode(item) {
  return item?.content?.word ?? {};
}
function getContent(item) {
  return getWordNode(item)?.content ?? {};
}
function cleanMeaning(text) {
  return String(text ?? '')
    .replace(/[<>]/gu, '')
    .replace(/\uFFFD/gu, '')
    .replace(/\s+/gu, ' ')
    .trim();
}
function displayWord(item) {
  return String(item?.displayWord ?? getWordNode(item)?.displayWord ?? getContent(item)?.displayWord ?? item?.headWord ?? getWordNode(item)?.wordHead ?? '').trim();
}
function sourceKey(item, word) {
  return String(getWordNode(item)?.wordId ?? word).trim();
}
function primaryPos(item) {
  const trans = Array.isArray(getContent(item).trans) ? getContent(item).trans : [];
  return String(trans.find((entry) => cleanMeaning(entry?.tranCn))?.pos ?? '').trim().toLowerCase();
}
function allMeanings(item) {
  const out = [];
  const trans = Array.isArray(getContent(item).trans) ? getContent(item).trans : [];
  for (const entry of trans) {
    const value = cleanMeaning(entry?.tranCn).replace(/^[a-z.]+\s+/iu, '').trim();
    if (value) out.push(value);
  }
  return [...new Set(out)];
}
function primaryMeaning(item) {
  return allMeanings(item)[0] ?? '';
}
function normalizedWord(text) {
  return String(text ?? '').toLowerCase().replace(/[^a-z0-9]/gu, '');
}
function tokenizeMeaning(text) {
  const cleaned = cleanMeaning(text).toLowerCase().replace(/[a-z]+\.?/gu, ' ');
  const tokens = cleaned
    .split(/[\s,.;:，。；：、/（）()\[\]【】]+/u)
    .map((token) => token.trim())
    .filter((token) => token.length >= 2 && !STOP_MEANING_TOKENS.has(token));
  const charTokens = [];
  const chinese = cleaned.match(/[\p{Script=Han}]{2,}/gu) ?? [];
  for (const segment of chinese) {
    if (segment.length <= 4 && !STOP_MEANING_TOKENS.has(segment)) charTokens.push(segment);
    for (let index = 0; index + 1 < segment.length; index += 1) {
      const bigram = segment.slice(index, index + 2);
      if (!STOP_MEANING_TOKENS.has(bigram)) charTokens.push(bigram);
    }
  }
  return [...new Set([...tokens, ...charTokens])];
}
function extractRootAffixes(item) {
  const content = getContent(item);
  const rem = String(content?.remMethod?.val ?? content?.remMethod?.desc ?? '').trim();
  const word = displayWord(item);
  const normalized = normalizedWord(word);
  const out = [];
  const add = (form, meaning, source) => {
    const cleanForm = String(form ?? '').trim().toLowerCase().replace(/[^a-z]/gu, '');
    const cleanCn = cleanMeaning(meaning).replace(/^[：:]+/u, '').trim();
    if (cleanForm.length < 2 || cleanForm.length > 12) return;
    if (ROOT_STOP_FORMS.has(cleanForm)) return;
    if (normalized && !normalized.includes(cleanForm)) return;
    if (out.some((item) => item.form === cleanForm && item.kind === source)) return;
    out.push({ form: cleanForm, meaningCn: cleanCn, kind: source });
  };
  for (const match of rem.matchAll(/([A-Za-z]{2,12})\s*[（(]([^()（）]{1,18})[）)]/gu)) {
    add(match[1], match[2], 'remMethod');
  }
  for (const [prefix, meaning] of COMMON_PREFIXES) {
    if (normalized.startsWith(prefix) && normalized.length >= prefix.length + 3) add(prefix, meaning, 'prefix');
  }
  for (const [suffix, meaning] of COMMON_SUFFIXES) {
    if (normalized.endsWith(suffix) && normalized.length >= suffix.length + 3) add(suffix, meaning, 'suffix');
  }
  return out.slice(0, maxRootAffixes);
}
function boundedLevenshtein(source, target, maxDistance) {
  if (Math.abs(source.length - target.length) > maxDistance) return null;
  let previous = Array.from({ length: target.length + 1 }, (_, index) => index);
  for (let i = 0; i < source.length; i += 1) {
    const current = [i + 1];
    let rowMin = current[0];
    for (let j = 0; j < target.length; j += 1) {
      const substitution = source[i] === target[j] ? 0 : 1;
      const value = Math.min(previous[j + 1] + 1, current[j] + 1, previous[j] + substitution);
      current.push(value);
      rowMin = Math.min(rowMin, value);
    }
    if (rowMin > maxDistance) return null;
    previous = current;
  }
  const distance = previous[previous.length - 1];
  return distance <= maxDistance ? distance : null;
}
function commonPrefixLength(left, right) {
  let index = 0;
  while (index < left.length && index < right.length && left[index] === right[index]) index += 1;
  return index;
}
function similarFormScore(left, right) {
  if (left === right || left.length < 4 || right.length < 4) return 0;
  const delta = Math.abs(left.length - right.length);
  if (delta > 4) return 0;
  const maxDistance = Math.max(2, Math.min(4, Math.floor(Math.max(left.length, right.length) / 4)));
  const distance = boundedLevenshtein(left, right, maxDistance);
  const prefix = commonPrefixLength(left, right);
  if (distance !== null && distance > 0) return Math.max(0.35, 1 - distance / 5);
  if (prefix >= 4) return Math.min(0.76, 0.35 + prefix / 20);
  return 0;
}
function rootOverlap(targetRoots, candidateRoots) {
  const out = [];
  for (const left of targetRoots) {
    if (GRAPH_ROOT_STOP_FORMS.has(left.form)) continue;
    for (const right of candidateRoots) {
      if (GRAPH_ROOT_STOP_FORMS.has(right.form)) continue;
      if (left.form === right.form) out.push(left);
    }
  }
  return [...new Map(out.map((item) => [item.form, item])).values()];
}
function meaningOverlapScore(leftTokens, rightTokens) {
  if (!leftTokens.length || !rightTokens.length) return { score: 0, shared: [] };
  const right = new Set(rightTokens);
  const shared = leftTokens.filter((token) => right.has(token));
  if (!shared.length) return { score: 0, shared: [] };
  const score = shared.length / Math.min(leftTokens.length, rightTokens.length);
  return { score, shared: shared.slice(0, 6) };
}
function relationKey(candidate) {
  return normalizedWord(candidate.word) || candidate.sourceId;
}

function relationTarget(candidate, extra = {}) {
  return {
    book: candidate.book,
    sourceId: candidate.sourceId,
    word: candidate.word,
    meaningCn: candidate.primaryMeaning,
    ...extra,
  };
}

const bookFiles = fs.readdirSync(bookDir).filter((name) => name.endsWith('.json')).sort();
const books = [];
const entries = [];
for (const fileName of bookFiles) {
  const bookPath = path.join(bookDir, fileName);
  const items = readJson(bookPath);
  const book = path.basename(fileName, '.json');
  books.push({ book, fileName, bookPath, items });
  for (const [index, item] of items.entries()) {
    const word = displayWord(item);
    if (!word) continue;
    const meanings = allMeanings(item);
    const primary = meanings[0] ?? '';
    const roots = extractRootAffixes(item);
    entries.push({
      item,
      index,
      book,
      sourceId: sourceKey(item, word),
      word,
      normalized: normalizedWord(word),
      pos: primaryPos(item),
      meanings,
      primaryMeaning: primary,
      meaningTokens: tokenizeMeaning(meanings.join('；')),
      roots,
    });
  }
}

const byItem = new Map(entries.map((entry) => [entry.item, entry]));
const report = {
  generatedAt: new Date().toISOString(),
  books: {},
  totals: {
    entries: entries.length,
    entriesWithRootAffixes: 0,
    entriesWithRootFamilyWords: 0,
    entriesWithSimilarFormWords: 0,
    entriesWithMeaningOverlapWords: 0,
  },
  sparse: [],
};

for (const target of entries) {
  const rootFamilyWords = [];
  const similarFormWords = [];
  const meaningOverlapWords = [];
  for (const candidate of entries) {
    if (candidate === target) continue;
    if (candidate.book !== target.book) continue;
    if (!candidate.word || candidate.normalized === target.normalized) continue;
    const sameRoots = rootOverlap(target.roots, candidate.roots);
    if (sameRoots.length) {
      rootFamilyWords.push(relationTarget(candidate, {
        sharedRoots: sameRoots.map((root) => ({ form: root.form, meaningCn: root.meaningCn, kind: root.kind })),
        weight: Math.min(0.95, 0.5 + sameRoots.length * 0.12),
      }));
    }
    const formScore = similarFormScore(target.normalized, candidate.normalized);
    if (formScore > 0) {
      similarFormWords.push(relationTarget(candidate, {
        distance: boundedLevenshtein(target.normalized, candidate.normalized, 4),
        commonPrefix: commonPrefixLength(target.normalized, candidate.normalized),
        weight: Number(formScore.toFixed(3)),
      }));
    }
    const overlap = meaningOverlapScore(target.meaningTokens, candidate.meaningTokens);
    if (overlap.score >= 0.34 || overlap.shared.length >= 2) {
      meaningOverlapWords.push(relationTarget(candidate, {
        sharedMeaningTokens: overlap.shared,
        weight: Number(Math.min(0.92, 0.42 + overlap.score).toFixed(3)),
      }));
    }
  }
  const dedupeRelations = (items) => {
    const byWord = new Map();
    for (const item of items) {
      const key = normalizedWord(item.word) || item.sourceId;
      const current = byWord.get(key);
      if (!current || item.weight > current.weight) byWord.set(key, item);
    }
    return [...byWord.values()];
  };
  const rootFamilyDeduped = dedupeRelations(rootFamilyWords).sort((a, b) => b.weight - a.weight || a.word.localeCompare(b.word));
  const similarFormDeduped = dedupeRelations(similarFormWords).sort((a, b) => b.weight - a.weight || b.commonPrefix - a.commonPrefix || a.word.localeCompare(b.word));
  const meaningOverlapDeduped = dedupeRelations(meaningOverlapWords).sort((a, b) => b.weight - a.weight || b.sharedMeaningTokens.length - a.sharedMeaningTokens.length || a.word.localeCompare(b.word));
  const relations = {
    version: 1,
    rootAffixes: target.roots,
    rootFamilyWords: rootFamilyDeduped.slice(0, maxRootFamilyWords),
    similarFormWords: similarFormDeduped.slice(0, maxSimilarFormWords),
    meaningOverlapWords: meaningOverlapDeduped.slice(0, maxMeaningOverlapWords),
  };
  target.item.wordGraphRelations = relations;
  const wordNode = getWordNode(target.item);
  if (wordNode && typeof wordNode === 'object') wordNode.wordGraphRelations = relations;
  const content = getContent(target.item);
  if (content && typeof content === 'object') content.wordGraphRelations = relations;

  if (relations.rootAffixes.length) report.totals.entriesWithRootAffixes += 1;
  if (relations.rootFamilyWords.length) report.totals.entriesWithRootFamilyWords += 1;
  if (relations.similarFormWords.length) report.totals.entriesWithSimilarFormWords += 1;
  if (relations.meaningOverlapWords.length) report.totals.entriesWithMeaningOverlapWords += 1;
  if (!relations.rootAffixes.length && !relations.similarFormWords.length && !relations.meaningOverlapWords.length) {
    report.sparse.push({ book: target.book, word: target.word, meaningCn: target.primaryMeaning });
  }
}

for (const book of books) {
  const bookEntries = book.items.map((item) => byItem.get(item)).filter(Boolean);
  report.books[book.book] = {
    entries: book.items.length,
    entriesWithRootAffixes: bookEntries.filter((entry) => entry.item.wordGraphRelations?.rootAffixes?.length).length,
    entriesWithRootFamilyWords: bookEntries.filter((entry) => entry.item.wordGraphRelations?.rootFamilyWords?.length).length,
    entriesWithSimilarFormWords: bookEntries.filter((entry) => entry.item.wordGraphRelations?.similarFormWords?.length).length,
    entriesWithMeaningOverlapWords: bookEntries.filter((entry) => entry.item.wordGraphRelations?.meaningOverlapWords?.length).length,
  };
  writeJson(book.bookPath, book.items);
}

fs.mkdirSync(path.dirname(reportPath), { recursive: true });
writeJson(reportPath, report);
console.log(Object.entries(report.books).map(([book, stats]) => `${book}: entries=${stats.entries} roots=${stats.entriesWithRootAffixes} rootFamily=${stats.entriesWithRootFamilyWords} similar=${stats.entriesWithSimilarFormWords} meaning=${stats.entriesWithMeaningOverlapWords}`).join('\n'));
console.log(`sparse=${report.sparse.length} report=${reportPath}`);
