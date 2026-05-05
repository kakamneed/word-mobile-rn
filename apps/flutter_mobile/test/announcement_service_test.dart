import 'package:flutter/foundation.dart';
import 'package:flutter_mobile/supabase/announcement_service.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

void main() {
  test('CloudAnnouncement parses and filters active windows', () {
    final now = DateTime.utc(2026, 5, 5, 12);
    final announcement = CloudAnnouncement.fromJson({
      'id': 'announcement-1',
      'title': 'Maintenance',
      'body': 'Cloud sync will be briefly unavailable.',
      'level': 'warning',
      'priority': 20,
      'published_at': '2026-05-05T10:00:00Z',
      'starts_at': '2026-05-05T11:00:00Z',
      'ends_at': '2026-05-05T13:00:00Z',
      'target_platform': 'all',
    });

    expect(announcement.id, 'announcement-1');
    expect(announcement.level, AnnouncementLevel.warning);
    expect(announcement.isVisibleAt(now), isTrue);
    expect(announcement.isVisibleAt(DateTime.utc(2026, 5, 5, 14)), isFalse);
    expect(announcement.isVisibleOnPlatform(TargetPlatform.android), isTrue);
    expect(announcement.isVisibleOnPlatform(TargetPlatform.iOS), isTrue);
  });

  test('CloudAnnouncement honors platform targeting', () {
    final announcement = CloudAnnouncement.fromJson({
      'id': 'announcement-android',
      'title': 'Android only',
      'body': 'This is only for Android users.',
      'target_platform': 'android',
    });

    expect(announcement.isVisibleOnPlatform(TargetPlatform.android), isTrue);
    expect(announcement.isVisibleOnPlatform(TargetPlatform.iOS), isFalse);
  });

  test(
    'AnnouncementService stores dismissed announcement ids locally',
    () async {
      SharedPreferences.setMockInitialValues({});
      final service = AnnouncementService();

      expect(await service.loadDismissedIds(), isEmpty);

      await service.dismiss('announcement-1');
      await service.dismiss('announcement-2');
      await service.dismiss('announcement-1');

      expect(await service.loadDismissedIds(), {
        'announcement-1',
        'announcement-2',
      });
    },
  );
}
