library;

import 'supabase_auth_service.dart';
import 'supabase_config.dart';

class WordDisputedMeaningService {
  WordDisputedMeaningService({SupabaseAuthService? authService})
    : _authService = authService ?? SupabaseAuthService();

  final SupabaseAuthService _authService;

  bool get isConfigured => SupabaseConfig.isConfigured;

  Future<bool> recordDispute({
    required String entrySourceId,
    required String word,
    required String submittedMeaning,
    required String questionId,
    required String questionType,
  }) async {
    final key = entrySourceId.trim();
    final meaning = submittedMeaning.trim();
    if (!isConfigured || key.isEmpty || meaning.isEmpty) return false;

    await _authService.ensureInitialized();
    final userId = _authService.client.auth.currentUser?.id;
    if (userId == null || userId.isEmpty) return false;

    try {
      await _authService.client.from('word_disputed_meanings').insert({
        'entry_source_id': key,
        'word': word.trim(),
        'user_id': userId,
        'submitted_meaning': meaning,
        'question_id': questionId.trim(),
        'question_type': questionType.trim(),
        'source': 'user_dispute',
        'status': 'pending',
      });
      return true;
    } catch (_) {
      return false;
    }
  }
}
