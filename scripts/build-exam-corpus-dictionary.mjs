import fs from 'node:fs';
import path from 'node:path';
import readline from 'node:readline';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const DEFAULT_EXAM_DIR = path.join(ROOT, 'apps/flutter_mobile/assets/exam-papers');
const DEFAULT_SEED_DIR = path.join(
  ROOT,
  'apps/mobile/android/app/src/main/assets/seed-vocab/book',
);
const DEFAULT_OUTPUT = path.join(
  ROOT,
  'apps/flutter_mobile/assets/exam-dictionary/exam-corpus-dictionary.json',
);
const DEFAULT_REPORT = path.join(ROOT, 'docs/exam-corpus-dictionary-report.json');
const EXAM_FILES = [
  'cet4.json',
  'cet6.json',
  'kaoyan-english-1.json',
  'kaoyan-english-2.json',
];

export function parseCsvLine(line) {
  const values = [];
  let value = '';
  let quoted = false;
  for (let index = 0; index < line.length; index += 1) {
    const character = line[index];
    if (character === '"') {
      if (quoted && line[index + 1] === '"') {
        value += '"';
        index += 1;
      } else {
        quoted = !quoted;
      }
    } else if (character === ',' && !quoted) {
      values.push(value);
      value = '';
    } else {
      value += character;
    }
  }
  values.push(value);
  return values;
}

