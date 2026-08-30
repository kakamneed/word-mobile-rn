import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';

import {
  extractQuestionStemFromExplanation,
  importExamPapers,
  mergePlaceholderQuestionStems,
} from './import-exam-papers.mjs';

function writeJson(filePath, value) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
}

test('bundled 2010 reading translations remain complete and paragraph aligned', () => {
  const asset = JSON.parse(
    fs.readFileSync(
      path.join('apps', 'flutter_mobile', 'assets', 'exam-papers', 'kaoyan-english-1.json'),
      'utf8',
    ),
  );
  const paper = asset.papers.find((candidate) => candidate.year === 2010);
  const expectedParagraphs = new Map([
    ['kaoyan-english-1-2010-text1-74', 5],
    ['kaoyan-english-1-2010-text2-75', 5],
    ['kaoyan-english-1-2010-text3-76', 5],
    ['kaoyan-english-1-2010-text4-77', 6],
  ]);

  for (const [sectionId, expectedCount] of expectedParagraphs) {
    const section = paper.sections.find((candidate) => candidate.id === sectionId);
    const paragraphs = String(section.passage).split(/\n+/u).filter((value) => value.trim());
    assert.equal(paragraphs.length, expectedCount, sectionId);
    assert.equal(section.paragraphTranslations.length, expectedCount, sectionId);
    assert.ok(section.paragraphTranslations.every((value) => value.trim()), sectionId);
  }
});

test('bundled 2008 English I Text 2 keeps the two recovered official stems', () => {
  const asset = JSON.parse(
    fs.readFileSync(
      path.join('apps', 'flutter_mobile', 'assets', 'exam-papers', 'kaoyan-english-1.json'),
      'utf8',
    ),
  );
  const paper = asset.papers.find((candidate) => candidate.year === 2008);
  const section = paper.sections.find((candidate) => candidate.type === 'text2');

  assert.equal(section.questions[0].stem, 'In the first paragraph, the author discusses');
  assert.equal(
    section.questions[4].stem,
    'Which of the following best summarizes the main idea of the text?',
  );
});

test('recovers a missing reading stem from the explanation overview', () => {
  const explanation = [
    '<p><strong>【题目及选项概览】</strong></p>',
    '<p><strong>It is indicated in Paragraphs 1 and 2 that_____ .<br></strong></p>',
    '<p><strong>文章第1、2段表明_______。</strong></p>',
    '<p>A. arts criticism has disappeared</p>',
  ].join('');

  assert.equal(
    extractQuestionStemFromExplanation(explanation),
    'It is indicated in Paragraphs 1 and 2 that_____ .',
  );
});

test('merges a structured stem into an answer-rich placeholder question', () => {
  const choices = [
    { label: 'A', text: 'First option' },
    { label: 'B', text: 'Second option' },
  ];
  const answerRich = {
    id: 'kaoyan-english-1-1998',
    sections: [{ questions: [{ stem: 'Question 1', choices, answer: 'B' }] }],
  };
  const stemSource = {
    id: 'kaoyan-english-1-1998',
    sections: [{ questions: [{
      stem: 'Which statement is true?',
      choices: [
        { label: 'A', text: 'First option.' },
        { label: 'B', text: '“Second option”' },
      ],
    }] }],
  };

  assert.deepEqual(
    mergePlaceholderQuestionStems([answerRich, stemSource]),
    { updatedQuestions: 1 },
  );
  assert.equal(answerRich.sections[0].questions[0].stem, 'Which statement is true?');
  assert.equal(answerRich.sections[0].questions[0].answer, 'B');
});

