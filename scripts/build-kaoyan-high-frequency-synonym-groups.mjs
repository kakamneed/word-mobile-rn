import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const DEFAULT_VOCABULARY_PATH = path.join(ROOT, 'apps/mobile/android/app/src/main/assets/seed-vocab/book/KaoYan_3.json');
const DEFAULT_FREQUENCY_PATH = path.join(ROOT, 'docs/kaoyan-english-1-vocab-frequency-2010-plus.json');
const DEFAULT_OUTPUT_PATH = path.join(ROOT, 'docs/kaoyan-high-frequency-synonym-groups.json');
const MAX_SYNONYMS_PER_GROUP = 8;

export function chineseSenseKeys(meaning) {
  return [...new Set(String(meaning ?? '')
    .split(/[；;，,、]/u)
    .map((part) => part.trim())
    .filter((part) => {
      const length = [...part].length;
      return length >= 2 && length <= 24 && /[\u4e00-\u9fff]/u.test(part);
    }))];
}

export function partOfSpeechCategories(pos) {
  const categories = new Set();
  for (const token of String(pos ?? '').toLowerCase().split(/[^a-z]+/u)) {
    if (['adj', 'adjective'].includes(token)) categories.add('adj');
    if (['adv', 'adverb'].includes(token)) categories.add('adv');
    if (['pron', 'pronoun'].includes(token)) categories.add('pron');
    if (['prep', 'preposition'].includes(token)) categories.add('prep');
    if (['conj', 'conjunction'].includes(token)) categories.add('conj');
    if (['interj', 'interjection'].includes(token)) categories.add('interj');
    if (['v', 'vi', 'vt', 'verb'].includes(token)) categories.add('v');
    if (['n', 'noun'].includes(token)) categories.add('n');
  }
  return categories;
}

function partsOfSpeechMatch(left, right) {
  return left.size === 0 || right.size === 0 || [...left].some((category) => right.has(category));
}

function entryContent(entry) {
  return entry?.content?.word?.content ?? {};
}

function entryMeanings(entry) {
  return (entryContent(entry).trans ?? [])
    .map((translation) => ({
      pos: partOfSpeechCategories(translation?.pos),
      senses: chineseSenseKeys(translation?.tranCn),
    }))
    .filter((translation) => translation.senses.length > 0);
}

function explicitSenses(entry) {
  return (entryContent(entry).syno?.synos ?? [])
    .flatMap((group) => chineseSenseKeys(group?.tran).map((meaning) => ({
      meaning,
      words: (group?.hwds ?? [])
        .map((candidate) => String(candidate?.w ?? '').trim().toLowerCase())
        .filter(Boolean),
    })));
}

function mergeCandidate(group, candidate, source) {
  const existing = group.get(candidate.word);
  if (existing) {
    existing.sources.add(source);
  } else {
    group.set(candidate.word, { ...candidate, sources: new Set([source]) });
  }
}

