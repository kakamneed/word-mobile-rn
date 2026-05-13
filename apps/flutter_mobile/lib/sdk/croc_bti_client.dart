/// Croc BTI client - typed SDK for personality profile sync.
library;

import '../bridge/bridge.dart';

class CrocBtiClient {
  final RustBridge _bridge;
  final BridgeCodec _codec;

  const CrocBtiClient(this._bridge, this._codec);

  Future<Map<String, dynamic>?> getProfile() async {
    final raw = await _bridge.call('getCrocBtiProfile');
    final decoded = _codec.decodeDynamicResponse(raw);
    if (decoded is! Map) return null;
    return decoded.cast<String, dynamic>();
  }

  Future<Map<String, dynamic>> saveProfile({
    required Map<String, dynamic> profile,
  }) async {
    final raw = await _bridge.call(
      'saveCrocBtiProfile',
      _codec.encodeRequest({'profile': profile}),
    );
    final decoded = _codec.decodeDynamicResponse(raw);
    if (decoded is! Map) return <String, dynamic>{};
    return decoded.cast<String, dynamic>();
  }
}
