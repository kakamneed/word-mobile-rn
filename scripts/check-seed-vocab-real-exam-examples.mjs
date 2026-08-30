import fs from 'node:fs';
import path from 'node:path';

const bookDir = 'apps/mobile/android/app/src/main/assets/seed-vocab/book';
const reportPath = 'docs/seed-vocab-real-exam-examples-report.json';
const requiredBooks = ['CET4_3.json', 'CET6_3.json', 'KaoYan_3.json', 'MEDICAL_RESP.json'];
const allowedExamsByBook = new Map([
  ['CET4_3.json', new Set(['cet4'])],
  ['CET6_3.json', new Set(['cet6'])],
  ['KaoYan_3.json', new Set(['kaoyan-english-1', 'kaoyan-english-2'])],
  ['MEDICAL_RESP.json', new Set()],
]);
const maxExamplesPerWord = 8;

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, 'utf8').replace(/^\uFEFF/, ''));
}

function cleanText(value) {
  return String(value ?? '').replace(/\s+/gu, ' ').trim();
}

function displayWord(item) {
  return String(item?.displayWord ?? item?.headWord ?? item?.content?.word?.wordHead ?? '').trim();
}

function normalizeWord(word) {
  return cleanText(word).toLowerCase().replace(/[^a-z0-9 -]/gu, '').replace(/\s+/gu, ' ').trim();
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/gu, '\\$&');
}

function inflectedForms(word) {
  const normalized = normalizeWord(word);
  if (!normalized) return [];
  if (normalized.includes(' ')) return [normalized];
  const forms = new Set([normalized]);
  if (normalized.length > 2) forms.add(`${normalized}s`);
  if (normalized.endsWith('y') && normalized.length > 2) forms.add(`${normalized.slice(0, -1)}ies`);
  if (normalized.endsWith('e') && normalized.length > 3) {
    forms.add(`${normalized}d`);
    forms.add(`${normalized.slice(0, -1)}ing`);
  } else if (normalized.length > 2) {
    forms.add(`${normalized}ed`);
    forms.add(`${normalized}ing`);
  }
  return [...forms].sort((a, b) => b.length - a.length);
}

function patternFor(word, content) {
  const frequencyForms = Object.keys(content?.realExamFrequency?.surfaceForms ?? {})
    .map(normalizeWord)
    .filter(Boolean);
  const forms = [...new Set([...inflectedForms(word), ...frequencyForms])];
  if (forms.length === 0) return null;
  return new RegExp(`(?<![A-Za-z])(?:${forms.map(escapeRegExp).join('|')})(?![A-Za-z])`, 'iu');
}

let failures = 0;
const summary = {};
for (const fileName of requiredBooks) {
  const entries = readJson(path.join(bookDir, fileName));
  const stats = {
    entries: entries.length,
    entriesWithRealExamExamples: 0,
    examples: 0,
    tooMany: 0,
    empty: 0,
    sourceMissing: 0,
    targetMissing: 0,
    duplicateSentence: 0,
    crossExam: 0,
  };
  const allowedExams = allowedExamsByBook.get(fileName) ?? new Set();
  for (const item of entries) {
    const word = displayWord(item);
    const content = item?.content?.word?.content ?? {};
    const pattern = patternFor(word, content);
    const examples = content.realExamSentence?.sentences ?? [];
    if (!Array.isArray(examples) || examples.length === 0) continue;
    stats.entriesWithRealExamExamples += 1;
    stats.examples += examples.length;
    if (examples.length > maxExamplesPerWord) stats.tooMany += 1;
    const seen = new Set();
    for (const example of examples) {
      const sentence = cleanText(example?.sContent);
      if (!sentence) stats.empty += 1;
      if (seen.has(sentence.toLowerCase())) stats.duplicateSentence += 1;
      seen.add(sentence.toLowerCase());
      if (!example?.source?.exam || !example?.source?.paperId || !example?.source?.sectionId) {
        stats.sourceMissing += 1;
      }
      if (!allowedExams.has(example?.source?.exam)) {
        stats.crossExam += 1;
      }
      if (pattern && !pattern.test(sentence)) {
        stats.targetMissing += 1;
      }
    }
  }
  summary[fileName] = stats;
  console.log(
    `${fileName}: entries=${stats.entries} matched=${stats.entriesWithRealExamExamples} examples=${stats.examples} tooMany=${stats.tooMany} empty=${stats.empty} sourceMissing=${stats.sourceMissing} targetMissing=${stats.targetMissing} duplicateSentence=${stats.duplicateSentence}`,
  );
  console.log(`  allowedExams=${[...allowedExams].join(',') || '(none)'} crossExam=${stats.crossExam}`);
  if (stats.tooMany || stats.empty || stats.sourceMissing || stats.targetMissing || stats.duplicateSentence || stats.crossExam) {
    failures += 1;
  }
}

if (fs.existsSync(reportPath)) {
  const report = readJson(reportPath);
  for (const [fileName, stats] of Object.entries(summary)) {
    const reported = report.books?.[fileName];
    if (!reported || reported.matchedEntries !== stats.entriesWithRealExamExamples || reported.writtenExamples !== stats.examples) {
      console.error(`report mismatch for ${fileName}`);
      failures += 1;
    }
  }
} else {
  console.error(`missing report: ${reportPath}`);
  failures += 1;
}

if (failures > 0) process.exitCode = 1;
