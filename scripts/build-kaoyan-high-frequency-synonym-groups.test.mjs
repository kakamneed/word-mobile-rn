import assert from 'node:assert/strict';
import test from 'node:test';

import { buildKaoyanHighFrequencySynonymGroups } from './build-kaoyan-high-frequency-synonym-groups.mjs';

function vocabularyEntry(headWord, pos, meaning, synos = []) {
  return {
    headWord,
    content: {
      word: {
        content: {
          trans: [{ pos, tranCn: meaning }],
          syno: synos.length ? { synos } : undefined,
        },
      },
    },
  };
}

test('groups high-frequency synonyms by each exact Chinese sense without crossing senses', () => {
  const result = buildKaoyanHighFrequencySynonymGroups({
    vocabulary: [
      vocabularyEntry('attract', 'v', '吸引；引起', [{
        pos: 'vt',
        tran: '吸引；引起',
        hwds: [{ w: 'engage' }, { w: 'cause' }],
      }]),
      vocabularyEntry('engage', 'v', '参与；吸引'),
      vocabularyEntry('cause', 'v', '引起；导致'),
    ],
    ranking: [
      { word: 'attract', rank: 1, occurrences: 10 },
      { word: 'engage', rank: 2, occurrences: 9 },
      { word: 'cause', rank: 3, occurrences: 8 },
    ],
  });

  assert.equal(result.summary.highFrequencyWordCount, 3);
  assert.deepEqual(result.entries[0], {
    word: 'attract',
    rank: 1,
    occurrences: 10,
    groups: [
      {
        meaning: '吸引',
        synonyms: [{ word: 'engage', rank: 2, sources: ['explicit', 'glossOverlap'] }],
      },
      {
        meaning: '引起',
        synonyms: [{ word: 'cause', rank: 3, sources: ['explicit', 'glossOverlap'] }],
      },
    ],
  });
});

test('keeps Chinese senses with no confirmed high-frequency synonym in the complete table', () => {
  const result = buildKaoyanHighFrequencySynonymGroups({
    vocabulary: [vocabularyEntry('solo', 'adj', '独自的')],
    ranking: [{ word: 'solo', rank: 1, occurrences: 1 }],
  });

  assert.deepEqual(result.entries, [{
    word: 'solo',
    rank: 1,
    occurrences: 1,
    groups: [{ meaning: '独自的', synonyms: [] }],
  }]);
  assert.deepEqual(result.summary, {
    highFrequencyWordCount: 1,
    wordCountWithSynonyms: 0,
    wordCountWithoutSynonyms: 1,
    groupCount: 1,
    nonemptyGroupCount: 0,
    synonymCount: 0,
  });
});
