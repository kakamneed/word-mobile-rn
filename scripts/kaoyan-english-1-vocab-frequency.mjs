import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const DEFAULT_EXAM_PATH = path.join(ROOT, 'apps/flutter_mobile/assets/exam-papers/kaoyan-english-1.json');
const DEFAULT_VOCAB_PATH = path.join(ROOT, 'apps/mobile/android/app/src/main/assets/seed-vocab/book/KaoYan_3.json');
const DEFAULT_SUPPLEMENTAL_VOCAB_PATH = path.join(ROOT, 'apps/flutter_mobile/assets/exam-dictionary/exam-corpus-dictionary.json');
const DEFAULT_FAMILY_OUTPUT = path.join(ROOT, 'apps/flutter_mobile/assets/exam-dictionary/kaoyan-derivational-frequencies.json');
const DEFAULT_OUTPUT = path.join(ROOT, 'docs/kaoyan-english-1-vocab-frequency-2010-plus.json');
const DEFAULT_PROMOTION_MIN_OCCURRENCES = 10;
const CURATED_VOCABULARY_MAX_RANK = 3728;

export const STOPWORDS = new Set(`a an and are as at be been being but by can could did do does doing for from had has have having he her here hers him his how i if in into is it its itself me more most my myself no nor not of on or our ours ourselves she should so some than that the their theirs them themselves then there these they this those to too under up us was we were what when where which who why will with would you your yours yourself yourselves`.split(' '));

