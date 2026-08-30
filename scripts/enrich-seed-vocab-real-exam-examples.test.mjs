import assert from 'node:assert/strict';
import fs from 'node:fs';
import test from 'node:test';

const bookPath = 'apps/mobile/android/app/src/main/assets/seed-vocab/book/KaoYan_3.json';

test('available in-scope examples outrank older and cross-exam examples', () => {
  const entries = JSON.parse(fs.readFileSync(bookPath, 'utf8'));
  let positive = 0;
  let inScope = 0;

  for (const entry of entries) {
    const content = entry?.content?.word?.content ?? {};
    const frequency = content.realExamFrequency;
    if (!(frequency?.occurrences > 0)) continue;

    const examples = content.realExamSentence?.sentences ?? [];
    const matching = examples.find((example) =>
      example?.source?.exam === frequency.exam &&
      example?.source?.year >= frequency.minYear &&
      example?.source?.year <= frequency.maxYear,
    );
    positive += 1;
    if (!matching) continue;
    inScope += 1;
    assert.equal(examples[0], matching, entry.headWord);
  }

  assert.ok(inScope / positive > 0.98, `${inScope}/${positive}`);
});

test('segment prioritizes a 2010-plus Kaoyan English I source sentence', () => {
  const entries = JSON.parse(fs.readFileSync(bookPath, 'utf8'));
  const segment = entries.find((entry) => entry.headWord === 'segment');
  const frequency = segment.content.word.content.realExamFrequency;
  const first = segment.content.word.content.realExamSentence.sentences[0];

  assert.equal(first.source.exam, frequency.exam);
  assert.ok(first.source.year >= frequency.minYear);
  assert.ok(first.source.year <= frequency.maxYear);
});
