import fs from 'node:fs';
import path from 'node:path';

const args = new Map();
for (let index = 2; index < process.argv.length; index += 1) {
  const [key, value] = process.argv[index].split('=', 2);
  if (key?.startsWith('--')) {
    args.set(key.slice(2), value ?? '');
  }
}

const bookDir = args.get('bookDir') ?? 'apps/mobile/android/app/src/main/assets/seed-vocab/book';
const overridesPath =
  args.get('overridesPath') ??
  'apps/mobile/android/app/src/main/assets/seed-vocab/example-overrides.json';
const reportPath = args.get('reportPath') ?? 'docs/seed-vocab-example-coverage.json';

function normalizePos(pos) {
  const value = String(pos ?? '').trim().replace(/\.+$/, '').toLowerCase();
  if (value.startsWith('v')) return 'v';
  if (value.startsWith('n')) return 'n';
  if (value.startsWith('adj') || value === 'a') return 'adj';
  if (value.startsWith('adv')) return 'adv';
  if (value.startsWith('prep')) return 'prep';
  if (value.startsWith('conj')) return 'conj';
  if (value.startsWith('pron')) return 'pron';
  return value;
}

function normalizeText(text) {
  return String(text ?? '').replace(/[\s,.;、。，；]/gu, '');
}

function overlapScore(translation, meaning) {
  const haystack = normalizeText(translation);
  const needle = normalizeText(meaning);
  if (!needle) return 0;
  if (haystack.includes(needle)) return 100 + needle.length;
  let score = 0;
  const haystackChars = new Set([...haystack]);
  for (const ch of needle) {
    if (haystackChars.has(ch)) score += 1;
  }
  return score;
}

function readJsonIfExists(filePath, fallback) {
  if (!fs.existsSync(filePath)) return fallback;
  const raw = fs.readFileSync(filePath, 'utf8').replace(/^\uFEFF/, '').trim();
  if (!raw) return fallback;
  return JSON.parse(raw);
}

const overrides = readJsonIfExists(overridesPath, {});
const issues = [];
const summary = {
  scannedBooks: 0,
  scannedEntries: 0,
  multiPosEntries: 0,
  entriesWithMissingPosExamples: 0,
  missingPosCount: 0,
};

for (const fileName of fs.readdirSync(bookDir).filter((name) => name.endsWith('.json')).sort()) {
  summary.scannedBooks += 1;
  const bookPath = path.join(bookDir, fileName);
  const entries = JSON.parse(fs.readFileSync(bookPath, 'utf8'));
  for (const item of entries) {
    summary.scannedEntries += 1;
    const word = String(item?.headWord ?? '');
    const wordId = String(item?.content?.word?.wordId ?? word);
    const content = item?.content?.word?.content ?? {};
    const meanings = Array.isArray(content.trans) ? content.trans : [];
    const targetPos = [
      ...new Set(meanings.map((meaning) => normalizePos(meaning?.pos)).filter(Boolean)),
    ].sort();
    if (targetPos.length < 2) continue;
    summary.multiPosEntries += 1;

    const examples = [
      ...(Array.isArray(content?.sentence?.sentences) ? content.sentence.sentences : []).map(
        (example) => ({
          sentenceEn: example?.sContent ?? '',
          sentenceCn: example?.sCn ?? '',
          source: 'sentence',
        }),
      ),
      ...(Array.isArray(content?.phrase?.phrases) ? content.phrase.phrases : []).map((phrase) => ({
        sentenceEn: phrase?.pContent ?? '',
        sentenceCn: phrase?.pCn ?? '',
        source: 'phrase',
      })),
      ...(Array.isArray(content?.realExamSentence?.sentences)
        ? content.realExamSentence.sentences
        : []
      ).map((example) => ({
        sentenceEn: example?.sContent ?? '',
        sentenceCn: example?.sCn ?? '',
        source: 'realExamSentence',
      })),
    ];
    const overrideExamples = Array.isArray(overrides?.[wordId]?.examples)
      ? overrides[wordId].examples
      : [];
    const normalizedOverrideExamples = overrideExamples.map((example) => ({
      ...example,
      sentenceEn: example?.sentenceEn ?? '',
      sentenceCn: example?.sentenceCn ?? '',
      source: 'override',
    }));
    const covered = new Set();
    for (const meaning of meanings) {
      const pos = normalizePos(meaning?.pos);
      if (!pos) continue;
      if (normalizedOverrideExamples.some((example) => normalizePos(example?.pos) === pos)) {
        covered.add(pos);
        continue;
      }
      const meaningText = String(meaning?.tranCn ?? '');
      for (const example of [...examples, ...normalizedOverrideExamples]) {
        const sentenceCn = example?.sentenceCn ?? '';
        if (overlapScore(sentenceCn, meaningText) > 0) {
          covered.add(pos);
          break;
        }
      }
    }

    const missingPos = targetPos.filter((pos) => !covered.has(pos));
    if (missingPos.length === 0) continue;

    summary.entriesWithMissingPosExamples += 1;
    summary.missingPosCount += missingPos.length;
    issues.push({
      book: path.basename(fileName, '.json'),
      wordId,
      word,
      declaredPos: targetPos,
      coveredPos: [...covered].sort(),
      missingPos,
      meanings: meanings.map((meaning) => ({
        pos: normalizePos(meaning?.pos),
        rawPos: String(meaning?.pos ?? ''),
        meaningCn: String(meaning?.tranCn ?? ''),
      })),
      examples,
      hasOverride: Boolean(overrides?.[wordId]),
    });
  }
}

const report = {
  generatedAt: new Date().toISOString(),
  summary,
  issues,
};

fs.mkdirSync(path.dirname(reportPath), { recursive: true });
fs.writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
console.log(
  `books=${summary.scannedBooks} entries=${summary.scannedEntries} multiPos=${summary.multiPosEntries} entriesMissing=${summary.entriesWithMissingPosExamples} missingPos=${summary.missingPosCount}`,
);
console.log(`report=${reportPath}`);
