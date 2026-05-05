library;

import 'package:shared_preferences/shared_preferences.dart';
import 'package:flutter/foundation.dart';

import 'supabase_auth_service.dart';
import 'supabase_config.dart';

enum AnnouncementLevel {
  info,
  success,
  warning,
  critical;

  static AnnouncementLevel fromWireName(String? value) {
    return AnnouncementLevel.values.firstWhere(
      (level) => level.name == value,
      orElse: () => AnnouncementLevel.info,
    );
  }
}

class CloudAnnouncement {
  const CloudAnnouncement({
    required this.id,
    required this.title,
    required this.body,
    required this.level,
    required this.priority,
    required this.publishedAt,
    required this.startsAt,
    required this.endsAt,
    required this.targetPlatform,
    required this.minAppVersion,
    required this.maxAppVersion,
  });

  final String id;
  final String title;
  final String body;
  final AnnouncementLevel level;
  final int priority;
  final DateTime? publishedAt;
  final DateTime? startsAt;
  final DateTime? endsAt;
  final String targetPlatform;
  final String? minAppVersion;
  final String? maxAppVersion;

  factory CloudAnnouncement.fromJson(Map<String, dynamic> json) {
    return CloudAnnouncement(
      id: '${json['id'] ?? ''}',
      title: '${json['title'] ?? ''}'.trim(),
      body: '${json['body'] ?? ''}'.trim(),
      level: AnnouncementLevel.fromWireName('${json['level'] ?? 'info'}'),
      priority: (json['priority'] as num?)?.toInt() ?? 0,
      publishedAt: DateTime.tryParse('${json['published_at'] ?? ''}'),
      startsAt: DateTime.tryParse('${json['starts_at'] ?? ''}'),
      endsAt: DateTime.tryParse('${json['ends_at'] ?? ''}'),
      targetPlatform: '${json['target_platform'] ?? 'all'}',
      minAppVersion: _nullableTrimmed(json['min_app_version']),
      maxAppVersion: _nullableTrimmed(json['max_app_version']),
    );
  }

  bool get hasContent => id.isNotEmpty && title.isNotEmpty && body.isNotEmpty;

  bool isVisibleAt(DateTime nowUtc) {
    final published = publishedAt;
    final start = startsAt;
    final end = endsAt;
    if (published != null && published.toUtc().isAfter(nowUtc)) return false;
    if (start != null && start.toUtc().isAfter(nowUtc)) return false;
    if (end != null && !end.toUtc().isAfter(nowUtc)) return false;
    return hasContent;
  }

  bool isVisibleOnPlatform(TargetPlatform platform) {
    return targetPlatform == 'all' ||
        targetPlatform ==
            switch (platform) {
              TargetPlatform.android => 'android',
              TargetPlatform.iOS => 'ios',
              _ => 'all',
            };
  }

  static String? _nullableTrimmed(Object? value) {
    final text = '${value ?? ''}'.trim();
    return text.isEmpty ? null : text;
  }
}

class AnnouncementService {
  AnnouncementService({SupabaseAuthService? authService})
    : _authService = authService ?? SupabaseAuthService();

  static const _dismissedStorageKey = 'dismissed_announcement_ids';

  final SupabaseAuthService _authService;

  Future<List<CloudAnnouncement>> fetchActiveAnnouncements({
    int limit = 10,
  }) async {
    if (!SupabaseConfig.isConfigured) {
      return const <CloudAnnouncement>[];
    }

    await _authService.ensureInitialized();
    final rows = await _authService.client
        .from('announcements')
        .select(
          'id,title,body,level,priority,published_at,starts_at,ends_at,'
          'target_platform,min_app_version,max_app_version',
        )
        .order('priority', ascending: false)
        .order('published_at', ascending: false)
        .limit(limit);

    final nowUtc = DateTime.now().toUtc();
    return rows
        .whereType<Map>()
        .map((row) => CloudAnnouncement.fromJson(row.cast<String, dynamic>()))
        .where(
          (announcement) =>
              announcement.isVisibleAt(nowUtc) &&
              announcement.isVisibleOnPlatform(defaultTargetPlatform),
        )
        .toList(growable: false);
  }

  Future<List<CloudAnnouncement>> fetchVisibleAnnouncements({
    int limit = 10,
  }) async {
    final announcements = await fetchActiveAnnouncements(limit: limit);
    if (announcements.isEmpty) return announcements;
    final dismissedIds = await loadDismissedIds();
    return announcements
        .where((announcement) => !dismissedIds.contains(announcement.id))
        .toList(growable: false);
  }

  Future<Set<String>> loadDismissedIds() async {
    final prefs = await SharedPreferences.getInstance();
    return (prefs.getStringList(_dismissedStorageKey) ?? const <String>[])
        .where((id) => id.trim().isNotEmpty)
        .toSet();
  }

  Future<void> dismiss(String id) async {
    final trimmed = id.trim();
    if (trimmed.isEmpty) return;
    final prefs = await SharedPreferences.getInstance();
    final dismissed = await loadDismissedIds();
    dismissed.add(trimmed);
    await prefs.setStringList(
      _dismissedStorageKey,
      dismissed.toList(growable: false)..sort(),
    );
  }
}
