/// Bridge error model.
///
/// Unified error classification for all bridge call failures.
/// Flutter pages only consume these error types, never raw native strings.
library;

/// The category of bridge error.
enum BridgeErrorKind {
  /// Rust domain rule violation (e.g., invalid state transition).
  domain,

  /// Rust runtime not initialized, database unavailable, seed missing.
  runtime,

  /// Android/iOS adapter failure, path not writable, secure storage failed.
  platform,

  /// JSON decode failure, missing field, unexpected null.
  protocol,

  /// Feature not implemented on current platform.
  unsupported,
}

/// A structured error from the Rust bridge.
class BridgeError implements Exception {
  final BridgeErrorKind kind;
  final String code;
  final String message;
  final String? nativeDetail;

  const BridgeError({
    required this.kind,
    required this.code,
    required this.message,
    this.nativeDetail,
  });

  factory BridgeError.domain(String code, String message) => BridgeError(
        kind: BridgeErrorKind.domain,
        code: code,
        message: message,
      );

  factory BridgeError.runtime(String code, String message) => BridgeError(
        kind: BridgeErrorKind.runtime,
        code: code,
        message: message,
      );

  factory BridgeError.platform(String code, String message) => BridgeError(
        kind: BridgeErrorKind.platform,
        code: code,
        message: message,
      );

  factory BridgeError.protocol(String code, String message) => BridgeError(
        kind: BridgeErrorKind.protocol,
        code: code,
        message: message,
      );

  factory BridgeError.unsupported(String code, String message) => BridgeError(
        kind: BridgeErrorKind.unsupported,
        code: code,
        message: message,
      );

  /// Parse a raw error string from the native bridge into a typed error.
  factory BridgeError.fromNative(String rawError) {
    final lower = rawError.toLowerCase();

    if (lower.contains('not found') ||
        lower.contains('no active session') ||
        lower.contains('invalid mode') ||
        lower.contains('not enough words')) {
      return BridgeError.domain('STUDY_RULE', rawError);
    }
    if (lower.contains('runtime') ||
        lower.contains('database') ||
        lower.contains('not initialized') ||
        lower.contains('schema')) {
      return BridgeError.runtime('RUNTIME', rawError);
    }
    if (lower.contains('library_not_loaded') ||
        lower.contains('ffi') ||
        lower.contains('jni') ||
        lower.contains('path')) {
      return BridgeError.platform('PLATFORM', rawError);
    }
    if (lower.contains('json') ||
        lower.contains('decode') ||
        lower.contains('parse') ||
        lower.contains('missing field')) {
      return BridgeError.protocol('DECODE', rawError);
    }

    return BridgeError.domain('UNKNOWN', rawError);
  }

  /// User-friendly description safe to show in the UI.
  String get userMessage => switch (kind) {
        BridgeErrorKind.domain => message,
        BridgeErrorKind.runtime =>
          '\u672c\u5730\u5b66\u4e60\u6570\u636e\u6682\u65f6\u4e0d\u53ef\u7528\uff0c\u8bf7\u91cd\u8bd5\u3002',
        BridgeErrorKind.platform =>
          '\u8bbe\u5907\u6865\u63a5\u670d\u52a1\u4e0d\u53ef\u7528\uff0c\u8bf7\u91cd\u542f\u5e94\u7528\u540e\u518d\u8bd5\u3002',
        BridgeErrorKind.protocol =>
          '\u672c\u5730\u6570\u636e\u683c\u5f0f\u65e0\u6cd5\u8bc6\u522b\uff0c\u8bf7\u66f4\u65b0\u5e94\u7528\u3002',
        BridgeErrorKind.unsupported =>
          '\u5f53\u524d\u8bbe\u5907\u6216\u7248\u672c\u6682\u4e0d\u652f\u6301\u8be5\u529f\u80fd\u3002',
      };

  @override
  String toString() => 'BridgeError($kind, $code): $message';
}
