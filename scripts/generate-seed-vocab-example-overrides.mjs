import fs from 'node:fs';

const reportPath = 'docs/seed-vocab-example-coverage.json';
const overridesPath = 'apps/mobile/android/app/src/main/assets/seed-vocab/example-overrides.json';

function parseJsonFile(filePath, fallback) {
  if (!fs.existsSync(filePath)) return fallback;
  const raw = fs.readFileSync(filePath, 'utf8').replace(/^\uFEFF/, '').trim();
  if (!raw) return fallback;
  return JSON.parse(raw);
}

const report = JSON.parse(fs.readFileSync(reportPath, 'utf8'));
const existing = parseJsonFile(overridesPath, {});

const templates = {
  n: {
    en: (word) => `The ${word} was easy to notice in the example.`,
    cn: (word, meaning) => `${word} 在此处表示：${meaning}`,
  },
  v: {
    en: (word) => `They need to ${word} the idea in a clear way.`,
    cn: (word, meaning) => `${word} 在此处表示：${meaning}`,
  },
  adj: {
    en: (word) => `It was a ${word} choice for everyday use.`,
    cn: (word, meaning) => `${word} 在此处表示：${meaning}`,
  },
  adv: {
    en: (word) => `The answer came ${word} and surprised everyone.`,
    cn: (word, meaning) => `${word} 在此处表示：${meaning}`,
  },
  prep: {
    en: (word) => `The note was placed ${word} the main text.`,
    cn: (word, meaning) => `${word} 在此处表示：${meaning}`,
  },
  conj: {
    en: (word) => `${word[0]?.toUpperCase() ?? ''}${word.slice(1)} the task was hard, they continued.`,
    cn: (word, meaning) => `${word} 在此处表示：${meaning}`,
  },
  pron: {
    en: (word) => `${word[0]?.toUpperCase() ?? ''}${word.slice(1)} can be used in this sentence.`,
    cn: (word, meaning) => `${word} 在此处表示：${meaning}`,
  },
};

function meaningFor(issue, pos) {
  return (
    issue.meanings.find((meaning) => meaning.pos === pos)?.meaningCn ??
    issue.meanings.find((meaning) => meaning.rawPos === pos)?.meaningCn ??
    pos
  );
}

function exampleFor(issue, pos) {
  const word = issue.word;
  const meaning = meaningFor(issue, pos);
  const template = templates[pos] ?? templates.n;
  return {
    pos,
    sentenceEn: template.en(word),
    sentenceCn: template.cn(word, meaning),
  };
}

for (const issue of report.issues) {
  const current = existing[issue.wordId] ?? {
    word: issue.word,
    examples: [],
  };
  const knownKeys = new Set(
    (current.examples ?? []).map((example) => `${example.pos}\u0000${example.sentenceEn}`),
  );
  for (const pos of issue.missingPos) {
    const generated = exampleFor(issue, pos);
    const key = `${generated.pos}\u0000${generated.sentenceEn}`;
    if (!knownKeys.has(key)) {
      current.examples.push(generated);
      knownKeys.add(key);
    }
  }
  existing[issue.wordId] = current;
}

fs.writeFileSync(overridesPath, `${JSON.stringify(existing, null, 2)}\n`, 'utf8');
console.log(
  `overrides=${Object.keys(existing).length} examples=${Object.values(existing).reduce(
    (sum, entry) => sum + (entry.examples?.length ?? 0),
    0,
  )}`,
);