test('normalizes kaoyan JSON, CET4 parsed JSON, and CET6 TXT into one schema', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'exam-import-'));
  const kaoyanDir = path.join(root, 'kaoyan');
  const cet4Dir = path.join(root, 'cet4');
  const cet6Dir = path.join(root, 'cet6');
  const outDir = path.join(root, 'out');
  const reportPath = path.join(root, 'report.json');

  writeJson(path.join(kaoyanDir, 'public/data/2025.json'), {
    year: 2025,
    sections: {
      cloze: {
        name: 'Section I Use of English',
        type: 'cloze',
        article: 'Located in the southern peninsula...',
        questions: [
          {
            number: 1,
            choices: [
              { label: 'A', text: 'relevant' },
              { label: 'B', text: 'prone' },
            ],
          },
        ],
      },
    },
  });

  writeJson(path.join(cet4Dir, 'parsed_data.json'), {
    '2025-06-CET4-1.pdf': [
      {
        section: 'Listening Comprehension',
        q_number: 1,
        content: 'Question 1',
        answer: 'A',
        options: [
          { label: 'A', text: 'It can be touching.' },
          { label: 'B', text: 'It is hard to predict.' },
        ],
      },
    ],
  });

  fs.mkdirSync(path.join(cet6Dir, 'cet6_zhenti_cleaned'), { recursive: true });
  fs.writeFileSync(
    path.join(cet6Dir, 'cet6_zhenti_cleaned/2025-06_01.txt'),
    [
      '2025年6月英语六级真题(第1套)',
      'Part II',
      'Listening Comprehension (30 minutes)',
      'Section A',
      'Directions: Choose the best answer.',
      'Questions 1 to 2 are based on the conversation.',
      '1. Met the computer technician.',
      'A. Met the computer technician.',
      'B. Called the company.',
      '2. Consulted someone in charge.',
      'A. Came as soon as possible.',
      'B. Informed the office.',
    ].join('\n'),
    'utf8',
  );

  const report = importExamPapers({ kaoyanDir, cet4Dir, cet6TxtDir: cet6Dir, outDir, reportPath });
  assert.equal(report.totalPapers, 1);
  assert.equal(report.totalQuestions, 1);
  assert.deepEqual(
    report.droppedZeroAnswerPapers.map((paper) => paper.exam).sort(),
    ['cet6', 'kaoyan-english-1'],
  );

  const manifest = JSON.parse(fs.readFileSync(path.join(outDir, 'manifest.json'), 'utf8'));
  assert.equal(manifest.schemaVersion, 1);
  assert.deepEqual(
    manifest.sources.map((source) => [source.source, source.latest]),
    [
      ['XixiGod7/kaoyan-english', 2025],
      ['AndrewYuZhenYu/cet6_exam_questions_txt_collection', 2025],
      ['ShepiTT/CET_practice_questions', 2025],
    ],
  );

  const cet4 = JSON.parse(fs.readFileSync(path.join(outDir, 'cet4.json'), 'utf8'));
  assert.equal(cet4.papers[0].month, 6);
  assert.equal(cet4.papers[0].sections[0].title, 'Listening Comprehension');
  assert.equal(cet4.papers[0].sections[0].questions[0].answer, 'A');

  assert.equal(fs.existsSync(path.join(outDir, 'kaoyan-english-1.json')), false);
  assert.equal(fs.existsSync(path.join(outDir, 'cet6.json')), false);
});

