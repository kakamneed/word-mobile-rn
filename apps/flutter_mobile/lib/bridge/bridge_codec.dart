/// Rust bridge codec layer.
///
/// Single responsible layer for Flutter <-> native/Rust message boundary.
/// All bridge calls use JSON string serialization via dart:convert.
library;

import 'dart:convert';

import 'bridge_error.dart';

/// The codec that handles serialization and error mapping for bridge calls.
class BridgeCodec {
  const BridgeCodec();

  /// Encode a request object to JSON string for the native bridge.
  String encodeRequest(Object? request) {
    if (request == null) return '';
    return jsonEncode(request);
  }

  /// Decode a JSON response string from the native bridge.
  /// Throws [BridgeError] on failure.
  Map<String, dynamic> decodeResponse(String raw) {
    if (raw.isEmpty) {
      throw BridgeError.protocol('EMPTY_RESPONSE', 'Bridge returned empty response');
    }
    try {
      final decoded = jsonDecode(raw);
      if (decoded is Map<String, dynamic>) return decoded;
      throw BridgeError.protocol('UNEXPECTED_TYPE', 'Expected JSON object, got ${decoded.runtimeType}');
    } on BridgeError {
      rethrow;
    } on FormatException catch (e) {
      throw BridgeError.protocol('DECODE_FAILED', e.message);
    }
  }

  /// Decode a JSON response that may be a list or object.
  dynamic decodeDynamicResponse(String raw) {
    if (raw.isEmpty) {
      throw BridgeError.protocol('EMPTY_RESPONSE', 'Bridge returned empty response');
    }
    try {
      return jsonDecode(raw);
    } on FormatException catch (e) {
      throw BridgeError.protocol('DECODE_FAILED', e.message);
    }
  }

  /// Decode a response that indicates void success (empty or null).
  /// Returns true on success, throws on error.
  bool decodeVoidResponse(String? raw) {
    if (raw == null || raw.isEmpty) return true;
    throw BridgeError.fromNative(raw);
  }
}
