import assert from 'node:assert/strict';
import test from 'node:test';

import {
  applyEffectiveRuntimeQuestionPreps,
  classifyMeaningConflict,
  rankRuntimeDistractors,
} from './list-seed-vocab-choice-conflicts.mjs';

test('detects the four reported ambiguity patterns', () => {
  const cases = [
    ['控制；约束', '抑制；克制；戒除'],
    ['移置；转移；取代；置换', '交换；互换；调换；更换'],
    ['插入；嵌入', '把...嵌入'],
    ['炉；灶；烤箱', '炉子；熔炉'],
  ];

  for (const [correct, distractor] of cases) {
    assert.ok(
      classifyMeaningConflict(correct, distractor),
      `expected conflict: ${correct} <-> ${distractor}`,
    );
  }
});

test('does not flag unrelated same-part-of-speech meanings', () => {
  assert.equal(classifyMeaningConflict('炉；灶；烤箱', '眉毛'), null);
  assert.equal(classifyMeaningConflict('控制；约束', '制造；加工'), null);
  assert.equal(classifyMeaningConflict('插入；嵌入', '拒绝；退回'), null);
  assert.equal(classifyMeaningConflict('工作；雇用；使用', '申请；应用；实施'), null);
  assert.equal(classifyMeaningConflict('实际上的；事实上的；虚的', '值得的；配得上的'), null);
  assert.equal(classifyMeaningConflict('统治；治理；支配', '阻止；劝阻'), null);
  assert.equal(classifyMeaningConflict('发现；调查的结果；裁决', '实验室；研究室'), null);
});

test('runtime ranking puts the most overlapping same-pos meaning first', () => {
  const target = { id: 1, word: 'insert', pos: 'v', meaning: '插入；嵌入', rank: 10 };
  const candidates = [
    { id: 2, word: 'embed', pos: 'v', meaning: '把...嵌入', rank: 20 },
    { id: 3, word: 'reject', pos: 'v', meaning: '拒绝；退回', rank: 11 },
    { id: 4, word: 'help', pos: 'v', meaning: '帮助；协助', rank: 12 },
  ];

  assert.equal(rankRuntimeDistractors(target, candidates, 3)[0].word, 'embed');
});

test('effective runtime keeps complete bundled safe precomputed choices', () => {
  const items = [{
    headWord: 'curb',
    cnChoiceDistractors: ['\u5236\u9020', '\u6350\u732e', '\u5bb3\u6015'],
    enChoiceDistractors: ['manufacture', 'donate', 'frighten'],
    content: { word: { content: { trans: [{ pos: 'vt', tranCn: '\u63a7\u5236' }] } } },
  }];

  const [effective] = applyEffectiveRuntimeQuestionPreps(items);
  assert.deepEqual(effective.cnChoiceDistractors, ['\u5236\u9020', '\u6350\u732e', '\u5bb3\u6015']);
  assert.deepEqual(effective.enChoiceDistractors, ['manufacture', 'donate', 'frighten']);
});
