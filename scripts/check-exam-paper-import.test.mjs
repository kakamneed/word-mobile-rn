import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';

import { auditExamPaperAssets } from './check-exam-paper-import.mjs';

function writeJson(filePath, value) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
}

function writeFixture(questions) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'exam-paper-check-'));
  const assetDir = path.join(root, 'exam-papers');
  const paperPath = path.join(assetDir, 'cet4.json');
  writeJson(paperPath, {
    schemaVersion: 1,
    exam: 'cet4',
    papers: [
      {
        schemaVersion: 1,
        id: 'cet4-2025-6-1',
        exam: 'cet4',
        title: 'CET4 2025-06 Set 1',
        year: 2025,
        month: 6,
        set: 1,
        source: { repo: 'fixture', path: 'fixture.json' },
        sections: [
          {
            id: 'cet4-2025-6-1-reading',
            type: 'reading',
            title: 'Reading',
            passage: 'A short passage.',
            questions,
          },
        ],
      },
    ],
  });
  writeJson(path.join(assetDir, 'manifest.json'), {
    schemaVersion: 1,
    files: [paperPath],
    totalPapers: 1,
  });
  return assetDir;
}

test('reports answer presence separately from renderer capabilities', () => {
  const assetDir = writeFixture([
    {
      id: 'objective-1',
      number: 1,
      kind: 'objective',
      capabilities: { autoGradable: true },
      stem: 'Choose one.',
      choices: [
        { label: 'A', text: 'First' },
        { label: 'B', text: 'Second' },
      ],
      answer: 'A',
      explanation: '',
    },
    {
      id: 'translation-1',
      number: 2,
      kind: 'translation',
      stem: 'Translate this sentence.',
      choices: [],
      answer: 'Reference translation.',
      explanation: '',
    },
    {
      id: 'writing-1',
      number: 3,
      kind: 'writing',
      stem: 'Write an essay.',
      choices: [],
      answer: null,
      explanation: '',
    },
  ]);

  const result = auditExamPaperAssets({ assetDir });

  assert.equal(result.totalQuestions, 3);
  assert.equal(result.answerBearingQuestions, 2);
  assert.equal(result.capabilities.answerable, 3);
  assert.equal(result.capabilities.autoGradable, 1);
  assert.deepEqual(result.byKind, {
    objective: { questions: 1, answerBearing: 1, autoGradable: 1 },
    translation: { questions: 1, answerBearing: 1, autoGradable: 0 },
    writing: { questions: 1, answerBearing: 0, autoGradable: 0 },
  });
});

test('rejects objective answers that the declared choices cannot render', () => {
  const assetDir = writeFixture([
    {
      id: 'objective-invalid',
      number: 1,
      kind: 'objective',
      capabilities: { autoGradable: true },
      stem: 'Choose one.',
      choices: [
        { label: 'A', text: 'First' },
        { label: 'B', text: 'Second' },
      ],
      answer: 'C',
      explanation: '',
    },
  ]);

  assert.throws(
    () => auditExamPaperAssets({ assetDir }),
    /objective-invalid: answer C is not present in choices/,
  );
});
