library;

enum CloudBackendKind {
  supabase,
  wordAdmin,
}

class CloudBackendConfig {
  const CloudBackendConfig({
    required this.kind,
    this.wordAdminApiUrl = '',
  });

  final CloudBackendKind kind;
  final String wordAdminApiUrl;

  static const backendName = String.fromEnvironment(
    'CLOUD_BACKEND',
    defaultValue: 'supabase',
  );

  static const envWordAdminApiUrl = String.fromEnvironment(
    'WORD_ADMIN_API_URL',
  );

  static CloudBackendConfig fromEnvironment() {
    final normalized = backendName.trim().toLowerCase();
    if (normalized == 'word_admin' || normalized == 'word-admin') {
      return const CloudBackendConfig(
        kind: CloudBackendKind.wordAdmin,
        wordAdminApiUrl: envWordAdminApiUrl,
      );
    }
    return const CloudBackendConfig(kind: CloudBackendKind.supabase);
  }

  static bool get usesWordAdmin =>
      fromEnvironment().kind == CloudBackendKind.wordAdmin;

  bool get isConfigured {
    return switch (kind) {
      CloudBackendKind.supabase => false,
      CloudBackendKind.wordAdmin => wordAdminApiUrl.trim().isNotEmpty,
    };
  }
}
