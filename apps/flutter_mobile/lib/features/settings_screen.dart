import 'package:flutter/material.dart';

import 'theme_settings.dart';

class SettingsScreen extends StatelessWidget {
  const SettingsScreen({super.key, required this.themeController});

  final ThemeSettingsController themeController;

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: themeController,
      builder: (context, _) {
        return Scaffold(
          appBar: AppBar(title: const Text('设置')),
          body: ListView(
            padding: const EdgeInsets.all(16),
            children: [
              Text(
                'Color style',
                style: Theme.of(context).textTheme.titleMedium,
              ),
              const SizedBox(height: 12),
              ...AppColorStyle.values.map(
                (style) => Card(
                  child: ListTile(
                    leading: CircleAvatar(backgroundColor: style.seedColor),
                    title: Text(style.label),
                    trailing: themeController.style == style
                        ? const Icon(Icons.check)
                        : null,
                    onTap: () => themeController.setStyle(style),
                  ),
                ),
              ),
            ],
          ),
        );
      },
    );
  }
}