export function tokenizeEnglish(text) {
  return [...String(text ?? '').matchAll(/[A-Za-z]+(?:['’][A-Za-z]+)?(?:-[A-Za-z]+)*/g)].map((match) => match[0].toLowerCase());
}

function corpusTexts(section) {
  const sectionLabel = `${section.title ?? ''} ${section.instructions ?? ''}`;
  const isWritingReference = /写作|\bwriting\b/i.test(sectionLabel);
  return [
    section.instructions,
    isWritingReference ? null : section.passage,
    ...(section.questions ?? []).flatMap((question) => [
      /^Question\s*\d+$/i.test(String(question.stem ?? '').trim()) ? null : question.stem,
      ...(question.choices ?? []).map((choice) => choice.text),
    ]),
  ].filter(Boolean);
}

function normalizeVocabulary(vocabulary, supplementalVocabulary = []) {
  const aliases = new Map();
  for (const entry of vocabulary) {
    const word = String(entry.headWord ?? '').trim().toLowerCase();
    if (word) aliases.set(word, word);
  }
  for (const entry of supplementalVocabulary) {
    const term = String(entry.term ?? '').trim().toLowerCase();
    const lemma = String(entry.lemma ?? term).trim().toLowerCase();
    if (!term || !lemma || term.includes(' ') || lemma.includes(' ')) continue;
    if (!aliases.has(lemma)) aliases.set(lemma, lemma);
    if (!aliases.has(term)) aliases.set(term, lemma);
  }
  return aliases;
}

function wordFamilyCandidates(word) {
  const normalized = word.trim().toLowerCase();
  const candidates = [];
  const irregular = {
    children: 'child', people: 'person', men: 'man', women: 'woman', mice: 'mouse',
    feet: 'foot', teeth: 'tooth', geese: 'goose', went: 'go', gone: 'go', saw: 'see',
    seen: 'see', made: 'make', took: 'take', taken: 'take', gave: 'give', given: 'give',
    found: 'find', thought: 'think', bought: 'buy', brought: 'bring', wrote: 'write',
    written: 'write',
  }[normalized];
  if (irregular) candidates.push(irregular);
  if (normalized.endsWith('ies') && normalized.length > 3) candidates.push(`${normalized.slice(0, -3)}y`);
  if (normalized.endsWith('ing') && normalized.length > 4) {
    const stem = normalized.slice(0, -3);
    candidates.push(`${stem}e`, stem);
    if (stem.length > 2 && stem.at(-1) === stem.at(-2)) candidates.push(stem.slice(0, -1));
  }
  if (normalized.endsWith('ed') && normalized.length > 3) {
    const stem = normalized.slice(0, -2);
    candidates.push(`${stem}e`, stem);
    if (stem.length > 2 && stem.at(-1) === stem.at(-2)) candidates.push(stem.slice(0, -1));
  }
  if (normalized.endsWith('es') && normalized.length > 3) candidates.push(normalized.slice(0, -2));
  if (normalized.endsWith('s') && normalized.length > 2 && !normalized.endsWith('ss') && !['news', 'series', 'species', 'means', 'analysis'].includes(normalized)) {
    candidates.push(normalized.slice(0, -1));
  }
  candidates.push(normalized);
  return [...new Set(candidates)];
}

function buildFamilyMap(vocabularyMap) {
  const familyMap = new Map();
  for (const surface of vocabularyMap.keys()) {
    const canonical = wordFamilyCandidates(surface).find((candidate) => vocabularyMap.has(candidate)) ?? surface;
    familyMap.set(surface, canonical);
  }
  return familyMap;
}

export function analyzeVocabularyFrequency({ papers, vocabulary, supplementalVocabulary = [], minYear = 2010 }) {
  const vocabularyMap = normalizeVocabulary(vocabulary, supplementalVocabulary);
  const familyMap = buildFamilyMap(vocabularyMap);
  const canonicalVocabulary = new Set(vocabularyMap.values());
  const stopwordVocabulary = new Set([...canonicalVocabulary].filter((word) => STOPWORDS.has(word)));
  const occurrences = new Map();
  const surfaceForms = new Map();
  const perYear = new Map();
  let includedTextFragments = 0;
  let corpusTokensAfterStopwords = 0;
  let matchedTokens = 0;

  for (const paper of papers) {
    if (Number(paper.year) < minYear) continue;
    const year = Number(paper.year);
    const yearCounts = perYear.get(year) ?? new Map();
    for (const section of paper.sections ?? []) {
      for (const text of corpusTexts(section)) {
        const tokens = tokenizeEnglish(text);
        if (!tokens.length) continue;
        includedTextFragments += 1;
        for (const token of tokens) {
          if (token.length === 1 || STOPWORDS.has(token)) continue;
          corpusTokensAfterStopwords += 1;
          const candidate = familyMap.get(token) ?? wordFamilyCandidates(token).find((word) => vocabularyMap.has(word));
          const canonical = vocabularyMap.get(candidate) ?? candidate;
          if (!canonical || STOPWORDS.has(canonical)) continue;
          matchedTokens += 1;
          occurrences.set(canonical, (occurrences.get(canonical) ?? 0) + 1);
          yearCounts.set(canonical, (yearCounts.get(canonical) ?? 0) + 1);
          const counts = surfaceForms.get(canonical) ?? new Map();
          counts.set(token, (counts.get(token) ?? 0) + 1);
          surfaceForms.set(canonical, counts);
        }
      }
    }
    perYear.set(year, yearCounts);
  }

  const ranking = [...occurrences.entries()]
    .map(([word, count]) => ({
      word,
      occurrences: count,
      years: Object.fromEntries([...perYear.entries()].sort(([left], [right]) => left - right).map(([year, counts]) => [year, counts.get(word) ?? 0])),
      surfaceForms: Object.fromEntries([...surfaceForms.get(word).entries()].sort(([left], [right]) => left.localeCompare(right))),
    }))
    .sort((left, right) => right.occurrences - left.occurrences || left.word.localeCompare(right.word))
    .map((entry, index) => ({ rank: index + 1, ...entry }));

  const includedPapers = papers.filter((paper) => Number(paper.year) >= minYear);
  return {
    metadata: {
      exam: 'kaoyan-english-1',
      vocabularyBook: 'KaoYan_3 + exam-corpus-dictionary supplement',
      minYear,
      yearRule: 'year >= minYear',
      matching: 'case-insensitive surface token merged to a vocabulary headWord by the shared exam word-family rule',
      countedFields: 'section instructions, non-writing-reference passages, meaningful question stems, and choices',
      excludedNoise: 'generic Question N placeholders, single-letter tokens, and stored writing reference essays',
      stopwords: [...STOPWORDS].sort(),
      wordFamilyRule: 'canonicalized only when a candidate headWord exists in KaoYan_3; raw surfaceForms are retained',
    },
    summary: {
      paperCount: includedPapers.length,
      years: includedPapers.map((paper) => Number(paper.year)).sort((left, right) => left - right),
      vocabularyCount: canonicalVocabulary.size,
      stopwordVocabularyCount: stopwordVocabulary.size,
      vocabularyCountAfterStopwords: canonicalVocabulary.size - stopwordVocabulary.size,
      appearedVocabularyCount: ranking.length,
      includedTextFragments,
      corpusTokensAfterStopwords,
      matchedVocabularyOccurrences: matchedTokens,
      vocabularyOccurrenceCoveragePercent: corpusTokensAfterStopwords ? Number(((matchedTokens / corpusTokensAfterStopwords) * 100).toFixed(2)) : 0,
    },
    ranking,
  };
}

function cleanSupplementalMeaning(value) {
  return String(value ?? '')
    .replace(/^[a-z]+\.\s*/i, '')
    .trim();
}

export function promoteSupplementalVocabulary({
  vocabulary,
  supplementalVocabulary,
  ranking,
  minOccurrences = DEFAULT_PROMOTION_MIN_OCCURRENCES,
}) {
  const existing = new Set(vocabulary.map((entry) => String(entry.headWord ?? '').trim().toLowerCase()));
  const supplementalByLemma = new Map();
  for (const entry of supplementalVocabulary) {
    const term = String(entry.term ?? '').trim().toLowerCase();
    const lemma = String(entry.lemma ?? term).trim().toLowerCase();
    if (!term || !lemma || term.includes(' ') || lemma.includes(' ')) continue;
    const current = supplementalByLemma.get(lemma);
    if (!current || term === lemma) supplementalByLemma.set(lemma, entry);
  }

  const frequencyByWord = new Map(ranking.map((entry) => [String(entry.word ?? '').toLowerCase(), entry]));
  for (const entry of vocabulary) {
    const word = String(entry.headWord ?? '').trim().toLowerCase();
    const frequency = frequencyByWord.get(word);
    if (
      Number(entry.wordRank) > CURATED_VOCABULARY_MAX_RANK &&
      supplementalByLemma.has(word) &&
      Number(frequency?.occurrences) >= minOccurrences
    ) {
      entry.realExamFrequencyPromotion ??= {
        source: 'exam-corpus-dictionary',
        minOccurrences,
      };
    }
  }

  let nextRank = Math.max(0, ...vocabulary.map((entry) => Number(entry.wordRank) || 0));
  const promoted = [];
  for (const frequency of ranking) {
    const word = String(frequency.word ?? '').trim().toLowerCase();
    if (!word || existing.has(word) || Number(frequency.occurrences) < minOccurrences) continue;
    const source = supplementalByLemma.get(word);
    if (!source) continue;
    const meaning = cleanSupplementalMeaning(source.meaning);
    if (!meaning) continue;
    nextRank += 1;
    const entry = {
      wordRank: nextRank,
      headWord: word,
      content: {
        word: {
          wordHead: word,
          wordId: `KaoYan_3_${nextRank}`,
          content: {
            trans: [{
              tranCn: meaning,
              pos: String(source.partOfSpeech ?? '').trim(),
              descCn: '中释',
            }],
            displayWord: word,
          },
        },
      },
      bookId: 'KaoYan_3',
      realExamFrequencyPromotion: {
        source: 'exam-corpus-dictionary',
        minOccurrences,
      },
    };
    vocabulary.push(entry);
    promoted.push(entry);
    existing.add(word);
  }
  return promoted;
}

export function buildDerivationalFamilyIndex(vocabulary, ranking) {
  const parent = new Map();
  const ensure = (word) => {
    const normalized = String(word ?? '').trim().toLowerCase();
    if (normalized && !normalized.includes(' ')) parent.set(normalized, parent.get(normalized) ?? normalized);
    return normalized;
  };
  const find = (word) => {
    const current = parent.get(word);
    if (!current || current === word) return word;
    const root = find(current);
    parent.set(word, root);
    return root;
  };
  const union = (left, right) => {
    const leftRoot = find(ensure(left));
    const rightRoot = find(ensure(right));
    if (leftRoot && rightRoot && leftRoot !== rightRoot) parent.set(rightRoot, leftRoot);
  };
  const derivationalBranch = (word) => {
    const normalized = String(word ?? '').trim().toLowerCase();
    if (/^reali[sz]/.test(normalized)) return 'realize';
    if (/^real(?:ity|ly|ism|ist|ness)?$/.test(normalized)) return 'real';
    return '';
  };

  const formalWords = new Set();
  for (const entry of vocabulary) {
    const word = ensure(entry.headWord);
    if (!word) continue;
    formalWords.add(word);
    const relWord = entry.content?.word?.content?.relWord;
    if (relWord?.desc !== '同根') continue;
    for (const relation of relWord.rels ?? []) {
      const groups = new Map();
      for (const member of [word, ...(relation.words ?? []).map((related) => related.hwd)]) {
        const branch = derivationalBranch(member);
        const group = groups.get(branch) ?? [];
        group.push(member);
        groups.set(branch, group);
      }
      for (const group of groups.values()) {
        for (const related of group.slice(1)) union(group[0], related);
      }
    }
  }

  const ranked = new Map(ranking.map((entry) => [String(entry.word ?? '').trim().toLowerCase(), entry]));
  const components = new Map();
  for (const word of ranked.keys()) {
    if (!parent.has(word)) continue;
    const component = components.get(find(word)) ?? [];
    component.push(word);
    components.set(find(word), component);
  }

  const byMember = new Map();
  for (const words of components.values()) {
    const members = [...new Set(words)].sort((left, right) => left.localeCompare(right));
    if (members.length < 2) continue;
    const formalMembers = members.filter((word) => formalWords.has(word));
    const root = [...(formalMembers.length ? formalMembers : members)]
      .sort((left, right) => left.length - right.length || left.localeCompare(right))[0];
    const memberCounts = Object.fromEntries(members.map((word) => [word, Number(ranked.get(word)?.occurrences) || 0]));
    const family = {
      root,
      occurrences: Object.values(memberCounts).reduce((total, count) => total + count, 0),
      members: memberCounts,
    };
    for (const member of members) byMember.set(member, family);
  }
  return byMember;
}

export function serializeDerivationalFamilyIndex(vocabulary, ranking) {
  return Object.fromEntries(
    [...buildDerivationalFamilyIndex(vocabulary, ranking).entries()]
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([member, family]) => [member, {
        root: family.root,
        occurrences: family.occurrences,
        strictOccurrences: family.members[member] ?? 0,
        members: family.members,
      }]),
  );
}

export function embedFrequencyData(vocabulary, result) {
  const ranked = new Map(result.ranking.map((entry) => [entry.word, entry]));
  const derivationalFamilies = buildDerivationalFamilyIndex(vocabulary, result.ranking);
  const years = result.summary.years ?? [];
  const maxYear = years.length ? Math.max(...years) : result.metadata.minYear;
  for (const entry of vocabulary) {
    const word = String(entry.headWord ?? '').trim().toLowerCase();
    const frequency = ranked.get(word);
    entry.content ??= {};
    entry.content.word ??= {};
    entry.content.word.content ??= {};
    entry.content.word.content.realExamFrequency = {
      schemaVersion: 2,
      source: 'exam-papers',
      exam: result.metadata.exam,
      minYear: result.metadata.minYear,
      maxYear,
      rank: frequency?.rank ?? null,
      occurrences: frequency?.occurrences ?? 0,
      years: frequency?.years ?? {},
      surfaceForms: frequency?.surfaceForms ?? {},
    };
    const derivationalFamily = derivationalFamilies.get(word);
    if (derivationalFamily) {
      entry.content.word.content.realExamFrequency.derivationalFamily = derivationalFamily;
    }
  }
  return vocabulary;
}

export function runAnalysis({
  examPath = DEFAULT_EXAM_PATH,
  vocabularyPath = DEFAULT_VOCAB_PATH,
  supplementalVocabularyPath = DEFAULT_SUPPLEMENTAL_VOCAB_PATH,
  familyOutputPath = DEFAULT_FAMILY_OUTPUT,
  outputPath = DEFAULT_OUTPUT,
  minYear = 2010,
  promotionMinOccurrences = DEFAULT_PROMOTION_MIN_OCCURRENCES,
} = {}) {
  const papers = JSON.parse(fs.readFileSync(examPath, 'utf8')).papers ?? [];
  const vocabulary = JSON.parse(fs.readFileSync(vocabularyPath, 'utf8'));
  const supplementalVocabulary = JSON.parse(fs.readFileSync(supplementalVocabularyPath, 'utf8')).entries ?? [];
  const result = analyzeVocabularyFrequency({ papers, vocabulary, supplementalVocabulary, minYear });
  const promoted = promoteSupplementalVocabulary({
    vocabulary,
    supplementalVocabulary,
    ranking: result.ranking,
    minOccurrences: promotionMinOccurrences,
  });
  result.summary.promotedVocabularyCount = vocabulary.filter(
    (entry) => entry.realExamFrequencyPromotion?.source === 'exam-corpus-dictionary',
  ).length;
  result.summary.newlyPromotedVocabularyCount = promoted.length;
  result.summary.promotedVocabularyMinOccurrences = promotionMinOccurrences;
  embedFrequencyData(vocabulary, result);
  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  fs.mkdirSync(path.dirname(familyOutputPath), { recursive: true });
  fs.writeFileSync(outputPath, `${JSON.stringify(result, null, 2)}\n`, 'utf8');
  fs.writeFileSync(
    familyOutputPath,
    `${JSON.stringify(serializeDerivationalFamilyIndex(vocabulary, result.ranking))}\n`,
    'utf8',
  );
  fs.writeFileSync(vocabularyPath, `${JSON.stringify(vocabulary)}\n`, 'utf8');
  return result;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const result = runAnalysis();
  console.log(JSON.stringify({ output: DEFAULT_OUTPUT, ...result.summary, top20: result.ranking.slice(0, 20) }, null, 2));
}
