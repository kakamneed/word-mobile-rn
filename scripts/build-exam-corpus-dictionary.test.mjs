import assert from 'node:assert/strict';
import test from 'node:test';

import {
  cleanDictionaryMeaning,
  parseCsvLine,
  stripDictionaryKey,
  tokenizeEnglish,
} from './build-exam-corpus-dictionary.mjs';

test('parses quoted ECDICT translations containing commas', () => {
  const values = parseCsvLine('turn out,,,"结果是, 证明是",phrase,,,,0,0,,,');
  assert.equal(values[0], 'turn out');
  assert.equal(values[3], '结果是, 证明是');
});

test('matches dictionary space and possessive variants through a stable strip key', () => {
  assert.equal(stripDictionaryKey('social-media'), 'socialmedia');
  assert.equal(stripDictionaryKey('social media'), 'socialmedia');
  assert.equal(stripDictionaryKey("scientist's"), 'scientists');
});

test('tokenizes exam words with apostrophes and hyphens consistently', () => {
  assert.deepEqual(tokenizeEnglish("Well-known words aren't rare."), [
    'well-known',
    'words',
    "aren't",
    'rare',
  ]);
});

test('keeps concise Chinese meanings and removes network-only noise', () => {
  assert.equal(
    cleanDictionaryMeaning('prep. 在……之上\\n[网络] 临时结果\\nadv. 在上面'),
    'prep. 在……之上；adv. 在上面',
  );
});
