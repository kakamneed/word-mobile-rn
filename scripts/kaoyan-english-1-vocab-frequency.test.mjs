import assert from 'node:assert/strict';
import test from 'node:test';

import {
  analyzeVocabularyFrequency,
  embedFrequencyData,
  promoteSupplementalVocabulary,
  serializeDerivationalFamilyIndex,
  tokenizeEnglish,
} from './kaoyan-english-1-vocab-frequency.mjs';

test('tokenizes English words without counting punctuation or line numbers', () => {
  assert.deepEqual(tokenizeEnglish("The study's well-known result, 1."), [
    'the',
    "study's",
    'well-known',
    'result',
  ]);
});

test('filters stopwords and ranks vocabulary word families with surface details', () => {
  const result = analyzeVocabularyFrequency({
    papers: [
      {
        year: 2010,
        sections: [{
          instructions: 'Read the passage.',
          passage: 'Study study studies.',
          questions: [{ stem: 'For study?', choices: [{ text: 'Study' }] }],
        }],
      },
      {
        year: 2009,
        sections: [{ passage: 'Study study.' }],
      },
    ],
    vocabulary: [{ headWord: 'study' }, { headWord: 'result' }, { headWord: 'the' }],
    minYear: 2010,
  });

  assert.equal(result.summary.paperCount, 1);
  assert.equal(result.summary.stopwordVocabularyCount, 1);
  assert.deepEqual(result.ranking.map((entry) => [entry.word, entry.occurrences]), [
    ['study', 5],
  ]);
  assert.deepEqual(result.ranking[0].surfaceForms, { studies: 1, study: 4 });
});

test('embeds ranked and zero-count exam frequency data into every vocabulary entry', () => {
  const vocabulary = [
    { headWord: 'study', content: { word: { content: {} } } },
    { headWord: 'absent', content: { word: { content: {} } } },
  ];
  embedFrequencyData(vocabulary, {
    metadata: { exam: 'kaoyan-english-1', minYear: 2010 },
    summary: { years: [2010, 2011] },
    ranking: [{
      rank: 1,
      word: 'study',
      occurrences: 5,
      years: { 2010: 2, 2011: 3 },
      surfaceForms: { studies: 1, study: 4 },
    }],
  });

  assert.deepEqual(
    vocabulary[0].content.word.content.realExamFrequency,
    {
      schemaVersion: 2,
      source: 'exam-papers',
      exam: 'kaoyan-english-1',
      minYear: 2010,
      maxYear: 2011,
      rank: 1,
      occurrences: 5,
      years: { 2010: 2, 2011: 3 },
      surfaceForms: { studies: 1, study: 4 },
    },
  );
  assert.equal(
    vocabulary[1].content.word.content.realExamFrequency.occurrences,
    0,
  );
  assert.equal(
    vocabulary[1].content.word.content.realExamFrequency.rank,
    null,
  );
});

test('embeds one derivational-family total while preserving strict headword counts', () => {
  const relatedWords = {
    desc: '同根',
    rels: [{
      pos: 'mixed',
      words: [
        { hwd: 'observation' },
        { hwd: 'observer' },
        { hwd: 'observable' },
        { hwd: 'observational' },
      ],
    }],
  };
  const vocabulary = [
    { headWord: 'observe', content: { word: { content: { relWord: relatedWords } } } },
    { headWord: 'observation', content: { word: { content: {} } } },
  ];
  embedFrequencyData(vocabulary, {
    metadata: { exam: 'kaoyan-english-1', minYear: 2010 },
    summary: { years: [2010, 2011] },
    ranking: [
      { rank: 10, word: 'observe', occurrences: 6, years: {}, surfaceForms: { observe: 3, observed: 3 } },
      { rank: 20, word: 'observation', occurrences: 3, years: {}, surfaceForms: { observation: 3 } },
      { rank: 30, word: 'observer', occurrences: 1, years: {}, surfaceForms: { observer: 1 } },
      { rank: 31, word: 'observable', occurrences: 1, years: {}, surfaceForms: { observable: 1 } },
      { rank: 32, word: 'observational', occurrences: 1, years: {}, surfaceForms: { observational: 1 } },
    ],
  });

  assert.equal(vocabulary[0].content.word.content.realExamFrequency.occurrences, 6);
  assert.equal(vocabulary[1].content.word.content.realExamFrequency.occurrences, 3);
  assert.deepEqual(
    vocabulary[0].content.word.content.realExamFrequency.derivationalFamily,
    {
      root: 'observe',
      occurrences: 12,
      members: {
        observable: 1,
        observation: 3,
        observational: 1,
        observe: 6,
        observer: 1,
      },
    },
  );
  assert.deepEqual(
    vocabulary[1].content.word.content.realExamFrequency.derivationalFamily,
    vocabulary[0].content.word.content.realExamFrequency.derivationalFamily,
  );
  const portableIndex = serializeDerivationalFamilyIndex(vocabulary, [
    { word: 'observe', occurrences: 6 },
    { word: 'observation', occurrences: 3 },
    { word: 'observer', occurrences: 1 },
    { word: 'observable', occurrences: 1 },
    { word: 'observational', occurrences: 1 },
  ]);
  assert.equal(portableIndex.observer.strictOccurrences, 1);
  assert.equal(portableIndex.observer.occurrences, 12);
});

