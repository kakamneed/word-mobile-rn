import fs from 'node:fs';
import path from 'node:path';

const bookDir = 'apps/mobile/android/app/src/main/assets/seed-vocab/book';
const reportPath = 'docs/seed-vocab-choice-conflict-repair-report.json';
const requiredBooks = ['CET4_3.json', 'CET6_3.json', 'KaoYan_3.json', 'MEDICAL_RESP.json'];
const maxDistractors = 7;
const supplementalDistractors = [
  ['although', 'conj', '\u867d\u7136'],
  ['because', 'conj', '\u56e0\u4e3a'],
  ['whether', 'conj', '\u662f\u5426'],
  ['once', 'conj', '\u4e00\u65e6'],
  ['before', 'conj', '\u4e4b\u524d'],
  ['after', 'conj', '\u4e4b\u540e'],
  ['until', 'conj', '\u76f4\u5230'],
  ['whenever', 'conj', '\u65e0\u8bba\u4f55\u65f6'],
  ['wherever', 'conj', '\u65e0\u8bba\u4f55\u5904'],
  ['above', 'prep', '\u4e0a\u65b9'],
  ['below', 'prep', '\u4e0b\u65b9'],
  ['beyond', 'prep', '\u8d85\u51fa'],
  ['beneath', 'prep', '\u4e0b\u9762'],
  ['beside', 'prep', '\u65c1\u8fb9'],
  ['throughout', 'prep', '\u904d\u53ca'],
  ['unlike', 'prep', '\u4e0d\u50cf'],
  ['toward', 'prep', '\u671d\u5411'],
  ['against', 'prep', '\u53cd\u5bf9'],
  ['someone', 'pron', '\u67d0\u4eba'],
  ['nobody', 'pron', '\u6ca1\u6709\u4eba'],
  ['others', 'pron', '\u5176\u4ed6\u4eba'],
  ['either', 'pron', '\u4efb\u4e00\u4e2a'],
  ['neither', 'pron', '\u4e24\u8005\u90fd\u4e0d'],
  ['each', 'pron', '\u6bcf\u4e2a'],
  ['both', 'pron', '\u4e24\u8005\u90fd'],
  ['hello', 'int', '\u4f60\u597d'],
  ['alas', 'int', '\u5509'],
  ['bravo', 'int', '\u597d\u554a'],
  ['ouch', 'int', '\u54ce\u54df'],
  ['wow', 'int', '\u54c7'],
  ['hey', 'int', '\u563f'],
  ['cheers', 'int', '\u5e72\u676f'],
  ['hush', 'int', '\u5618'],
];

const splitRe = /[;,\s/\?\uff0c\uff1b\u3001\u3002\uff1a\uff1f]+/u;
const commonHan = new Set(
  Array.from(
    '\u7684\u5730\u5f97\u4e00\u4e8c\u4e09\u56db\u4e94\u516d\u4e03\u516b\u4e5d\u5341\u4e2a\u79cd\u4eba\u4e8b\u7269\u8005\u6027\u5316\u6cd5\u884c\u4f5c\u7528\u6709\u65e0\u4e0d\u975e\u4e0e\u548c\u53ca\u6216\u5728\u4e8e\u4e3a\u4ee5\u4f7f\u88ab\u628a\u7ed9\u5bf9\u4e2d\u4e0a\u4e0b\u90e8\u4f53\u7b49\u8fdb\u884c\u8868\u793a\u4ea7\u751f\u53d1\u751f\u6210\u4e3a'
  )
);

const meaningOverrides = new Map(Object.entries({
  reform: '\u6539\u9769\uff1b\u6539\u826f',
  tow: '\u62d6\u66f3\uff1b\u7275\u5f15',
  dread: '\u6050\u60e7\uff1b\u754f\u60e7',
  boycott: '\u62b5\u5236\uff1b\u8054\u5408\u62b5\u5236',
  curse: '\u8bc5\u5492\uff1b\u5492\u9a82',
  nonetheless: '\u7136\u800c\uff1b\u5c3d\u7ba1\u5982\u6b64',
  twinkle: '\u95ea\u70c1\uff1b\u95ea\u5149',
  glide: '\u6ed1\u884c\uff1b\u6ed1\u52a8',
  auction: '\u62cd\u5356',
  revolt: '\u53cd\u6297\uff1b\u53db\u4e71',
  slap: '\u638c\u63b4\uff1b\u62cd\u51fb',
  arrest: '\u902e\u6355\uff1b\u62d8\u7559',
  fore: '\u524d\u9762\u7684\uff1b\u524d\u90e8\u7684',
  gaze: '\u51dd\u89c6\uff1b\u6ce8\u89c6',
  exile: '\u6d41\u653e\uff1b\u6d41\u4ea1',
  compute: '\u8ba1\u7b97',
  shiver: '\u98a4\u6296\uff1b\u53d1\u6296',
  hug: '\u62e5\u62b1',
  envy: '\u5ac9\u5992\uff1b\u7fa1\u6155',
  blush: '\u8138\u7ea2\uff1b\u7f9e\u6127',
  collapse: '\u5012\u584c\uff1b\u5d29\u6e83',
  debate: '\u8fa9\u8bba\uff1b\u4e89\u8bba',
  assault: '\u653b\u51fb\uff1b\u88ad\u51fb',
  giggle: '\u54af\u54af\u7b11\uff1b\u50bb\u7b11',
  glimpse: '\u4e00\u77a5\uff1b\u77a5\u89c1',
  stroll: '\u6563\u6b65\uff1b\u95f2\u901b',
  cease: '\u505c\u6b62\uff1b\u7ec8\u6b62',
}));
const semanticGroups = loadAuditSemanticGroups();

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, 'utf8').replace(/^\uFEFF/, ''));
}

