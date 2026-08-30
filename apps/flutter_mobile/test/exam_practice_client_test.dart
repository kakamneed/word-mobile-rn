import 'dart:convert';

import 'package:flutter_mobile/bridge/bridge.dart';
import 'package:flutter_mobile/sdk/exam_practice_client.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('exam analysis inbox decodes background status and unread results', () {
    final inbox = ExamAnalysisInbox.fromJson({
      'unreadCount': 1,
      'items': [
        {
          'taskId': 'task-1',
          'exam': 'kaoyan-english-1',
          'paperId': 'paper-2010',
          'paperTitle': 'Kaoyan English I 2010',
          'sectionId': 'reading-1',
          'sectionTitle': 'Reading Text 1',
          'status': 'completed',
          'unread': true,
          'createdAt': '2026-07-27T10:00:00Z',
          'completedAt': '2026-07-27T10:01:00Z',
          'findings': [
            {
              'questionNumber': 1,
              'word': 'consequential',
              'reasoning': 'Changed the choice.',
              'confidence': 0.9,
            },
          ],
          'review': {
            'reviewFormat': 'cloze-review-v1',
            'vocabularyPriority': [
              {'word': 'observe', 'examFrequency': 42},
            ],
          },
        },
      ],
    });

    expect(inbox.unreadCount, 1);
    expect(inbox.items.single.isCompleted, isTrue);
    expect(inbox.items.single.findings.single.word, 'consequential');
    expect(inbox.items.single.review['reviewFormat'], 'cloze-review-v1');
  });

  test('exam analysis inbox normalizes malformed structured review arrays', () {
    final inbox = ExamAnalysisInbox.fromJson({
      'items': [
        {
          'taskId': 'legacy-task',
          'status': 'completed',
          'review': {
            'reviewFormat': 'reading-review-v1',
            'questions': 'not-an-array',
            'correctMarkedQuestions': 'not-an-array',
            'vocabularyPriority': 'not-an-array',
          },
        },
      ],
    });

    final review = inbox.items.single.review;
    expect(review['questions'], isEmpty);
    expect(review['correctMarkedQuestions'], isEmpty);
    expect(review['vocabularyPriority'], isEmpty);
  });

  test('annotation state restores persisted AI causal words', () {
    final state = ExamAnnotationState.fromJson({
      'currentMarks': [
        {'normalized': 'consequential', 'meaning': '重要的', 'mark': 'familiar'},
      ],
      'priorMarks': [],
      'causalWords': ['consequential'],
      'annotations': [],
    });

    expect(state.causalWords, {'consequential'});
    expect(state.currentMarks['consequential'], 'familiar');
  });

  test(
    'definition lookup omits userMark so it cannot clear persisted state',
    () async {
      final bridge = _InspectRecordingBridge();
      final client = ExamPracticeClient(bridge, const BridgeCodec());

      await client.inspectWord(
        articleId: 'paper:section',
        title: 'Reading',
        body: 'Habits matter.',
        token: const ExamWordToken(
          text: 'Habits',
          normalized: 'habits',
          startOffset: 0,
          endOffset: 6,
        ),
        sentenceText: 'Habits matter.',
        metadata: const {'scope': 'passage'},
      );

      final request = jsonDecode(bridge.argument!) as Map<String, dynamic>;
      expect(request.containsKey('userMark'), isFalse);
    },
  );

  test(
    'section analysis sends all wrong attempts through one bridge call',
    () async {
      final bridge = _InspectRecordingBridge();
      final client = ExamPracticeClient(bridge, const BridgeCodec());

      await client.analyzeSectionVocabulary(
        exam: 'kaoyan-english-1',
        paperId: 'paper-2010',
        sectionId: 'reading-1',
        attempts: const [
          ExamAnalysisAttempt(questionId: 'q1', attemptId: 'attempt-1'),
          ExamAnalysisAttempt(questionId: 'q3', attemptId: 'attempt-3'),
        ],
        purePurpleWords: const {'rare', 'observe'},
      );

      expect(bridge.method, 'analyzeExamSectionVocabulary');
      expect(jsonDecode(bridge.argument!)['attempts'], [
        {'questionId': 'q1', 'attemptId': 'attempt-1'},
        {'questionId': 'q3', 'attemptId': 'attempt-3'},
      ]);
      expect(jsonDecode(bridge.argument!)['purePurpleWords'], [
        'observe',
        'rare',
      ]);
    },
  );

  test('catalog decodes paper and section capability summaries', () {
    final catalog = ExamCatalog.fromJson({
      'schemaVersion': 1,
      'exams': [
        {
          'exam': 'cet4',
          'papers': [
            {
              'id': 'cet4-2025-6-1',
              'title': 'CET4 2025-06 Set 1',
              'year': 2025,
              'month': 6,
              'set': 1,
              'questionCount': 2,
              'answerBearingCount': 2,
              'autoGradableCount': 1,
              'origin': 'user',
              'sections': [
                {
                  'id': 'reading',
                  'sectionType': 'reading',
                  'title': 'Reading',
                  'questionCount': 2,
                  'autoGradableCount': 1,
                },
              ],
            },
          ],
        },
      ],
    });

    expect(catalog.exams.single.exam, 'cet4');
    expect(catalog.exams.single.papers.single.autoGradableCount, 1);
    expect(catalog.exams.single.papers.single.origin, 'user');
    expect(catalog.exams.single.papers.single.sections.single.title, 'Reading');
  });

  test('import draft exposes provenance without retaining raw media', () {
    final draft = ExamPaperImportDraft.fromJson({
      'paper': {'id': 'user-1', 'exam': 'custom', 'sections': []},
      'warnings': ['answer missing'],
      'provenance': {'sourceName': 'scan.jpg'},
      'rawMediaRetained': false,
    });

    expect(draft.paper['id'], 'user-1');
    expect(draft.warnings, ['answer missing']);
    expect(draft.provenance['sourceName'], 'scan.jpg');
    expect(draft.rawMediaRetained, isFalse);
  });

  test('priority item exposes every deterministic contributing factor', () {
    final item = ExamVocabularyPriority.fromJson({
      'word': 'resilient',
      'priorityScore': 14.5,
      'paperCount': 3,
      'articleCount': 4,
      'occurrenceCount': 7,
      'unknownMarkCount': 2,
      'wrongAssociationCount': 1,
      'masteredMarkCount': 0,
      'factors': [
        {'type': 'crossPaperFrequency', 'value': 3},
        {'type': 'unknownMarks', 'value': 2},
      ],
    });

    expect(item.paperCount, 3);
    expect(item.wrongAssociationCount, 1);
    expect(item.factors.map((factor) => factor['type']), [
      'crossPaperFrequency',
      'unknownMarks',
    ]);
  });

  test('exam report decodes paper and objective-type accuracy series', () {
    final report = ExamPracticeReport.fromJson({
      'exams': [
        {
          'exam': 'kaoyan-english-1',
          'papers': [
            {
              'paperId': 'paper-2019',
              'title': 'Kaoyan English I 2019',
              'year': 2019,
              'correctCount': 7,
              'totalQuestions': 10,
              'accuracyPercent': 70,
              'questionTypes': {
                'cloze': {
                  'correctCount': 3,
                  'totalQuestions': 5,
                  'accuracyPercent': 60,
                },
                'reading': {
                  'correctCount': 4,
                  'totalQuestions': 5,
                  'accuracyPercent': 80,
                },
              },
            },
          ],
        },
      ],
    });

    final paper = report.exams.single.papers.single;
    expect(paper.accuracyPercent, 70);
    expect(paper.questionTypes['cloze']!.correctCount, 3);
    expect(paper.questionTypes['reading']!.accuracyPercent, 80);
  });

  test('paper preserves answer presence separately from auto grading', () {
    final paper = ExamPaper.fromJson({
      'schemaVersion': 1,
      'id': 'cet4-2025-6-1',
      'exam': 'cet4',
      'title': 'CET4 2025-06 Set 1',
      'year': 2025,
      'month': 6,
      'set': 1,
      'source': {'repo': 'fixture'},
      'sections': [
        {
          'id': 'translation',
          'type': 'translation',
          'title': 'Translation',
          'passage': '',
          'questions': [
            {
              'id': 'q1',
              'number': 1,
              'kind': 'translation',
              'stem': 'Translate this.',
              'choices': [],
              'answer': 'Reference translation.',
              'explanation': '',
              'capabilities': {
                'browsable': true,
                'answerable': true,
                'autoGradable': false,
                'causalAnalyzable': false,
              },
            },
          ],
        },
      ],
    });

    final question = paper.sections.single.questions.single;
    expect(question.hasAnswer, isTrue);
    expect(question.capabilities.autoGradable, isFalse);
  });

  test('paper section keeps bundled paragraph translations offline', () {
    final paper = ExamPaper.fromJson({
      'schemaVersion': 1,
      'id': 'kaoyan-2010',
      'exam': 'kaoyan-english-1',
      'title': 'Kaoyan English I 2010',
      'year': 2010,
      'source': {'repo': 'fixture'},
      'sections': [
        {
          'id': 'cloze',
          'type': 'cloze',
          'title': '完型填空',
          'passage': 'First paragraph.\nSecond paragraph.',
          'paragraphTranslations': ['第一段。', '第二段。'],
          'questions': [],
        },
      ],
    });

    expect(paper.sections.single.paragraphTranslations, ['第一段。', '第二段。']);
  });

  test('attempt decodes nullable grading state and answer history', () {
    final attempt = ExamAttempt.fromJson({
      'attemptId': 'attempt-1',
      'paperId': 'cet4-2025-6-1',
      'sectionId': 'reading',
      'questionId': 'q1',
      'selectedAnswer': 'B',
      'isCorrect': false,
      'answerHistoryJson': '["A","B"]',
      'status': 'answered',
      'startedAt': '2026-07-15T00:00:00Z',
      'updatedAt': '2026-07-15T00:01:00Z',
      'vocabularyEvidence': [
        {'occurrenceId': 3, 'word': 'resilient', 'userMark': 'unknown'},
      ],
    });

    expect(attempt.isCorrect, isFalse);
    expect(attempt.answerHistory, ['A', 'B']);
    expect(attempt.vocabularyEvidence.single['word'], 'resilient');
  });

  test('word inspection keeps token offsets and unknown mark state', () {
    final token = ExamWordToken.fromJson({
      'text': 'known',
      'normalized': 'known',
      'startOffset': 7,
      'endOffset': 12,
    });
    final inspection = ExamWordInspection.fromJson({
      'occurrenceId': 9,
      'entryId': 42,
      'word': 'known',
      'normalized': 'known',
      'meanings': ['\u5df2\u77e5\u7684'],
      'userMark': 'unknown',
      'isUnknown': true,
    });

    expect((token.startOffset, token.endOffset), (7, 12));
    expect(inspection.meanings, ['\u5df2\u77e5\u7684']);
    expect(inspection.isUnknown, isTrue);
  });

  test('annotation state separates current yellow and prior purple words', () {
    final state = ExamAnnotationState.fromJson({
      'currentMarks': [
        {'normalized': 'resilient', 'meaning': '有韧性的'},
      ],
      'priorMarks': [
        {'normalized': 'legacy', 'meaning': '遗留的'},
      ],
      'annotations': [
        {
          'annotationId': 'note-1',
          'questionId': 'q1',
          'scope': 'stem',
          'startOffset': 2,
          'endOffset': 11,
          'selectedText': 'resilient',
          'noteText': '转折依据',
          'color': 'yellow',
        },
      ],
    });

    expect(state.currentMeanings['resilient'], '有韧性的');
    expect(state.priorWords, {'legacy'});
    expect(state.annotations.single.noteText, '转折依据');
  });
}

class _InspectRecordingBridge extends RustBridge {
  String? method;
  String? argument;

  @override
  Future<String> call(String method, [String? argument]) async {
    this.method = method;
    this.argument = argument;
    return jsonEncode({
      'occurrenceId': 1,
      'entryId': 2,
      'word': 'Habits',
      'normalized': 'habits',
      'meanings': ['习惯'],
      'userMark': 'unknown',
      'isUnknown': true,
    });
  }
}
