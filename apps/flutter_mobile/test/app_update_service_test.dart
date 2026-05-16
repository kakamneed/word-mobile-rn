import 'package:flutter_mobile/supabase/app_update_service.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('AppRelease parses Android APK metadata', () {
    final release = AppRelease.fromJson({
      'id': 'release-10400',
      'platform': 'android',
      'runtime': 'flutter',
      'channel': 'stable',
      'version_name': '1.4.0',
      'version_code': 10400,
      'min_supported_code': 10300,
      'force_update': true,
      'apk_path': 'android/stable/app-1.4.0.apk',
      'sha256': 'A' * 64,
      'release_notes': 'Better offline updates.',
      'rollout_percent': 25,
      'published_at': '2026-05-15T09:00:00Z',
    });

    expect(release.versionName, '1.4.0');
    expect(release.versionCode, 10400);
    expect(release.minSupportedCode, 10300);
    expect(release.forceUpdate, isTrue);
    expect(release.sha256, 'a' * 64);
    expect(release.hasAndroidArtifact, isTrue);
  });

  test('AppUpdateCheckResult treats min supported code as forced update', () {
    final release = AppRelease.fromJson({
      'id': 'release-10400',
      'platform': 'android',
      'version_name': '1.4.0',
      'version_code': 10400,
      'min_supported_code': 10300,
      'force_update': false,
      'download_url': 'https://example.test/app.apk',
    });
    final result = AppUpdateCheckResult(
      currentVersionName: '1.2.0',
      currentVersionCode: 10200,
      release: release,
    );

    expect(result.hasUpdate, isTrue);
    expect(result.forceUpdate, isTrue);
  });

  test('AppUpdateService applies rollout buckets conservatively', () {
    final service = AppUpdateService();
    final release = AppRelease.fromJson({
      'id': 'release-10400',
      'platform': 'android',
      'version_name': '1.4.0',
      'version_code': 10400,
      'download_url': 'https://example.test/app.apk',
      'rollout_percent': 0,
    });

    expect(service.isInRolloutForTest(release, 'com.wordmobile'), isFalse);
    final fullRollout = AppRelease.fromJson({
      'id': 'release-10400',
      'platform': 'android',
      'version_name': '1.4.0',
      'version_code': 10400,
      'download_url': 'https://example.test/app.apk',
      'rollout_percent': 100,
    });
    expect(service.isInRolloutForTest(fullRollout, 'com.wordmobile'), isTrue);
  });
}