test('keeps real and realize derivational branches separate', () => {
  const vocabulary = [
    {
      headWord: 'reality',
      content: { word: { content: { relWord: {
        desc: '同根',
        rels: [{ words: [
          { hwd: 'real' },
          { hwd: 'really' },
          { hwd: 'realize' },
          { hwd: 'realization' },
        ] }],
      } } } },
    },
  ];
  const index = serializeDerivationalFamilyIndex(vocabulary, [
    { word: 'real', occurrences: 4 },
    { word: 'reality', occurrences: 3 },
    { word: 'really', occurrences: 2 },
    { word: 'realize', occurrences: 5 },
    { word: 'realization', occurrences: 1 },
  ]);

  assert.deepEqual(index.reality.members, { real: 4, reality: 3, really: 2 });
  assert.deepEqual(index.realize.members, { realization: 1, realize: 5 });
  assert.deepEqual(index.realization.members, { realization: 1, realize: 5 });
});

test('ranks supplemental dictionary lemmas and merges their corpus surface forms', () => {
  const result = analyzeVocabularyFrequency({
    papers: [{
      year: 2010,
      sections: [{
        passage: 'Generally the pattern applies generally.',
        questions: [{ stem: 'Which pattern?', choices: [{ text: 'Generally' }] }],
      }],
    }],
    vocabulary: [{ headWord: 'pattern' }],
    supplementalVocabulary: [{
      term: 'generally',
      lemma: 'general',
      meaning: 'adv. 通常，一般地',
      source: 'ECDICT',
    }],
    minYear: 2010,
  });

  assert.deepEqual(
    result.ranking.map((entry) => [entry.word, entry.occurrences, entry.surfaceForms]),
    [
      ['general', 3, { generally: 3 }],
      ['pattern', 2, { pattern: 2 }],
    ],
  );
});

test('promotes only supplemental words meeting the real-exam frequency threshold', () => {
  const vocabulary = [{
    wordRank: 1,
    headWord: 'pattern',
    bookId: 'KaoYan_3',
    content: { word: { wordHead: 'pattern', wordId: 'KaoYan_3_1', content: {} } },
  }];
  const promoted = promoteSupplementalVocabulary({
    vocabulary,
    supplementalVocabulary: [
      { term: 'generally', lemma: 'generally', meaning: 'adv. 通常，一般地', source: 'ECDICT' },
      { term: 'isolated', lemma: 'isolate', meaning: 'v. 隔离', source: 'ECDICT' },
    ],
    ranking: [
      { rank: 1, word: 'generally', occurrences: 5 },
      { rank: 2, word: 'isolate', occurrences: 4 },
    ],
    minOccurrences: 5,
  });

  assert.equal(promoted.length, 1);
  assert.equal(promoted[0].headWord, 'generally');
  assert.equal(promoted[0].wordRank, 2);
  assert.equal(promoted[0].bookId, 'KaoYan_3');
  assert.equal(promoted[0].content.word.wordId, 'KaoYan_3_2');
  assert.equal(promoted[0].content.word.content.trans[0].tranCn, '通常，一般地');
  assert.deepEqual(promoted[0].realExamFrequencyPromotion, {
    source: 'exam-corpus-dictionary',
    minOccurrences: 5,
  });
  assert.deepEqual(vocabulary.map((entry) => entry.headWord), ['pattern', 'generally']);

  const promotedAgain = promoteSupplementalVocabulary({
    vocabulary,
    supplementalVocabulary: [
      { term: 'generally', lemma: 'generally', meaning: 'adv. 通常，一般地', source: 'ECDICT' },
    ],
    ranking: [{ rank: 1, word: 'generally', occurrences: 5 }],
    minOccurrences: 5,
  });
  assert.equal(promotedAgain.length, 0);
  assert.equal(vocabulary.length, 2);
});

test('excludes placeholder stems, single-letter noise, and stored writing reference essays', () => {
  const result = analyzeVocabularyFrequency({
    papers: [{
      year: 2010,
      sections: [
        {
          title: '阅读Text1',
          passage: 'A useful signal.',
          questions: [
            { stem: 'Question 1', choices: [{ text: 'Signal' }, { text: 'B' }] },
            { stem: 'Which signal applies?', choices: [] },
          ],
        },
        {
          title: '写作 Part A',
          passage: 'Model essay signal.',
          questions: [{ stem: 'Question 1', choices: [] }],
        },
      ],
    }],
    vocabulary: [
      { headWord: 'question' },
      { headWord: 'signal' },
      { headWord: 'model' },
      { headWord: 'essay' },
      { headWord: 'b' },
    ],
    minYear: 2010,
  });

  assert.deepEqual(
    result.ranking.map((entry) => [entry.word, entry.occurrences]),
    [['signal', 3]],
  );
});
