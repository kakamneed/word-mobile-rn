library;

class SupabaseConfig {
  final String url;
  final String anonKey;

  const SupabaseConfig({
    required this.url,
    required this.anonKey,
  });

  static const envUrl = String.fromEnvironment('SUPABASE_URL');
  static const envAnonKey = String.fromEnvironment('SUPABASE_ANON_KEY');

  static bool get isConfigured => envUrl.isNotEmpty && envAnonKey.isNotEmpty;

  static SupabaseConfig? fromEnvironment() {
    if (!isConfigured) return null;
    return const SupabaseConfig(url: envUrl, anonKey: envAnonKey);
  }
}
