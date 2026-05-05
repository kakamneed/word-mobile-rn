import 'dart:io';

import 'package:flutter/material.dart';
import 'package:image_picker/image_picker.dart';
import 'package:path_provider/path_provider.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:supabase_flutter/supabase_flutter.dart';

import '../state/app_state.dart';

const _displayNameKey = 'account.profile.display_name';
const _avatarIndexKey = 'account.profile.avatar_index';
const _avatarImagePathKey = 'account.profile.avatar_image_path';

String _profileKey(String key, String? userId) {
  final normalized = userId?.trim();
  if (normalized == null || normalized.isEmpty) return key;
  return '$key.$normalized';
}

const profileAvatarColors = [
  Color(0xFFA7F3D0),
  Color(0xFFBFDBFE),
  Color(0xFFFDE68A),
  Color(0xFFFBCFE8),
  Color(0xFFC4B5FD),
  Color(0xFFFED7AA),
];

class LocalProfileSettings {
  const LocalProfileSettings({
    this.displayName = '',
    this.avatarIndex = 0,
    this.avatarImagePath,
  });

  final String displayName;
  final int avatarIndex;
  final String? avatarImagePath;

  bool get hasAvatarImage =>
      avatarImagePath != null && avatarImagePath!.trim().isNotEmpty;

  Color get avatarColor {
    final safeIndex = avatarIndex.clamp(0, profileAvatarColors.length - 1);
    return profileAvatarColors[safeIndex];
  }

  String avatarText(String? fallbackEmail) {
    final source = displayName.trim().isNotEmpty
        ? displayName.trim()
        : (fallbackEmail ?? '').trim();
    if (source.isEmpty) return '';
    return source.substring(0, 1).toUpperCase();
  }

  LocalProfileSettings copyWith({
    String? displayName,
    int? avatarIndex,
    String? avatarImagePath,
  }) {
    return LocalProfileSettings(
      displayName: displayName ?? this.displayName,
      avatarIndex: avatarIndex ?? this.avatarIndex,
      avatarImagePath: avatarImagePath ?? this.avatarImagePath,
    );
  }
}

Future<LocalProfileSettings> loadLocalProfileSettings({String? userId}) async {
  final prefs = await SharedPreferences.getInstance();
  final hasScopedUser = userId?.trim().isNotEmpty ?? false;
  return LocalProfileSettings(
    displayName:
        prefs.getString(_profileKey(_displayNameKey, userId)) ??
        (hasScopedUser ? '' : prefs.getString(_displayNameKey) ?? ''),
    avatarIndex:
        prefs.getInt(_profileKey(_avatarIndexKey, userId)) ??
        (hasScopedUser ? 0 : prefs.getInt(_avatarIndexKey) ?? 0),
    avatarImagePath:
        prefs.getString(_profileKey(_avatarImagePathKey, userId)) ??
        (hasScopedUser ? null : prefs.getString(_avatarImagePathKey)),
  );
}

Future<void> saveLocalProfileSettings(
  LocalProfileSettings settings, {
  String? userId,
}) async {
  final prefs = await SharedPreferences.getInstance();
  await prefs.setString(
    _profileKey(_displayNameKey, userId),
    settings.displayName.trim(),
  );
  await prefs.setInt(_profileKey(_avatarIndexKey, userId), settings.avatarIndex);
  final avatarImagePath = settings.avatarImagePath?.trim();
  if (avatarImagePath == null || avatarImagePath.isEmpty) {
    await prefs.remove(_profileKey(_avatarImagePathKey, userId));
  } else {
    await prefs.setString(_profileKey(_avatarImagePathKey, userId), avatarImagePath);
  }
}

class ProfileSettingsScreen extends StatefulWidget {
  const ProfileSettingsScreen({
    super.key,
    required this.appState,
  });

  final AppState appState;

  @override
  State<ProfileSettingsScreen> createState() => _ProfileSettingsScreenState();
}

class _ProfileSettingsScreenState extends State<ProfileSettingsScreen> {
  final _nameController = TextEditingController();
  final _imagePicker = ImagePicker();

  bool _loading = true;
  bool _saving = false;
  int _avatarIndex = 0;
  String? _avatarImagePath;

  @override
  void initState() {
    super.initState();
    _nameController.addListener(_refreshPreview);
    _load();
  }

  @override
  void dispose() {
    _nameController.removeListener(_refreshPreview);
    _nameController.dispose();
    super.dispose();
  }

  void _refreshPreview() {
    if (mounted) {
      setState(() {});
    }
  }

  Future<void> _load() async {
    final settings = await loadLocalProfileSettings(
      userId: widget.appState.authState.userId,
    );
    if (!mounted) return;
    setState(() {
      _nameController.text = settings.displayName;
      _avatarIndex = settings.avatarIndex;
      _avatarImagePath = settings.avatarImagePath;
      _loading = false;
    });
  }