function loadAuditSemanticGroups() {
  const auditPath = 'scripts/audit-seed-vocab-choice-conflicts.mjs';
  const raw = fs.readFileSync(auditPath, 'utf8');
  const match = raw.match(/const groups = \[([\s\S]*?)\]\.map/u);
  if (!match) return [];
  return Function(`return [${match[1]}];`)().map((line) => line.split(/\s+/u));
}

function writeJson(filePath, value) {
  fs.writeFileSync(filePath, `${JSON.stringify(value)}\n`, 'utf8');
}

function clean(text) {
  return String(text ?? '')
    .replace(/[<>\uFFFD]/gu, '')
    .replace(/^[a-z.]+\s+/iu, '')
    .replace(/\s+/gu, ' ')
    .trim();
}

function getWordNode(item) {
  return item?.content?.word ?? {};
}

function getContent(item) {
  return getWordNode(item)?.content ?? {};
}

function displayWord(item) {
  return String(
    item?.displayWord ??
      getWordNode(item)?.displayWord ??
      getContent(item)?.displayWord ??
      item?.headWord ??
      getWordNode(item)?.wordHead ??
      ''
  ).trim();
}

function normWord(word) {
  return String(word ?? '').toLowerCase().replace(/[^a-z0-9]/gu, '');
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
  if (
    [
      'effect',
      'dioxide',
      'hormone',
      'failure',
      'dose',
      'concentration',
      'therapy',
      'index',
      'computed',
      'magnetic',
    ].includes(value)
  ) {
    return 'n';
  }
  return value;
}

function primaryPos(item) {
  const entry = (getContent(item).trans ?? []).find((candidate) => clean(candidate?.tranCn));
  const meaning = clean(entry?.tranCn);
  const embedded = meaning.match(/^(n|v|vt|vi|adj|adv|prep|conj|pron|int)\./iu)?.[1];
  return normalizePos(embedded ?? entry?.pos);
}

function meanings(item) {
  const values = (getContent(item).trans ?? [])
    .map((entry) => clean(entry?.tranCn))
    .filter((value) => /[\u4e00-\u9fff]/u.test(value));
  if (values.length) return values;
  const override = meaningOverrides.get(normWord(displayWord(item)));
  return override ? [override] : [];
}

function meaningKey(text) {
  return clean(text).replace(/[\s,.;:/\?\uff0c\u3002\uff1b\uff1a\u3001\uff1f]/gu, '').toLowerCase();
}

function hanChunks(text) {
  return clean(text).match(/[\u4e00-\u9fff]+/gu) ?? [];
}

function ngrams(chunk, size) {
  const out = [];
  const chars = Array.from(chunk);
  for (let i = 0; i <= chars.length - size; i += 1) {
    out.push(chars.slice(i, i + size).join(''));
  }
  return out;
}

function profile(text) {
  const cleaned = clean(text);
  const tokens = cleaned
    .split(splitRe)
    .map((part) => part.trim())
    .filter((part) => part.length >= 2);
  const grams = new Set();
  const rareChars = new Set();
  for (const chunk of hanChunks(cleaned)) {
    for (const gram of ngrams(chunk, 2)) grams.add(gram);
    for (const gram of ngrams(chunk, 3)) grams.add(gram);
    for (const ch of Array.from(chunk)) {
      if (!commonHan.has(ch)) rareChars.add(ch);
    }
  }
  return { text: cleaned, key: meaningKey(cleaned), tokens, grams, rareChars };
}

function intersects(left, right) {
  for (const value of left) if (right.has(value)) return true;
  return false;
}

function textProfilesConflict(left, right) {
  if (!left.key || !right.key) return false;
  if (sameSemanticGroup(left.text, right.text)) return true;
  if (left.key === right.key) return true;
  if (left.key.length >= 2 && right.key.length >= 2) {
    if (left.key.includes(right.key) || right.key.includes(left.key)) return true;
  }
  for (const token of left.tokens) {
    if (token.length >= 2 && right.key.includes(meaningKey(token))) return true;
  }
  for (const token of right.tokens) {
    if (token.length >= 2 && left.key.includes(meaningKey(token))) return true;
  }
  if (intersects(left.grams, right.grams)) return true;
  return intersects(left.rareChars, right.rareChars);
}

function sameSemanticGroup(left, right) {
  const a = clean(left);
  const b = clean(right);
  if (!a || !b) return false;
  return semanticGroups.some(
    (group) =>
      group.some((token) => a.includes(token)) &&
      group.some((token) => b.includes(token))
  );
}

