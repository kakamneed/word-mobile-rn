import fs from 'node:fs';
import path from 'node:path';

const bookDir = 'apps/mobile/android/app/src/main/assets/seed-vocab/book';
const reportPath = 'docs/seed-vocab-real-exam-meaning-repair-report.json';

const patches = [
  {
    book: 'CET4_3.json',
    word: 'mobile',
    pos: 'adj',
    additions: ['手机的', '移动通信的'],
    reason: 'CET4 exam examples use mobile in mobile phone/apps/network contexts.',
  },
  {
    book: 'CET6_3.json',
    word: 'stock',
    pos: 'v',
    additions: ['备有', '进货'],
    reason: 'CET6 exam examples include supermarket/company stock as a verb meaning keep goods for sale.',
  },
  {
    book: 'CET6_3.json',
    word: 'stock',
    pos: 'n',
    additions: ['股票', '库存'],
    reason: 'CET6 exam examples include stock market and burnt stock contexts.',
  },
  {
    book: 'CET6_3.json',
    word: 'cancel',
    pos: 'vt',
    additions: ['免除', '抵消'],
    reason: 'CET6 exam examples include cancel student debt, an extended sense beyond event cancellation.',
  },
  {
    book: 'KaoYan_3.json',
    word: 'explosive',
    pos: 'adj',
    additions: ['激烈的', '紧张的', '一触即发的'],
    reason: 'Kaoyan example uses explosive situation, not literal physical explosion.',
  },
  {
    book: 'KaoYan_3.json',
    word: 'collapse',
    pos: 'n',
    additions: ['瓦解', '失败'],
    reason: 'Kaoyan examples include political, institutional, and operational collapse.',
  },
  {
    book: 'KaoYan_3.json',
    word: 'collapse',
    pos: 'v',
    additions: ['瓦解', '崩盘', '失败'],
    reason: 'Kaoyan examples include industry/integrity had collapsed.',
  },
];

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, 'utf8').replace(/^\uFEFF/, ''));
}

function writeJson(filePath, value) {
  const payload = `${JSON.stringify(value)}\n`;
  const tmp = `${filePath}.${process.pid}.tmp`;
  let lastError;
  for (let attempt = 0; attempt < 8; attempt += 1) {
    try {
      fs.writeFileSync(tmp, payload, 'utf8');
      fs.renameSync(tmp, filePath);
      return;
    } catch (error) {
      lastError = error;
      try {
        if (fs.existsSync(tmp)) fs.unlinkSync(tmp);
      } catch {}
      Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 120 * (attempt + 1));
    }
  }
  throw lastError;
}

function clean(value) {
  return String(value ?? '').replace(/\s+/gu, ' ').trim();
}

function displayWord(item) {
  return String(item?.displayWord ?? item?.headWord ?? item?.content?.word?.wordHead ?? '').trim();
}

function normalizePos(pos) {
  const value = String(pos ?? '').trim().replace(/\.+$/u, '').toLowerCase();
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

function terms(value) {
  return clean(value)
    .replace(/^[a-z.]+\s+/iu, '')
    .split(/[;,，；、。/]/u)
    .map((term) => term.trim())
    .filter(Boolean);
}

function appendTerms(text, additions) {
  const existing = terms(text);
  const seen = new Set(existing);
  for (const addition of additions) {
    if (!seen.has(addition)) {
      existing.push(addition);
      seen.add(addition);
    }
  }
  return existing.join('；');
}

const patchesByBook = new Map();
for (const patch of patches) {
  if (!patchesByBook.has(patch.book)) patchesByBook.set(patch.book, []);
  patchesByBook.get(patch.book).push(patch);
}

const report = { generatedAt: new Date().toISOString(), books: {}, patches: [], misses: [] };
for (const [book, bookPatches] of patchesByBook.entries()) {
  const filePath = path.join(bookDir, book);
  const entries = readJson(filePath);
  let applied = 0;
  for (const patch of bookPatches) {
    const item = entries.find((entry) => displayWord(entry).toLowerCase() === patch.word.toLowerCase());
    const content = item?.content?.word?.content;
    if (!content) {
      report.misses.push({ ...patch, reason: 'entry not found' });
      continue;
    }
    const normalizedPatchPos = normalizePos(patch.pos);
    const trans = Array.isArray(content.trans) ? content.trans : [];
    let target = trans.find((entry) => normalizePos(entry?.pos) === normalizedPatchPos);
    if (!target) {
      target = { tranCn: '', pos: patch.pos, descCn: '中释' };
      trans.push(target);
      content.trans = trans;
    }
    const before = clean(target.tranCn);
    const after = appendTerms(before, patch.additions);
    if (after !== before) {
      target.tranCn = after;
      if (!target.descCn) target.descCn = '中释';
      const meta = content.realExamMeaningPatches ?? [];
      meta.push({
        word: patch.word,
        pos: patch.pos,
        additions: patch.additions,
        reason: patch.reason,
        appliedAt: new Date().toISOString(),
      });
      content.realExamMeaningPatches = meta;
      applied += 1;
      report.patches.push({ book, word: patch.word, pos: patch.pos, before, after, additions: patch.additions, reason: patch.reason });
    } else {
      report.patches.push({ book, word: patch.word, pos: patch.pos, before, after, additions: [], reason: 'already covered' });
    }
  }
  writeJson(filePath, entries);
  report.books[book] = { entries: entries.length, requested: bookPatches.length, applied };
}

fs.mkdirSync(path.dirname(reportPath), { recursive: true });
writeJson(reportPath, report);
console.log(JSON.stringify(report.books, null, 2));
console.log(`patches=${report.patches.length} misses=${report.misses.length} report=${reportPath}`);
if (report.misses.length > 0) process.exitCode = 1;