test('imports kaoyansou English API snapshots with answers and explanations', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'exam-import-kaoyansou-'));
  const apiDir = path.join(root, 'kaoyansou');
  const outDir = path.join(root, 'out');
  const reportPath = path.join(root, 'report.json');

  writeJson(path.join(apiDir, 'paper-list.json'), {
    code: 200,
    data: {
        records: [
          { id: 1, year: '2026', englishType: '\u82f1\u4e00', sectionName: '\u5b8c\u578b\u586b\u7a7a' },
          { id: 2, year: '2025', englishType: '\u56db\u7ea7-6\u67081\u5957', sectionName: '\u542c\u529b' },
          { id: 3, year: '2026', englishType: '\u82f1\u4e00', sectionName: '\u7ffb\u8bd1' },
          { id: 4, year: '2025', englishType: '\u56db\u7ea7-6\u67081\u5957', sectionName: 'Part 4 Translation' },
        ],
      },
    });
  writeJson(path.join(apiDir, 'papers/1.json'), {
    code: 200,
    data: {
      id: 1,
      year: '2026',
      englishType: '\u82f1\u4e00',
      sectionName: '\u5b8c\u578b\u586b\u7a7a',
      contentJson: JSON.stringify({
        data: {
          title: '\u5b8c\u578b\u586b\u7a7a',
          conts: [{
            econt: [{ name: 'Still' }, { name: ',' }, { name: 'AI' }, { name: 'changes' }],
            zcont: '\u5c3d\u7ba1\u5982\u6b64，AI \u4ecd\u5728\u53d8\u5316\u3002',
          }],
          tm: [
            {
              daan: {
                xx: [
                  { name: 'A', subject: 'Still' },
                  { name: 'B', subject: 'Therefore' },
                ],
                zhengque: '1',
              },
              jiexi: 'turning point',
            },
          ],
        },
      }),
      tiMuJson: '',
    },
  });
  writeJson(path.join(apiDir, 'papers/2.json'), {
    code: 200,
    data: {
      id: 2,
      year: '2025',
      englishType: '\u56db\u7ea7-6\u67081\u5957',
      sectionName: '\u542c\u529b',
      contentJson: JSON.stringify({
        hearingOriginalText: {
          data: [{ data: [{ econt: 'Everything changed.' }] }],
        },
      }),
      tiMuJson: JSON.stringify({
        data: [
          [
            {
              num: 1,
              title: 'News report question',
              daan: {
                xx: [
                  [{ pos: 'A', name: 'By' }, { pos: 'A', name: 'butter.' }],
                  [{ pos: 'B', name: 'By' }, { pos: 'B', name: 'oil.' }],
                ],
                zhengque: 2,
              },
              jiexi_text: 'listen for detail',
            },
          ],
        ],
      }),
    },
  });
  writeJson(path.join(apiDir, 'papers/3.json'), {
    code: 200,
    data: {
      id: 3,
      year: '2026',
      englishType: '\u82f1\u4e00',
      sectionName: '\u7ffb\u8bd1',
      contentJson: JSON.stringify({
        data: {
          tm: [
            {
              daan: {
                xx: [{ name: 'A', subject: '' }],
                zhengque: '\u8fd9\u662f\u4e00\u4e2a\u53c2\u8003\u8bd1\u6587\u3002',
              },
              jiexi: 'translation rubric',
            },
          ],
        },
      }),
      tiMuJson: '',
    },
  });
  writeJson(path.join(apiDir, 'papers/4.json'), {
    code: 200,
    data: {
      id: 4,
      year: '2025',
      englishType: '\u56db\u7ea7-6\u67081\u5957',
      sectionName: 'Part 4 Translation',
      contentJson: JSON.stringify({
        data: {
          conts: [
            {
              zcont: '\u4e2d\u56fd\u7684\u624b\u673a\u7528\u6237\u6570\u91cf\u589e\u957f\u5f88\u5feb\u3002',
              econt: [{ name: 'The' }, { name: 'number' }, { name: 'of' }, { name: 'mobile' }, { name: 'phone' }, { name: 'users' }, { name: 'in' }, { name: 'China' }, { name: 'has' }, { name: 'grown' }, { name: 'rapidly.' }],
            },
          ],
          tm: [{}],
        },
      }),
      tiMuJson: '',
    },
  });

  const report = importExamPapers({ kaoyansouEnglishDir: apiDir, outDir, reportPath });
  assert.equal(report.totalPapers, 2);
  assert.equal(report.answerBearingQuestions, 4);

  const kaoyan = JSON.parse(fs.readFileSync(path.join(outDir, 'kaoyan-english-1.json'), 'utf8'));
  assert.equal(kaoyan.papers[0].year, 2026);
  const cloze = kaoyan.papers[0].sections.find((section) => section.title === '\u5b8c\u578b\u586b\u7a7a');
  assert.equal(cloze.questions[0].answer, 'A');
  assert.equal(cloze.questions[0].kind, 'objective');
  assert.deepEqual(cloze.paragraphTranslations, ['\u5c3d\u7ba1\u5982\u6b64，AI \u4ecd\u5728\u53d8\u5316\u3002']);
  assert.match(cloze.questions[0].answerSource, /kaoyansou/);
  const translation = kaoyan.papers[0].sections.find((section) => section.title === '\u7ffb\u8bd1');
  assert.equal(translation.questions[0].kind, 'translation');
  assert.equal(translation.questions[0].choices.length, 0);
  assert.equal(translation.questions[0].answer, '\u8fd9\u662f\u4e00\u4e2a\u53c2\u8003\u8bd1\u6587\u3002');

  const cet4 = JSON.parse(fs.readFileSync(path.join(outDir, 'cet4.json'), 'utf8'));
  assert.equal(cet4.papers[0].month, 6);
  assert.equal(cet4.papers[0].set, 1);
  const listening = cet4.papers[0].sections.find((section) => section.title === '\u542c\u529b');
  assert.equal(listening.questions[0].choices[1].label, 'B');
  assert.equal(listening.questions[0].kind, 'objective');
  const cetTranslation = cet4.papers[0].sections.find((section) => section.title === 'Part 4 Translation');
  assert.equal(cetTranslation.questions[0].kind, 'translation');
  assert.equal(cetTranslation.questions[0].stem, '\u4e2d\u56fd\u7684\u624b\u673a\u7528\u6237\u6570\u91cf\u589e\u957f\u5f88\u5feb\u3002');
  assert.equal(cetTranslation.questions[0].answer, 'The number of mobile phone users in China has grown rapidly.');
});
