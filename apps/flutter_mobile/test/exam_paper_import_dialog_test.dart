import 'package:flutter/material.dart';
import 'package:flutter_mobile/features/exam_paper_import_dialog.dart';
import 'package:flutter_mobile/sdk/sdk.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('review dialog edits normalized paper fields before save', (
    tester,
  ) async {
    final draft = ExamPaperImportDraft(
      paper: {
        'schemaVersion': 1,
        'id': 'user-paper-1',
        'exam': 'custom',
        'title': 'Imported paper',
        'year': 2026,
        'source': {'origin': 'user_import'},
        'sections': [
          {
            'id': 'reading',
            'type': 'reading',
            'title': 'Reading',
            'passage': 'Old passage',
            'questions': [
              {
                'id': 'q1',
                'number': 1,
                'kind': 'objective',
                'stem': 'Old stem',
                'choices': [
                  {'label': 'A', 'text': 'Old choice'},
                ],
                'answer': null,
              },
            ],
          },
        ],
      },
      warnings: const ['Please verify the answer'],
      provenance: const {'sourceName': 'paper.txt'},
      rawMediaRetained: false,
    );

    await tester.pumpWidget(
      MaterialApp(
        home: Builder(
          builder: (context) => TextButton(
            onPressed: () => showExamPaperImportReviewDialog(context, draft),
            child: const Text('open'),
          ),
        ),
      ),
    );
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();

    await tester.enterText(
      find.byKey(const ValueKey('exam-import-passage')),
      'Revised passage',
    );
    await tester.enterText(
      find.byKey(const ValueKey('exam-import-stem')),
      'Revised stem',
    );
    await tester.enterText(
      find.byKey(const ValueKey('exam-import-choices')),
      'A: First\nB: Second',
    );
    await tester.enterText(
      find.byKey(const ValueKey('exam-import-answer')),
      'b',
    );
    await tester.tap(find.byKey(const ValueKey('exam-import-save')));
    await tester.pumpAndSettle();

    final section = (draft.paper['sections'] as List).single as Map;
    final question = (section['questions'] as List).single as Map;
    expect(section['passage'], 'Revised passage');
    expect(question['stem'], 'Revised stem');
    expect((question['choices'] as List).length, 2);
    expect(question['answer'], 'B');
  });
}
