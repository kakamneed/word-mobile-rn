library;

import 'package:flutter/material.dart';
import 'package:shared_preferences/shared_preferences.dart';

enum AppColorStyle {
  forest('forest', 'Forest', Color(0xFF1F6F5E)),
  ocean('ocean', 'Ocean', Color(0xFF1F6FA6)),
  violet('violet', 'Violet', Color(0xFF6F5AA8)),
  ember('ember', 'Ember', Color(0xFFB45F2A));

  const AppColorStyle(this.storageKey, this.label, this.seedColor);

  final String storageKey;
  final String label;
  final Color seedColor;

  static AppColorStyle fromStorageKey(String? value) {
    return AppColorStyle.values.firstWhere(
      (style) => style.storageKey == value,
      orElse: () => AppColorStyle.forest,
    );
  }
}

class ThemeSettingsController extends ChangeNotifier {
  static const _storageKey = 'app_color_style';

  AppColorStyle _style = AppColorStyle.forest;
  bool _loaded = false;

  AppColorStyle get style => _style;
  bool get loaded => _loaded;

  ThemeData get theme => ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: _style.seedColor),
        useMaterial3: true,
      );

  Future<void> load() async {
    final prefs = await SharedPreferences.getInstance();
    _style = AppColorStyle.fromStorageKey(prefs.getString(_storageKey));
    _loaded = true;
    notifyListeners();
  }

  Future<void> setStyle(AppColorStyle style) async {
    if (_style == style && _loaded) return;
    _style = style;
    _loaded = true;
    notifyListeners();
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString(_storageKey, style.storageKey);
  }
}
