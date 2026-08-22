import 'package:flutter_mobile/sdk/sync_client.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('mobile study events use stable per-user identities', () {
    final first = mobileStudyEventRowsForTest(
      userId: '11111111-1111-1111-1111-111111111111',
      payload: const {
        'events': [
          {
            'localResultId': 7,
            'sessionId': 'session-7',
            'occurredAt': '2024-01-01T00:00:30Z',
            'payloadJson': {
              'schemaVersion': 1,
              'eventId': 'word-mobile:session-7:q-7:2024-01-01T00:00:30Z',
              'questionId': 'q-7',
              'entryId': 1,
              'entrySourceId': 'KaoYan_3_1',
              'bookId': 'kaoyan',
              'bookVersion': '2026.1',
              'mode': 'highFrequency',
              'questionType': 'enToCnInput',
              'outcome': 'correct',
              'userResponse': '过程',
              'canonicalAnswer': '过程；进程',
              'responseTimeMs': 1200,
              'hintUsed': true,
              'localDay': '2024-01-01',
              'legacyPointProjection': true,
            },
          },
        ],
      },
    );
    final second = mobileStudyEventRowsForTest(
      userId: '11111111-1111-1111-1111-111111111111',
      payload: const {'events': []},
    );

    expect(first.deviceId, second.deviceId);
    expect(first.rows.single['device_id'], first.deviceId);
    expect(
      first.rows.single['idempotency_key'],
      startsWith('word-mobile:11111111-1111-1111-1111-111111111111:'),
    );
    expect(first.rows.single['event_id'], isNotEmpty);
    expect(first.rows.single['event_type'], 'answer_submitted');
  });
}
