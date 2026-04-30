library;

import 'dart:convert';

import 'package:flutter/services.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../bridge/bridge.dart';

class TodayRewardState {
  const TodayRewardState({
    required this.todayDate,
    this.rewardId,
    this.claimedAt,
  });

  final String todayDate;
  final String? rewardId;
  final String? claimedAt;

  bool get hasReward => rewardId != null && rewardId!.isNotEmpty;

  factory TodayRewardState.fromJson(Map<String, dynamic> json) =>
      TodayRewardState(
        todayDate: json['todayDate'] as String? ?? '',
        rewardId: json['rewardId'] as String?,
        claimedAt: json['claimedAt'] as String?,
      );
}

class RewardClient {
  const RewardClient(this._bridge, this._codec);

  static const _localStateKey = 'today_reward_state_json';

  final RustBridge _bridge;
  final BridgeCodec _codec;

  Future<TodayRewardState> getTodayRewardState() async {
    final today = _todayDate();
    final local = await _readLocalState(today);
    if (local != null) return local;

    try {
      final raw = await _bridge.call('getTodayRewardState');
      final json = _codec.decodeResponse(raw);
      final state = TodayRewardState.fromJson(json);
      if (state.todayDate == today && state.hasReward) {
        await _writeLocalState(state);
      }
      return state.todayDate.isEmpty ? TodayRewardState(todayDate: today) : state;
    } catch (_) {
      return TodayRewardState(todayDate: today);
    }
  }

  Future<TodayRewardState> drawTodayReward(String rewardId) async {
    final raw = await _bridge.call(
      'drawTodayReward',
      _codec.encodeRequest({'rewardId': rewardId}),
    );
    final json = _codec.decodeResponse(raw);
    return TodayRewardState.fromJson(json);
  }

  Future<TodayRewardState> saveTodayReward(String rewardId) async {
    final state = TodayRewardState(
      todayDate: _todayDate(),
      rewardId: rewardId,
      claimedAt: DateTime.now().toIso8601String(),
    );
    await _writeLocalState(state);
    return state;
  }

  Future<void> saveRewardImageToGallery(String assetPath, String rewardId) async {
    final data = await rootBundle.load(assetPath);
    final bytes = data.buffer.asUint8List(data.offsetInBytes, data.lengthInBytes);
    final extension = assetPath.split('.').last.toLowerCase();
    final mimeType = extension == 'png'
        ? 'image/png'
        : extension == 'jpg' || extension == 'jpeg'
        ? 'image/jpeg'
        : 'image/webp';
    await _bridge.call(
      'saveImageToGallery',
      _codec.encodeRequest({
        'fileName': 'word_mobile_reward_${_todayDate()}_$rewardId.$extension',
        'mimeType': mimeType,
        'bytesBase64': base64Encode(bytes),
      }),
    );
  }

  Future<TodayRewardState?> _readLocalState(String today) async {
    final prefs = await SharedPreferences.getInstance();
    final raw = prefs.getString(_localStateKey);
    if (raw == null || raw.isEmpty) return null;
    try {
      final decoded = jsonDecode(raw);
      if (decoded is! Map<String, dynamic>) return null;
      final state = TodayRewardState.fromJson(decoded);
      if (state.todayDate != today || !state.hasReward) return null;
      return state;
    } catch (_) {
      return null;
    }
  }

  Future<void> _writeLocalState(TodayRewardState state) async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString(
      _localStateKey,
      jsonEncode({
        'todayDate': state.todayDate,
        'rewardId': state.rewardId,
        'claimedAt': state.claimedAt,
      }),
    );
  }

  String _todayDate() => DateTime.now().toIso8601String().split('T').first;
}
