library;

import 'dart:io';

import 'package:convert/convert.dart';
import 'package:crypto/crypto.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:path_provider/path_provider.dart';

import 'supabase_auth_service.dart';
import 'supabase_config.dart';

class AppRelease {
  const AppRelease({
    required this.id,
    required this.platform,
    required this.runtime,
    required this.channel,
    required this.versionName,
    required this.versionCode,
    required this.minSupportedCode,
    required this.forceUpdate,
    required this.apkPath,
    required this.downloadUrl,
    required this.sha256,
    required this.releaseNotes,
    required this.rolloutPercent,
    required this.publishedAt,
  });

  final String id;
  final String platform;
  final String runtime;
  final String channel;
  final String versionName;
  final int versionCode;
  final int minSupportedCode;
  final bool forceUpdate;
  final String? apkPath;
  final String? downloadUrl;
  final String? sha256;
  final String releaseNotes;
  final int rolloutPercent;
  final DateTime? publishedAt;

  factory AppRelease.fromJson(Map<String, dynamic> json) {
    return AppRelease(
      id: '${json['id'] ?? ''}',
      platform: '${json['platform'] ?? ''}',
      runtime: '${json['runtime'] ?? 'flutter'}',
      channel: '${json['channel'] ?? 'stable'}',
      versionName: '${json['version_name'] ?? ''}'.trim(),
      versionCode: (json['version_code'] as num?)?.toInt() ?? 0,
      minSupportedCode: (json['min_supported_code'] as num?)?.toInt() ?? 0,
      forceUpdate: json['force_update'] == true,
      apkPath: _nullableTrimmed(json['apk_path']),
      downloadUrl: _nullableTrimmed(json['download_url']),
      sha256: _nullableTrimmed(json['sha256'])?.toLowerCase(),
      releaseNotes: '${json['release_notes'] ?? ''}'.trim(),
      rolloutPercent: (json['rollout_percent'] as num?)?.toInt() ?? 100,
      publishedAt: DateTime.tryParse('${json['published_at'] ?? ''}'),
    );
  }

  bool get hasAndroidArtifact =>
      platform == 'android' &&
      ((apkPath != null && apkPath!.isNotEmpty) ||
          (downloadUrl != null && downloadUrl!.isNotEmpty));

  static String? _nullableTrimmed(Object? value) {
    final text = '${value ?? ''}'.trim();
    return text.isEmpty ? null : text;
  }
}

class AppUpdateCheckResult {
  const AppUpdateCheckResult({
    required this.currentVersionName,
    required this.currentVersionCode,
    required this.release,
  });

  final String currentVersionName;
  final int currentVersionCode;
  final AppRelease? release;

  bool get hasUpdate => release != null;

  bool get forceUpdate {
    final target = release;
    if (target == null) return false;
    return target.forceUpdate || currentVersionCode < target.minSupportedCode;
  }
}

class AppUpdateDownload {
  const AppUpdateDownload({required this.file, required this.sha256});

  final File file;
  final String sha256;
}

class AppUpdateService {
  AppUpdateService({SupabaseAuthService? authService})
    : _authService = authService ?? SupabaseAuthService();

  final SupabaseAuthService _authService;
  static const MethodChannel _channel = MethodChannel(
    'com.wordmobile/rust_bridge',
  );

  Future<AppUpdateCheckResult> checkLatest({
    String channel = 'stable',
    PackageInfo? packageInfo,
  }) async {
    final info = packageInfo ?? await PackageInfo.fromPlatform();
    final currentCode = int.tryParse(info.buildNumber) ?? 0;
    if (!SupabaseConfig.isConfigured ||
        defaultTargetPlatform != TargetPlatform.android) {
      return AppUpdateCheckResult(
        currentVersionName: info.version,
        currentVersionCode: currentCode,
        release: null,
      );
    }

    await _authService.ensureInitialized();
    final rows = await _authService.client
        .from('app_releases')
        .select(
          'id,platform,runtime,channel,version_name,version_code,'
          'min_supported_code,force_update,apk_path,download_url,sha256,'
          'release_notes,rollout_percent,published_at',
        )
        .eq('platform', 'android')
        .eq('runtime', 'flutter')
        .eq('channel', channel)
        .eq('enabled', true)
        .lte('published_at', DateTime.now().toUtc().toIso8601String())
        .gt('version_code', currentCode)
        .order('version_code', ascending: false)
        .limit(1);

    final release = rows
        .whereType<Map>()
        .map((row) => AppRelease.fromJson(row.cast<String, dynamic>()))
        .where((candidate) => candidate.hasAndroidArtifact)
        .where((candidate) => isInRolloutForTest(candidate, info.packageName))
        .firstOrNull;

    return AppUpdateCheckResult(
      currentVersionName: info.version,
      currentVersionCode: currentCode,
      release: release,
    );
  }

  Future<String> resolveDownloadUrl(AppRelease release) async {
    final directUrl = release.downloadUrl;
    if (directUrl != null && directUrl.isNotEmpty) return directUrl;
    final apkPath = release.apkPath;
    if (apkPath == null || apkPath.isEmpty) {
      throw StateError('Release does not include an APK path or download URL.');
    }
    await _authService.ensureInitialized();
    return _authService.client.storage
        .from('app-releases')
        .createSignedUrl(apkPath, 60 * 30);
  }

  Future<AppUpdateDownload> downloadApk(
    AppRelease release, {
    void Function(int received, int? total)? onProgress,
  }) async {
    final url = await resolveDownloadUrl(release);
    final directory = await getTemporaryDirectory();
    final file = File(
      '${directory.path}${Platform.pathSeparator}word-mobile-${release.versionCode}.apk',
    );
    final request = await HttpClient().getUrl(Uri.parse(url));
    final response = await request.close();
    if (response.statusCode < 200 || response.statusCode >= 300) {
      throw HttpException(
        'APK download failed with HTTP ${response.statusCode}',
        uri: Uri.parse(url),
      );
    }

    final sink = file.openWrite();
    final digestSink = AccumulatorSink<Digest>();
    final hashSink = sha256.startChunkedConversion(digestSink);
    var received = 0;
    try {
      await for (final chunk in response) {
        received += chunk.length;
        sink.add(chunk);
        hashSink.add(chunk);
        onProgress?.call(received, response.contentLength);
      }
    } finally {
      await sink.close();
      hashSink.close();
    }

    final actualHash = digestSink.events.single.toString();
    final expectedHash = release.sha256;
    if (expectedHash != null &&
        expectedHash.isNotEmpty &&
        expectedHash != actualHash) {
      await file.delete().catchError((_) => file);
      throw StateError(
        'Downloaded APK checksum did not match release metadata.',
      );
    }
    return AppUpdateDownload(file: file, sha256: actualHash);
  }

  Future<void> requestInstall(File apkFile) async {
    await _channel.invokeMethod<void>('installApk', apkFile.path);
  }

  @visibleForTesting
  bool isInRolloutForTest(AppRelease release, String packageName) {
    if (release.rolloutPercent >= 100) return true;
    if (release.rolloutPercent <= 0) return false;
    final source = '${release.id}:$packageName';
    final bucket = source.codeUnits.fold<int>(
      0,
      (value, codeUnit) => (value + codeUnit) % 100,
    );
    return bucket < release.rolloutPercent;
  }
}