export function buildKaoyanHighFrequencySynonymGroups({ vocabulary, ranking }) {
  const rankByWord = new Map((ranking ?? []).map((entry) => [
    String(entry?.word ?? '').trim().toLowerCase(),
    { rank: Number(entry?.rank), occurrences: Number(entry?.occurrences) || 0 },
  ]));
  const records = new Map();
  for (const entry of vocabulary ?? []) {
    const word = String(entry?.headWord ?? '').trim().toLowerCase();
    const frequency = rankByWord.get(word);
    if (!word || !frequency || !Number.isFinite(frequency.rank)) continue;
    records.set(word, {
      word,
      ...frequency,
      meanings: entryMeanings(entry),
      explicit: explicitSenses(entry),
    });
  }
  const candidates = [...records.values()]
    .sort((left, right) => left.rank - right.rank || left.word.localeCompare(right.word));
  const entries = candidates.map((source) => {
    const groups = new Map();
    for (const meaning of source.meanings) {
      for (const sense of meaning.senses) {
        const group = groups.get(sense) ?? new Map();
        groups.set(sense, group);
        for (const candidate of candidates) {
          if (candidate.word === source.word) continue;
          if (candidate.meanings.some((candidateMeaning) => (
            candidateMeaning.senses.includes(sense)
              && partsOfSpeechMatch(meaning.pos, candidateMeaning.pos)
          ))) {
            mergeCandidate(group, candidate, 'glossOverlap');
          }
        }
      }
    }
    for (const explicit of source.explicit) {
      const group = groups.get(explicit.meaning) ?? new Map();
      groups.set(explicit.meaning, group);
      for (const word of explicit.words) {
        const candidate = records.get(word);
        if (!candidate || candidate.word === source.word) continue;
        if (candidate.meanings.some((meaning) => meaning.senses.includes(explicit.meaning))) {
          mergeCandidate(group, candidate, 'explicit');
        }
      }
    }
    return {
      word: source.word,
      rank: source.rank,
      occurrences: source.occurrences,
      groups: [...groups.entries()]
        .map(([meaning, candidatesForSense]) => ({
          meaning,
          synonyms: [...candidatesForSense.values()]
            .sort((left, right) => left.rank - right.rank || left.word.localeCompare(right.word))
            .slice(0, MAX_SYNONYMS_PER_GROUP)
            .map((candidate) => ({
              word: candidate.word,
              rank: candidate.rank,
              sources: [...candidate.sources].sort(),
            })),
        })),
    };
  });
  const groupCount = entries.reduce((total, entry) => total + entry.groups.length, 0);
  return {
    schemaVersion: 1,
    metadata: {
      wordbook: 'KaoYan_3',
      frequencySource: 'kaoyan-english-1-vocab-frequency-2010-plus.json',
      selection: 'KaoYan_3 headwords with a 2010-plus Kaoyan English I frequency rank',
      grouping: 'Chinese definitions split by semicolon/comma delimiters; candidates require the same exact Chinese sense. Gloss-overlap candidates also require compatible part of speech.',
      candidateSources: {
        explicit: 'bundled source word syno relation, retained only when the candidate also has the exact Chinese sense',
        glossOverlap: 'ranked Kaoyan high-frequency candidate with the exact Chinese sense and compatible part of speech',
      },
      maxSynonymsPerGroup: MAX_SYNONYMS_PER_GROUP,
    },
    summary: {
      highFrequencyWordCount: entries.length,
      wordCountWithSynonyms: entries.filter((entry) => entry.groups.some((group) => group.synonyms.length > 0)).length,
      wordCountWithoutSynonyms: entries.filter((entry) => entry.groups.every((group) => group.synonyms.length === 0)).length,
      groupCount,
      nonemptyGroupCount: entries.reduce(
        (total, entry) => total + entry.groups.filter((group) => group.synonyms.length > 0).length,
        0,
      ),
      synonymCount: entries.reduce((total, entry) => (
        total + entry.groups.reduce((subtotal, group) => subtotal + group.synonyms.length, 0)
      ), 0),
    },
    entries,
  };
}

export function runKaoyanHighFrequencySynonymGroupBuild({
  vocabularyPath = DEFAULT_VOCABULARY_PATH,
  frequencyPath = DEFAULT_FREQUENCY_PATH,
  outputPath = DEFAULT_OUTPUT_PATH,
} = {}) {
  const vocabulary = JSON.parse(fs.readFileSync(vocabularyPath, 'utf8'));
  const ranking = JSON.parse(fs.readFileSync(frequencyPath, 'utf8')).ranking ?? [];
  const result = buildKaoyanHighFrequencySynonymGroups({ vocabulary, ranking });
  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  fs.writeFileSync(outputPath, `${JSON.stringify(result, null, 2)}\n`, 'utf8');
  return result;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const result = runKaoyanHighFrequencySynonymGroupBuild();
  console.log(JSON.stringify({ output: DEFAULT_OUTPUT_PATH, ...result.summary }, null, 2));
}
