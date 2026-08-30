import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { collectExamCorpus } from './build-exam-corpus-dictionary.mjs';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const EXAM_DIR = path.join(ROOT, 'apps/flutter_mobile/assets/exam-papers');
const DICTIONARY_PATH = path.join(
  ROOT,
  'apps/flutter_mobile/assets/exam-dictionary/exam-corpus-dictionary.json',
);
const REPORT_PATH = path.join(ROOT, 'docs/exam-corpus-dictionary-report.json');
const REQUIRED_BASIC_WORDS = [
  'a',
  'an',
  'and',
  'are',
  'as',
  'at',
  'be',
  'by',
  'for',
  'from',
  'had',
  'has',
  'have',
  'he',
  'her',
  'his',
  'i',
  'if',
  'in',
  'is',
  'it',
  'not',
  'of',
  'on',
  'or',
  'she',
  'that',
  'the',
  'their',
  'they',
  'this',
  'to',
  'was',
  'we',
  'were',
  'with',
  'you',
];
const REQUIRED_PHRASES = [
  'according to',
  'as far as',
  'for fear that',
  'in case that',
  'so long as',
  'turn out',
];

export function checkExamCorpusDictionary({
  examDir = EXAM_DIR,
  dictionaryPath = DICTIONARY_PATH,
  reportPath = REPORT_PATH,
} = {}) {
  const corpus = collectExamCorpus(examDir);
  const dictionary = JSON.parse(fs.readFileSync(dictionaryPath, 'utf8'));
  const report = JSON.parse(fs.readFileSync(reportPath, 'utf8'));
  const entries = new Map();
  for (const entry of dictionary.entries ?? []) {
    assert.ok(entry.term, 'dictionary entry term must not be empty');
    assert.ok(entry.meaning, `dictionary meaning must not be empty: ${entry.term}`);
    assert.ok(!entries.has(entry.term), `dictionary entry must be unique: ${entry.term}`);
    entries.set(entry.term, entry);
  }
  for (const word of REQUIRED_BASIC_WORDS) {
    if (corpus.words.has(word)) {
      assert.ok(entries.has(word), `missing basic exam word: ${word}`);
    }
  }
  for (const phrase of REQUIRED_PHRASES) {
    assert.ok(entries.has(phrase), `missing required exam phrase: ${phrase}`);
  }
  assert.equal(dictionary.coverage.corpusWordTypes, corpus.words.size);
  assert.equal(dictionary.coverage.corpusWordOccurrences, corpus.wordOccurrences);
  assert.equal(dictionary.coverage.uncoveredWordTypes, report.uncoveredWords.length);
  assert.ok(dictionary.coverage.matchedPhraseTypes >= 12000);
  assert.ok(dictionary.coverage.coveredWordOccurrences > 0);
  return {
    entries: entries.size,
    ...dictionary.coverage,
    wordOccurrenceCoveragePercent:
      (dictionary.coverage.coveredWordOccurrences /
        dictionary.coverage.corpusWordOccurrences) *
      100,
  };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  console.log(JSON.stringify(checkExamCorpusDictionary(), null, 2));
}
