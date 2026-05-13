library;

import 'supabase_auth_service.dart';
import 'supabase_config.dart';

class WordComment {
  const WordComment({
    required this.id,
    required this.entrySourceId,
    required this.word,
    required this.body,
    required this.createdAt,
    this.authorName,
  });

  final String id;
  final String entrySourceId;
  final String word;
  final String body;
  final DateTime? createdAt;
  final String? authorName;

  factory WordComment.fromJson(Map<String, dynamic> json) {
    final profile = json['profiles'];
    String? authorName;
    if (profile is Map) {
      final name = '${profile['display_name'] ?? ''}'.trim();
      authorName = name.isEmpty ? null : name;
    }
    return WordComment(
      id: '${json['id'] ?? ''}',
      entrySourceId: '${json['entry_source_id'] ?? ''}',
      word: '${json['word'] ?? ''}',
      body: '${json['body'] ?? ''}'.trim(),
      createdAt: DateTime.tryParse('${json['created_at'] ?? ''}'),
      authorName: authorName,
    );
  }
}

class WordCommentService {
  WordCommentService({SupabaseAuthService? authService})
      : _authService = authService ?? SupabaseAuthService();

  final SupabaseAuthService _authService;

  bool get isConfigured => SupabaseConfig.isConfigured;

  Future<List<WordComment>> fetchComments({
    required String entrySourceId,
    int limit = 50,
  }) async {
    final key = entrySourceId.trim();
    if (!isConfigured || key.isEmpty) return const <WordComment>[];

    await _authService.ensureInitialized();
    final rows = await _authService.client
        .from('word_comments')
        .select('id,entry_source_id,word,body,created_at,profiles(display_name)')
        .eq('entry_source_id', key)
        .order('created_at', ascending: false)
        .limit(limit);

    return rows
        .whereType<Map>()
        .map((row) => WordComment.fromJson(row.cast<String, dynamic>()))
        .where((comment) => comment.id.isNotEmpty && comment.body.isNotEmpty)
        .toList(growable: false);
  }

  Future<WordComment> addComment({
    required String entrySourceId,
    required String word,
    required String body,
  }) async {
    final key = entrySourceId.trim();
    final text = body.trim();
    if (!isConfigured) {
      throw StateError('Supabase is not configured.');
    }
    if (key.isEmpty || text.isEmpty) {
      throw ArgumentError('Comment content is empty.');
    }

    await _authService.ensureInitialized();
    final userId = _authService.client.auth.currentUser?.id;
    if (userId == null || userId.isEmpty) {
      throw StateError('Sign in before commenting.');
    }

    final rows = await _authService.client
        .from('word_comments')
        .insert({
          'entry_source_id': key,
          'word': word.trim(),
          'user_id': userId,
          'body': text,
        })
        .select('id,entry_source_id,word,body,created_at,profiles(display_name)')
        .limit(1);

    final row = rows.whereType<Map>().firstOrNull;
    if (row == null) {
      throw StateError('Comment was not returned by Supabase.');
    }
    return WordComment.fromJson(row.cast<String, dynamic>());
  }
}