function wordTooSimilar(left, right) {
  const a = normWord(left);
  const b = normWord(right);
  if (!a || !b) return false;
  if (a === b) return true;
  const shorter = Math.min(a.length, b.length);
  return shorter >= 4 && (a.includes(b) || b.includes(a));
}

function setFields(item, choices, version) {
  const cn = choices.map((candidate) => candidate.meaning);
  const en = choices.map((candidate) => candidate.word);
  const meta = {
    ...(item.questionPrepMeta ?? {}),
    version,
    maxDistractors,
    strictSamePos: true,
    strictSemanticFilter: 'han-ngram-rare-char-v1',
    choiceDistractorSources: choices.map((candidate) => ({
      word: candidate.word,
      pos: candidate.pos || 'unknown',
      broadPos: candidate.pos || 'unknown',
      meaning: candidate.meaning,
      book: candidate.book,
    })),
  };
  item.cnChoiceDistractors = cn;
  item.enChoiceDistractors = en;
  item.questionPrepMeta = meta;
  item.choiceDistractorSources = meta.choiceDistractorSources;
  for (const node of [getWordNode(item), getContent(item)]) {
    if (!node || typeof node !== 'object') continue;
    node.cnChoiceDistractors = cn;
    node.enChoiceDistractors = en;
    node.questionPrepMeta = meta;
    node.choiceDistractorSources = meta.choiceDistractorSources;
  }
}

const books = [];
const candidates = [];
for (const fileName of requiredBooks) {
  const filePath = path.join(bookDir, fileName);
  const items = readJson(filePath);
  const book = path.basename(fileName, '.json');
  books.push({ book, fileName, filePath, items });
  for (const [index, item] of items.entries()) {
    const word = displayWord(item);
    const pos = primaryPos(item);
    const meaning = meanings(item)[0] ?? '';
    const allMeanings = meanings(item).join('\uff1b');
    if (!word || !pos || !meaning) continue;
    candidates.push({
      item,
      book,
      index,
      word,
      pos,
      meaning,
      allMeanings,
      rank: Number(item?.wordRank ?? index + 1),
      profile: profile(allMeanings),
    });
  }
}
for (const [index, [word, pos, meaning]] of supplementalDistractors.entries()) {
  candidates.push({
    item: null,
    book: 'SUPPLEMENTAL',
    index,
    word,
    pos,
    meaning,
    allMeanings: meaning,
    rank: 100000 + index,
    profile: profile(meaning),
  });
}

const byPos = new Map();
for (const candidate of candidates) {
  if (!byPos.has(candidate.pos)) byPos.set(candidate.pos, []);
  byPos.get(candidate.pos).push(candidate);
}

const report = {
  generatedAt: new Date().toISOString(),
  rule: 'han-ngram-rare-char-v1',
  books: {},
  repaired: 0,
  incomplete: [],
};

for (const target of candidates.filter((candidate) => candidate.item != null)) {
  const pool = byPos.get(target.pos) ?? [];
  const ordered = pool
    .filter((candidate) => candidate !== target)
    .sort((a, b) => {
      const sameBookA = a.book === target.book ? 0 : 1;
      const sameBookB = b.book === target.book ? 0 : 1;
      const supplementalA = a.item == null ? 1 : 0;
      const supplementalB = b.item == null ? 1 : 0;
      return (
        sameBookA - sameBookB ||
        supplementalA - supplementalB ||
        Math.abs(a.rank - target.rank) - Math.abs(b.rank - target.rank) ||
        a.word.localeCompare(b.word)
      );
    });

  const choices = [];
  const seenWords = new Set();
  const seenMeanings = new Set();
  for (const candidate of ordered) {
    if (choices.length >= maxDistractors) break;
    if (wordTooSimilar(target.word, candidate.word)) continue;
    if (textProfilesConflict(target.profile, candidate.profile)) continue;
    if (choices.some((choice) => textProfilesConflict(choice.profile, candidate.profile))) continue;
    const wordKey = normWord(candidate.word);
    const meaning = meaningKey(candidate.meaning);
    if (!wordKey || !meaning || seenWords.has(wordKey) || seenMeanings.has(meaning)) continue;
    choices.push(candidate);
    seenWords.add(wordKey);
    seenMeanings.add(meaning);
  }

  setFields(target.item, choices, 5);
  report.repaired += 1;
  if (choices.length < maxDistractors) {
    report.incomplete.push({
      book: target.book,
      word: target.word,
      pos: target.pos,
      count: choices.length,
    });
  }
}

for (const book of books) {
  const entries = candidates.filter((candidate) => candidate.book === book.book);
  report.books[book.fileName] = {
    entries: book.items.length,
    repaired: entries.length,
    incomplete: report.incomplete.filter((item) => item.book === book.book).length,
  };
  writeJson(book.filePath, book.items);
}

fs.mkdirSync(path.dirname(reportPath), { recursive: true });
fs.writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
console.log(JSON.stringify(report.books, null, 2));
console.log(`repaired=${report.repaired} incomplete=${report.incomplete.length} report=${reportPath}`);
if (report.incomplete.length > 0) process.exitCode = 1;