export function tokenizeEnglish(text) {
  return [...String(text ?? '').matchAll(/[A-Za-z]+(?:['’-][A-Za-z]+)*/g)].map(
    (match) => match[0].toLowerCase().replaceAll('’', "'"),
  );
}

export function cleanDictionaryMeaning(value) {
  const lines = String(value ?? '')
    .replaceAll('\\r', ' ')
    .replaceAll('\\n', '\n')
    .split(/\r?\n/)
    .map((line) => line.replace(/\s+/g, ' ').trim())
    .filter((line) => line && !/^\[(网络|例句|其他)\]/.test(line));
  return lines.slice(0, 3).join('；').replace(/；{2,}/g, '；');
}

function normalizeTerm(value) {
  return String(value ?? '')
    .toLowerCase()
    .replaceAll('’', "'")
    .replace(/\s+/g, ' ')
    .trim();
}

export function stripDictionaryKey(value) {
  return normalizeTerm(value).replace(/[^a-z0-9]/g, '');
}

function lookupKeys(value) {
  const term = normalizeTerm(value);
  const variants = [term];
  if (term.endsWith("'s")) variants.push(term.slice(0, -2));
  if (term.endsWith("s'")) variants.push(term.slice(0, -1), term.slice(0, -2));
  return [...new Set(variants.map(stripDictionaryKey).filter(Boolean))];
}

function corpusTexts(section) {
  return [
    section.instructions,
    section.passage,
    ...(section.questions ?? []).flatMap((question) => [
      question.stem,
      ...(question.choices ?? []).map((choice) => choice.text),
    ]),
  ].filter(Boolean);
}

export function collectExamCorpus(examDir = DEFAULT_EXAM_DIR) {
  const words = new Set();
  const wordCounts = new Map();
  const sequences = [];
  let textFragments = 0;
  for (const fileName of EXAM_FILES) {
    const document = JSON.parse(fs.readFileSync(path.join(examDir, fileName), 'utf8'));
    for (const paper of document.papers ?? []) {
      for (const section of paper.sections ?? []) {
        for (const text of corpusTexts(section)) {
          const tokens = tokenizeEnglish(text);
          if (!tokens.length) continue;
          textFragments += 1;
          sequences.push(tokens);
          tokens.forEach((token) => {
            words.add(token);
            wordCounts.set(token, (wordCounts.get(token) ?? 0) + 1);
          });
        }
      }
    }
  }
  return {
    words,
    wordCounts,
    wordOccurrences: [...wordCounts.values()].reduce((sum, count) => sum + count, 0),
    sequences,
    textFragments,
  };
}

function parseExchange(exchange) {
  const forms = new Map();
  for (const item of String(exchange ?? '').split('/')) {
    const separator = item.indexOf(':');
    if (separator <= 0) continue;
    const kind = item.slice(0, separator);
    const values = item
      .slice(separator + 1)
      .split(',')
      .map(normalizeTerm)
      .filter(Boolean);
    forms.set(kind, values);
  }
  return forms;
}

function entryFromDictionary({ term, lemma = term, pos = '', meaning, source }) {
  return {
    term,
    lemma,
    partOfSpeech: pos || (term.includes(' ') ? 'phrase' : ''),
    meaning,
    source,
  };
}

function seedMeanings(entry) {
  const content = entry?.content?.word?.content ?? {};
  const direct = (content.trans ?? []).map((item) => item.tranCn || item.tran);
  const synonym = (content.syno?.synos ?? []).map((item) => item.tran);
  return [...direct, ...synonym].map(cleanDictionaryMeaning).filter(Boolean);
}

function loadSeedFallbacks(seedDir, corpusWords, phraseDictionary) {
  const fallbacks = new Map();
  for (const fileName of fs.readdirSync(seedDir).filter((name) => name.endsWith('.json'))) {
    const entries = JSON.parse(fs.readFileSync(path.join(seedDir, fileName), 'utf8'));
    for (const entry of entries) {
      const term = normalizeTerm(entry.headWord);
      const meaning = seedMeanings(entry)[0];
      if (!term || !meaning) continue;
      const tokenCount = term.split(' ').length;
      const candidate = entryFromDictionary({
        term,
        pos: tokenCount > 1 ? 'phrase' : '',
        meaning,
        source: `seed-vocab/${fileName}`,
      });
      if (tokenCount > 1 && tokenCount <= 6) {
        phraseDictionary.set(term, candidate);
      } else if (corpusWords.has(term) && !fallbacks.has(term)) {
        fallbacks.set(term, candidate);
      }
    }
  }
  return fallbacks;
}

async function loadEcdict(ecdictPath, corpusWords) {
  const exactWords = new Map();
  const inflectedWords = new Map();
  const fuzzyWords = new Map();
  const phraseDictionary = new Map();
  const formToLemma = new Map();
  const fuzzyTargets = new Map();
  for (const term of corpusWords) {
    for (const key of lookupKeys(term)) {
      const targets = fuzzyTargets.get(key) ?? [];
      targets.push(term);
      fuzzyTargets.set(key, targets);
    }
  }
  const input = fs.createReadStream(ecdictPath, { encoding: 'utf8' });
  const lines = readline.createInterface({ input, crlfDelay: Infinity });
  let header;
  for await (const line of lines) {
    const values = parseCsvLine(line);
    if (!header) {
      header = values;
      continue;
    }
    const row = Object.fromEntries(header.map((name, index) => [name, values[index] ?? '']));
    const term = normalizeTerm(row.word);
    const meaning = cleanDictionaryMeaning(row.translation);
    if (!term || !meaning) continue;
    const parts = term.split(' ');
    const candidate = entryFromDictionary({
      term,
      pos: row.pos,
      meaning,
      source: 'ECDICT',
    });
    for (const target of fuzzyTargets.get(stripDictionaryKey(term)) ?? []) {
      if (!fuzzyWords.has(target)) {
        fuzzyWords.set(
          target,
          entryFromDictionary({
            term: target,
            lemma: term,
            pos: row.pos,
            meaning,
            source: 'ECDICT',
          }),
        );
      }
    }
    if (parts.length >= 2 && parts.length <= 6) {
      if (parts.every((part) => corpusWords.has(part))) {
        phraseDictionary.set(term, candidate);
      }
      continue;
    }
    if (parts.length !== 1) continue;
    if (corpusWords.has(term) && !exactWords.has(term)) exactWords.set(term, candidate);
    const exchange = parseExchange(row.exchange);
    const lemma = exchange.get('0')?.[0] || term;
    if (corpusWords.has(term)) formToLemma.set(term, lemma);
    for (const forms of exchange.values()) {
      for (const form of forms) {
        if (!corpusWords.has(form)) continue;
        formToLemma.set(form, lemma);
        if (!inflectedWords.has(form)) {
          inflectedWords.set(
            form,
            entryFromDictionary({
              term: form,
              lemma: term,
              pos: row.pos,
              meaning,
              source: 'ECDICT',
            }),
          );
        }
      }
    }
  }
  return { exactWords, inflectedWords, fuzzyWords, phraseDictionary, formToLemma };
}

function matchCorpusPhrases(sequences, phraseDictionary, formToLemma) {
  const matches = new Map();
  for (const tokens of sequences) {
    for (let start = 0; start < tokens.length; start += 1) {
      const surface = [];
      const lemmas = [];
      for (let length = 1; length <= 6 && start + length <= tokens.length; length += 1) {
        const token = tokens[start + length - 1];
        surface.push(token);
        lemmas.push(formToLemma.get(token) || token);
        if (length < 2) continue;
        const surfaceTerm = surface.join(' ');
        const lemmaTerm = lemmas.join(' ');
        const source = phraseDictionary.get(surfaceTerm) ?? phraseDictionary.get(lemmaTerm);
        if (!source || matches.has(surfaceTerm)) continue;
        matches.set(
          surfaceTerm,
          entryFromDictionary({
            term: surfaceTerm,
            lemma: source.term,
            pos: source.partOfSpeech || 'phrase',
            meaning: source.meaning,
            source: source.source,
          }),
        );
      }
    }
  }
  return matches;
}

export async function buildExamCorpusDictionary({
  ecdictPath,
  examDir = DEFAULT_EXAM_DIR,
  seedDir = DEFAULT_SEED_DIR,
  outputPath = DEFAULT_OUTPUT,
  reportPath = DEFAULT_REPORT,
  sourceCommit = 'unknown',
} = {}) {
  if (!ecdictPath) throw new Error('Missing --ecdict path');
  const corpus = collectExamCorpus(examDir);
  const ecdict = await loadEcdict(ecdictPath, corpus.words);
  const seedFallbacks = loadSeedFallbacks(
    seedDir,
    corpus.words,
    ecdict.phraseDictionary,
  );
  const words = new Map();
  for (const term of corpus.words) {
    const entry =
      ecdict.exactWords.get(term) ??
      ecdict.inflectedWords.get(term) ??
      ecdict.fuzzyWords.get(term) ??
      seedFallbacks.get(term);
    if (entry) words.set(term, entry);
  }
  const phrases = matchCorpusPhrases(
    corpus.sequences,
    ecdict.phraseDictionary,
    ecdict.formToLemma,
  );
  const uncoveredWords = [...corpus.words].filter((term) => !words.has(term)).sort();
  const coveredWordOccurrences = [...words.keys()].reduce(
    (sum, term) => sum + (corpus.wordCounts.get(term) ?? 0),
    0,
  );
  const entries = [...words.values(), ...phrases.values()].sort(
    (left, right) => left.term.localeCompare(right.term),
  );
  const payload = {
    schemaVersion: 1,
    source: {
      name: 'ECDICT',
      url: 'https://github.com/skywind3000/ECDICT',
      commit: sourceCommit,
      license: 'MIT',
      seedFallback: 'bundled seed-vocab books',
    },
    coverage: {
      textFragments: corpus.textFragments,
      corpusWordTypes: corpus.words.size,
      coveredWordTypes: words.size,
      uncoveredWordTypes: uncoveredWords.length,
      corpusWordOccurrences: corpus.wordOccurrences,
      coveredWordOccurrences,
      uncoveredWordOccurrences: corpus.wordOccurrences - coveredWordOccurrences,
      matchedPhraseTypes: phrases.size,
    },
    entries,
  };
  const report = {
    ...payload.coverage,
    ecdictWordTypes: [...words.values()].filter((entry) => entry.source === 'ECDICT').length,
    seedFallbackWordTypes: [...words.values()].filter((entry) => entry.source !== 'ECDICT').length,
    phraseTypesBySource: Object.fromEntries(
      [...phrases.values()].reduce((counts, entry) => {
        counts.set(entry.source, (counts.get(entry.source) ?? 0) + 1);
        return counts;
      }, new Map()),
    ),
    uncoveredSimpleWordTypes: uncoveredWords.filter((term) => /^[a-z]+$/.test(term)).length,
    uncoveredCompoundWordTypes: uncoveredWords.filter((term) => term.includes('-')).length,
    uncoveredWords,
  };
  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  fs.mkdirSync(path.dirname(reportPath), { recursive: true });
  fs.writeFileSync(outputPath, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
  fs.writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  return { payload, report };
}

function cliOptions(argv) {
  const valueAfter = (flag) => {
    const index = argv.indexOf(flag);
    return index >= 0 ? argv[index + 1] : undefined;
  };
  return {
    ecdictPath: valueAfter('--ecdict'),
    sourceCommit: valueAfter('--source-commit') ?? 'unknown',
    outputPath: valueAfter('--output') ?? DEFAULT_OUTPUT,
    reportPath: valueAfter('--report') ?? DEFAULT_REPORT,
  };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const { payload, report } = await buildExamCorpusDictionary(cliOptions(process.argv.slice(2)));
  console.log(
    JSON.stringify(
      {
        output: DEFAULT_OUTPUT,
        entries: payload.entries.length,
        ...payload.coverage,
        phraseTypesBySource: report.phraseTypesBySource,
        uncoveredSample: report.uncoveredWords.slice(0, 100),
      },
      null,
      2,
    ),
  );
}