  Future<void> _pickAvatarImage() async {
    final pickedImage = await _imagePicker.pickImage(
      source: ImageSource.gallery,
      maxWidth: 512,
      maxHeight: 512,
      imageQuality: 88,
    );
    if (pickedImage == null) return;

    try {
      final directory = await getApplicationDocumentsDirectory();
      final avatarDirectory = Directory('${directory.path}/profile');
      if (!await avatarDirectory.exists()) {
        await avatarDirectory.create(recursive: true);
      }
      final extension = _extensionFor(pickedImage.path);
      final savedImage = File(
        '${avatarDirectory.path}/avatar_${DateTime.now().millisecondsSinceEpoch}$extension',
      );
      await File(pickedImage.path).copy(savedImage.path);
      if (!mounted) return;
      setState(() {
        _avatarImagePath = savedImage.path;
      });
    } catch (_) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('头像保存失败，请重试')),
      );
    }
  }

  void _removeAvatarImage() {
    setState(() {
      _avatarImagePath = null;
    });
  }

  Future<void> _save() async {
    setState(() {
      _saving = true;
    });
    final userId = widget.appState.authState.userId;
    await saveLocalProfileSettings(
      LocalProfileSettings(
        displayName: _nameController.text,
        avatarIndex: _avatarIndex,
        avatarImagePath: _avatarImagePath,
      ),
      userId: userId,
    );
    if (userId != null && userId.isNotEmpty) {
      try {
        final client = Supabase.instance.client;
        final displayName = _nameController.text.trim();
        await client.from('profiles').upsert(
          {
            'user_id': userId,
            'display_name': displayName,
          },
          onConflict: 'user_id',
        );
        await client.auth.updateUser(
          UserAttributes(data: {'display_name': displayName}),
        );
      } catch (_) {
        // Local profile remains available even when cloud profile sync fails.
      }
    }
    if (!mounted) return;
    Navigator.of(context).pop(true);
  }

  @override
  Widget build(BuildContext context) {
    final email = widget.appState.authState.userEmail;
    final previewSettings = LocalProfileSettings(
      displayName: _nameController.text,
      avatarIndex: _avatarIndex,
      avatarImagePath: _avatarImagePath,
    );

    return Scaffold(
      appBar: AppBar(title: const Text('个人信息')),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : ListView(
              padding: const EdgeInsets.all(20),
              children: [
                Row(
                  children: [
                    CircleAvatar(
                      radius: 36,
                      backgroundColor: previewSettings.avatarColor,
                      foregroundImage: _avatarImageProvider(previewSettings),
                      child: previewSettings.hasAvatarImage
                          ? null
                          : _avatarLabel(previewSettings, email),
                    ),
                    const SizedBox(width: 16),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            previewSettings.displayName.trim().isEmpty
                                ? '设置昵称'
                                : previewSettings.displayName.trim(),
                            style: Theme.of(context).textTheme.titleLarge,
                          ),
                          const SizedBox(height: 4),
                          Text(
                            email ?? '当前账号',
                            style: Theme.of(context).textTheme.bodyMedium,
                          ),
                        ],
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 28),
                TextField(
                  controller: _nameController,
                  textInputAction: TextInputAction.done,
                  decoration: const InputDecoration(
                    labelText: '昵称',
                    hintText: '输入你想显示的名称',
                    border: OutlineInputBorder(),
                  ),
                ),
                const SizedBox(height: 24),
                Text(
                  '头像',
                  style: Theme.of(context).textTheme.titleMedium,
                ),
                const SizedBox(height: 12),
                Row(
                  children: [
                    FilledButton.icon(
                      onPressed: _pickAvatarImage,
                      icon: const Icon(Icons.photo_library_outlined),
                      label: const Text('从相册选择'),
                    ),
                    const SizedBox(width: 12),
                    if (previewSettings.hasAvatarImage)
                      TextButton(
                        onPressed: _removeAvatarImage,
                        child: const Text('移除图片'),
                      ),
                  ],
                ),
                const SizedBox(height: 12),
                Wrap(
                  spacing: 12,
                  runSpacing: 12,
                  children: [
                    for (var index = 0;
                        index < profileAvatarColors.length;
                        index += 1)
                      _AvatarChoice(
                        color: profileAvatarColors[index],
                        selected: index == _avatarIndex,
                        onTap: () {
                          setState(() {
                            _avatarIndex = index;
                          });
                        },
                      ),
                  ],
                ),
                const SizedBox(height: 8),
                Text(
                  previewSettings.hasAvatarImage
                      ? '头像图片保存在本机，后续可接入云端同步。'
                      : '未选择图片时使用本机头像预设。',
                  style: Theme.of(context).textTheme.bodySmall,
                ),
                const SizedBox(height: 32),
                FilledButton(
                  onPressed: _saving ? null : _save,
                  child: Text(_saving ? '保存中...' : '保存'),
                ),
              ],
            ),
    );
  }

  Widget _avatarLabel(LocalProfileSettings settings, String? email) {
    final text = settings.avatarText(email);
    if (text.isEmpty) {
      return const Icon(Icons.person, color: Color(0xFF104E3D), size: 34);
    }
    return Text(
      text,
      style: const TextStyle(
        color: Color(0xFF104E3D),
        fontSize: 28,
        fontWeight: FontWeight.w700,
      ),
    );
  }

  ImageProvider? _avatarImageProvider(LocalProfileSettings settings) {
    final imagePath = settings.avatarImagePath;
    if (imagePath == null || imagePath.trim().isEmpty) return null;
    final file = File(imagePath);
    if (!file.existsSync()) return null;
    return FileImage(file);
  }

  String _extensionFor(String path) {
    final dotIndex = path.lastIndexOf('.');
    if (dotIndex < 0 || dotIndex == path.length - 1) return '.jpg';
    final extension = path.substring(dotIndex).toLowerCase();
    if (extension.length > 8) return '.jpg';
    return extension;
  }
}

class _AvatarChoice extends StatelessWidget {
  const _AvatarChoice({
    required this.color,
    required this.selected,
    required this.onTap,
  });

  final Color color;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return InkWell(
      borderRadius: BorderRadius.circular(28),
      onTap: onTap,
      child: Container(
        width: 48,
        height: 48,
        decoration: BoxDecoration(
          shape: BoxShape.circle,
          color: color,
          border: Border.all(
            color: selected
                ? Theme.of(context).colorScheme.primary
                : Colors.transparent,
            width: 3,
          ),
        ),
        child: selected
            ? Icon(
                Icons.check,
                color: Theme.of(context).colorScheme.primary,
              )
            : null,
      ),
    );
  }
}
