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
  String get userMessage {
    switch (kind) {
      case BridgeErrorKind.domain:
        return message;
      case BridgeErrorKind.runtime:
        return '运行时初始化失败，请稍后重试。';
      case BridgeErrorKind.platform:
        return '平台桥接不可用，请检查当前设备构建。';
      case BridgeErrorKind.protocol:
        return '桥接返回的数据格式不正确。';
      case BridgeErrorKind.unsupported:
        return '当前平台暂不支持这个能力。';
    }
  }

  @override
  String toString() => 'BridgeError($kind, $code): $message';
}
