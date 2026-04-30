/// Rust bridge via platform channels.
///
/// This is the ONLY file that talks to native code. Flutter code outside
/// this layer must go through the SDK clients, never through this directly.
library;

import 'package:flutter/services.dart';

import 'bridge_error.dart';

/// Channel name shared between Flutter and native sides.
const _channelName = 'com.wordmobile/rust_bridge';

/// The bridge between Flutter and the Rust shared core.
///
/// Uses MethodChannel to call existing Android JNI / iOS ObjC adapters.
/// All calls are JSON string in, JSON string out.
class RustBridge {
  static const MethodChannel _channel = MethodChannel(_channelName);

  const RustBridge();

  /// Initialize the Rust runtime with platform paths.
  /// Must be called before any other bridge method.
  Future<void> initialize() async {
    try {
      await _channel.invokeMethod<void>('initialize');
    } on PlatformException catch (e) {
      throw BridgeError.platform('INIT_FAILED', e.message ?? 'Unknown platform error');
    } on MissingPluginException {
      throw BridgeError.platform(
        'PLUGIN_MISSING',
        'Rust bridge plugin not found on this platform',
      );
    }
  }

  /// Call a bridge method that returns a JSON string.
  Future<String> call(String method, [String? argument]) async {
    try {
      final result = await _channel.invokeMethod<String>(method, argument);
      if (result == null) {
        throw BridgeError.runtime('NULL_RESULT', 'Bridge returned null');
      }
      return result;
    } on PlatformException catch (e) {
      throw BridgeError.fromNative(e.message ?? 'Unknown platform error');
    } on MissingPluginException {
      throw BridgeError.platform(
        'METHOD_MISSING',
        'Method $method not found on native side',
      );
    }
  }

  /// Call a bridge method that returns void (success = empty/failure = error string).
  Future<void> callVoid(String method, [String? argument]) async {
    try {
      await _channel.invokeMethod<String>(method, argument);
    } on PlatformException catch (e) {
      throw BridgeError.fromNative(e.message ?? 'Unknown platform error');
    } on MissingPluginException {
      throw BridgeError.platform(
        'METHOD_MISSING',
        'Method $method not found on native side',
      );
    }
  }

  /// Check if the Rust bridge is available and initialized.
  Future<bool> isAvailable() async {
    try {
      await _channel.invokeMethod<void>('getBridgeStatus');
      return true;
    } catch (_) {
      return false;
    }
  }
}
