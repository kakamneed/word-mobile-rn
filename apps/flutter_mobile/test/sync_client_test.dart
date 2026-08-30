import 'package:flutter_mobile/sdk/sync_client.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('word disputed meaning payload maps to Supabase row', () {
    final row = wordDisputedMeaningSyncRowForTest(
      userId: 'user-1',
      payload: const {
        'entrySourceId': 'entry-1',
        'word': 'condemn',
        'submittedMeaning': '谴责',
        'questionId': 'q1',
        'questionType': 'exampleToCnChoice',
      },
    );

    expect(row['user_id'], 'user-1');
    expect(row['entry_source_id'], 'entry-1');
    expect(row['word'], 'condemn');
    expect(row['submitted_meaning'], '谴责');
    expect(row['question_id'], 'q1');
    expect(row['question_type'], 'exampleToCnChoice');
    expect(row['source'], 'user_dispute');
    expect(row['status'], 'pending');
  });

  test('word disputed meaning payload requires stable local identity', () {
    expect(
      () => wordDisputedMeaningSyncRowForTest(
        userId: 'user-1',
        payload: const {'submittedMeaning': '谴责'},
      ),
      throwsFormatException,
    );
    expect(
      () => wordDisputedMeaningSyncRowForTest(
        userId: 'user-1',
        payload: const {'entrySourceId': 'entry-1'},
      ),
      throwsFormatException,
    );
  });
}
