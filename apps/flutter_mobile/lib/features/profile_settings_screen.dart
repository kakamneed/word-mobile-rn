import 'dart:io';

import 'package:flutter/material.dart';
import 'package:image_picker/image_picker.dart';
import 'package:path_provider/path_provider.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:supabase_flutter/supabase_flutter.dart';

import '../widgets/crocodile_frame_animation.dart';

import '../state/app_state.dart';
import '../sdk/sdk.dart';
import '../supabase/profile_avatar_service.dart';
import '../supabase/reward_image_upload_service.dart';

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
  bool _uploadingRewardImage = false;
  int _avatarIndex = 0;
  String? _avatarImagePath;
  RewardImageUploadEntitlement? _uploadEntitlement;
  List<RewardImage> _rewardImages = const [];

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
    RewardImageUploadEntitlement? uploadEntitlement;
    try {
      final reports = await widget.appState.sdk.reports.getReportsOverview();
      final currentStreak =
          (reports.streakInfo['currentStreak'] as num?)?.toInt() ?? 0;
      uploadEntitlement =
          await widget.appState.sdk.rewardImages.refreshUploadEntitlement(
        currentStreakDays: currentStreak,
      );
    } catch (_) {
      try {
        uploadEntitlement =
            await widget.appState.sdk.rewardImages.getUploadEntitlement();
      } catch (_) {
        uploadEntitlement = null;
      }
    }
    if (!mounted) return;
    setState(() {
      _nameController.text = settings.displayName;
      _avatarIndex = settings.avatarIndex;
      _avatarImagePath = settings.avatarImagePath;
      _uploadEntitlement = uploadEntitlement;
      _loading = false;
    });
    await _loadRewardImages();
  }

  Future<void> _loadRewardImages() async {
    try {
      final images = await widget.appState.sdk.rewardImages.listImages();
      if (!mounted) return;
      setState(() {
        _rewardImages = images;
      });
    } catch (_) {
      if (!mounted) return;
      setState(() {
        _rewardImages = const [];
      });
    }
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
      if (widget.appState.authState.isSignedIn) {
        try {
          final avatarService = ProfileAvatarService();
          if (avatarService.isConfigured) {
            await avatarService.uploadAvatar(sourcePath: pickedImage.path);
          }
        } catch (_) {
          // Local avatar remains available even when cloud avatar sync fails.
        }
      }
      if (!mounted) return;
      setState(() {
        _avatarImagePath = savedImage.path;
      });
    } catch (_) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('澶村儚淇濆瓨澶辫触锛岃閲嶈瘯')),
      );
    }
  }

  void _removeAvatarImage() {
    setState(() {
      _avatarImagePath = null;
    });
  }

  Future<void> _pickRewardImage() async {
    final entitlement = _uploadEntitlement;
    if (entitlement == null || entitlement.availableUploads <= 0) {
      _showSnack('暂无上传机会');
      return;
    }
    final pickedImage = await _imagePicker.pickImage(
      source: ImageSource.gallery,
    );
    if (pickedImage == null) return;

    setState(() {
      _uploadingRewardImage = true;
    });
    try {
      final uploadService = RewardImageUploadService();
      final compressed = await uploadService.compressForRewardUpload(
        sourcePath: pickedImage.path,
      );
      final directory = await getApplicationDocumentsDirectory();
      final imageDirectory = Directory('${directory.path}/reward_images');
      if (!await imageDirectory.exists()) {
        await imageDirectory.create(recursive: true);
      }
      final savedImage = File(
        '${imageDirectory.path}/reward_${DateTime.now().millisecondsSinceEpoch}'
        '${compressed.extension}',
      );
      await File(compressed.path).copy(savedImage.path);
      await widget.appState.sdk.rewardImages.createUpload(
        localPath: savedImage.path,
        mimeType: compressed.mimeType,
        originalFilename: pickedImage.name,
      );
      Object? cloudUploadError;
      if (widget.appState.authState.isSignedIn && uploadService.isConfigured) {
        try {
          await uploadService.uploadCompressedRewardImage(
            image: compressed.copyWith(path: savedImage.path),
            originalFilename: pickedImage.name,
          );
        } catch (error) {
          cloudUploadError = error;
        }
      }
      final updatedEntitlement =
          await widget.appState.sdk.rewardImages.getUploadEntitlement();
      final images = await widget.appState.sdk.rewardImages.listImages();
      if (!mounted) return;
      setState(() {
        _uploadEntitlement = updatedEntitlement;
        _rewardImages = images;
        _uploadingRewardImage = false;
      });
      final sizeLabel = _formatImageSize(compressed.byteLength);
      if (cloudUploadError == null &&
          widget.appState.authState.isSignedIn &&
          uploadService.isConfigured) {
        _showSnack('图片已压缩至 $sizeLabel，并上传等待审核');
      } else if (cloudUploadError != null) {
        _showSnack('图片已压缩至 $sizeLabel；云端上传失败');
      } else {
        _showSnack('图片已压缩至 $sizeLabel，并保存到本地');
      }
    } catch (error) {
      if (!mounted) return;
      setState(() {
        _uploadingRewardImage = false;
      });
      _showSnack('图片提交失败：$error');
    }
  }

  Future<void> _moderateRewardImage(RewardImage image, String status) async {
    try {
      await widget.appState.sdk.rewardImages.moderateImage(
        imageId: image.id,
        status: status,
        reason: status == 'rejected' ? 'local test rejected' : '',
      );
      await _loadRewardImages();
      _showSnack(status == 'approved' ? '已通过本地审核' : '已拒绝本地审核');
    } catch (error) {
      _showSnack('审核操作失败：$error');
    }
  }

  Future<void> _selectRewardImageTag(RewardImage image) async {
    try {
      await widget.appState.sdk.rewardImages.selectLeaderboardTag(
        imageId: image.id,
      );
      await _loadRewardImages();
      _showSnack('已设为排行榜头像');
    } catch (error) {
      _showSnack('设置排行榜头像失败：$error');
    }
  }

  void _showSnack(String message) {
    if (!mounted) return;
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text(message)),
    );
  }

  String _formatImageSize(int bytes) {
    final kb = bytes / 1024;
    if (kb < 10) return '${kb.toStringAsFixed(1)}KB';
    return '${kb.round()}KB';
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
        final cloudAvatarUrl = _cloudAvatarUrlFromAuth();
        final profilePayload = <String, dynamic>{
          'user_id': userId,
          'display_name': displayName,
        };
        if (cloudAvatarUrl != null) {
          profilePayload['avatar_url'] = cloudAvatarUrl;
        }
        await client.from('profiles').upsert(
          profilePayload,
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

    return _buildProfileSettingsBody(context, email, previewSettings);
    /*
      appBar: AppBar(title: const Text('涓汉淇℃伅')),
      body: _loading
          ? const CrocodileLoadingAnimation(label: '鍔犺浇涓?..')
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
                                ? '璁剧疆鏄电О'
                                : previewSettings.displayName.trim(),
                            style: Theme.of(context).textTheme.titleLarge,
                          ),
                          const SizedBox(height: 4),
                          Text(
                            email ?? '褰撳墠璐﹀彿',
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
                    labelText: '鏄电О',
                    hintText: '杈撳叆浣犳兂鏄剧ず鐨勫悕绉?',
                    border: OutlineInputBorder(),
                  ),
                ),
                const SizedBox(height: 24),
                Text(
                  '澶村儚',
                  style: Theme.of(context).textTheme.titleMedium,
                ),
                const SizedBox(height: 12),
                Row(
                  children: [
                    FilledButton.icon(
                      onPressed: _pickAvatarImage,
                      icon: const Icon(Icons.photo_library_outlined),
                      label: const Text('浠庣浉鍐岄€夋嫨'),
                    ),
                    const SizedBox(width: 12),
                    if (previewSettings.hasAvatarImage)
                      TextButton(
                        onPressed: _removeAvatarImage,
                        child: const Text('绉婚櫎鍥剧墖'),
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
                      ? '澶村儚鍥剧墖淇濆瓨鍦ㄦ湰鏈猴紝鍚庣画鍙帴鍏ヤ簯绔悓姝ャ€?'
                      : '鏈€夋嫨鍥剧墖鏃朵娇鐢ㄦ湰鏈哄ご鍍忛璁俱€?',
                  style: Theme.of(context).textTheme.bodySmall,
                ),
                const SizedBox(height: 32),
                if (_uploadEntitlement != null) ...[
                  _RewardImageEntitlementCard(
                    entitlement: _uploadEntitlement!,
                    uploading: _uploadingRewardImage,
                    onUpload: _pickRewardImage,
                  ),
                  const SizedBox(height: 24),
                ],
                if (_rewardImages.isNotEmpty) ...[
                  _RewardImageList(
                    images: _rewardImages,
                    onApprove: (image) => _moderateRewardImage(
                      image,
                      'approved',
                    ),
                    onReject: (image) => _moderateRewardImage(
                      image,
                      'rejected',
                    ),
                    onSelectTag: _selectRewardImageTag,
                  ),
                  const SizedBox(height: 24),
                ],
                FilledButton(
                  onPressed: _saving ? null : _save,
                  child: Text(_saving ? '淇濆瓨涓?..' : '淇濆瓨'),
                ),
              ],
            ),
    */
  }

  String? _cloudAvatarUrlFromAuth() {
    final metadata = Supabase.instance.client.auth.currentUser?.userMetadata;
    final avatarUrl = '${metadata?['avatar_url'] ?? ''}'.trim();
    return avatarUrl.isEmpty ? null : avatarUrl;
  }

  Widget _buildProfileSettingsBody(
    BuildContext context,
    String? email,
    LocalProfileSettings previewSettings,
  ) {
    return Scaffold(
      appBar: AppBar(title: const Text('个人设置')),
      body: _loading
          ? const CrocodileLoadingAnimation(label: '加载中...')
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
                                ? '未设置昵称'
                                : previewSettings.displayName.trim(),
                            style: Theme.of(context).textTheme.titleLarge,
                          ),
                          const SizedBox(height: 4),
                          Text(
                            email ?? '未登录账号',
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
                    hintText: '输入你想展示在排行榜上的名字',
                    border: OutlineInputBorder(),
                  ),
                ),
                const SizedBox(height: 24),
                Text('头像', style: Theme.of(context).textTheme.titleMedium),
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
                      ? '当前使用相册图片作为头像，也可以改选下方颜色。'
                      : '未选择图片时，将使用昵称首字母和下方主题色作为头像。',
                  style: Theme.of(context).textTheme.bodySmall,
                ),
                const SizedBox(height: 32),
                if (_uploadEntitlement != null) ...[
                  _RewardImageEntitlementCard(
                    entitlement: _uploadEntitlement!,
                    uploading: _uploadingRewardImage,
                    onUpload: _pickRewardImage,
                  ),
                  const SizedBox(height: 24),
                ],
                if (_rewardImages.isNotEmpty) ...[
                  _RewardImageList(
                    images: _rewardImages,
                    onApprove: (image) => _moderateRewardImage(
                      image,
                      'approved',
                    ),
                    onReject: (image) => _moderateRewardImage(
                      image,
                      'rejected',
                    ),
                    onSelectTag: _selectRewardImageTag,
                  ),
                  const SizedBox(height: 24),
                ],
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

class _RewardImageEntitlementCard extends StatelessWidget {
  const _RewardImageEntitlementCard({
    required this.entitlement,
    required this.uploading,
    required this.onUpload,
  });

  final RewardImageUploadEntitlement entitlement;
  final bool uploading;
  final VoidCallback onUpload;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: colorScheme.primaryContainer.withValues(alpha: 0.55),
        borderRadius: BorderRadius.circular(16),
        border: Border.all(color: colorScheme.primary.withValues(alpha: 0.18)),
      ),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Row(
          children: [
            Icon(Icons.add_photo_alternate_outlined, color: colorScheme.primary, size: 30),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text('图片上传机会', style: Theme.of(context).textTheme.titleMedium),
                  const SizedBox(height: 4),
                  Text(
                    '剩余 ${entitlement.availableUploads} 次。连续学习 ${entitlement.nextMilestoneStreakDays} 天可获得下一次机会。',
                    style: Theme.of(context).textTheme.bodyMedium,
                  ),
                ],
              ),
            ),
            const SizedBox(width: 12),
            FilledButton.icon(
              onPressed: entitlement.availableUploads <= 0 || uploading ? null : onUpload,
              icon: uploading
                  ? const SizedBox(width: 16, height: 16, child: CircularProgressIndicator(strokeWidth: 2))
                  : const Icon(Icons.upload_file_outlined),
              label: Text(uploading ? '提交中' : '上传'),
            ),
          ],
        ),
      ),
    );
  }
}

class _RewardImageList extends StatelessWidget {
  const _RewardImageList({
    required this.images,
    required this.onApprove,
    required this.onReject,
    required this.onSelectTag,
  });

  final List<RewardImage> images;
  final ValueChanged<RewardImage> onApprove;
  final ValueChanged<RewardImage> onReject;
  final ValueChanged<RewardImage> onSelectTag;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text('排行榜图片', style: Theme.of(context).textTheme.titleMedium),
        const SizedBox(height: 12),
        for (final image in images) ...[
          _RewardImageTile(
            image: image,
            onApprove: () => onApprove(image),
            onReject: () => onReject(image),
            onSelectTag: () => onSelectTag(image),
          ),
          const SizedBox(height: 10),
        ],
      ],
    );
  }
}

class _RewardImageTile extends StatelessWidget {
  const _RewardImageTile({
    required this.image,
    required this.onApprove,
    required this.onReject,
    required this.onSelectTag,
  });

  final RewardImage image;
  final VoidCallback onApprove;
  final VoidCallback onReject;
  final VoidCallback onSelectTag;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final file = File(image.localPath);
    return DecoratedBox(
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(14),
        border: Border.all(color: colorScheme.outlineVariant),
      ),
      child: Padding(
        padding: const EdgeInsets.all(10),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            ClipRRect(
              borderRadius: BorderRadius.circular(10),
              child: file.existsSync()
                  ? Image.file(file, width: 76, height: 76, fit: BoxFit.cover)
                  : Container(width: 76, height: 76, color: colorScheme.surfaceContainerHighest, child: const Icon(Icons.broken_image_outlined)),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(_statusLabel(image), style: Theme.of(context).textTheme.labelLarge?.copyWith(color: _statusColor(colorScheme, image))),
                  const SizedBox(height: 4),
                  Text(
                    image.originalFilename.isEmpty ? image.localPath.split(Platform.pathSeparator).last : image.originalFilename,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                  const SizedBox(height: 8),
                  Wrap(
                    spacing: 8,
                    runSpacing: 4,
                    children: [
                      if (image.moderationStatus == 'pending') ...[
                        OutlinedButton(onPressed: onApprove, child: const Text('通过')),
                        OutlinedButton(onPressed: onReject, child: const Text('拒绝')),
                      ],
                      if (image.isApproved)
                        FilledButton.tonalIcon(
                          onPressed: image.selectedAsTag ? null : onSelectTag,
                          icon: Icon(image.selectedAsTag ? Icons.check_circle_outline : Icons.sell_outlined),
                          label: Text(image.selectedAsTag ? '已选择' : '设为榜单头像'),
                        ),
                    ],
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }

  String _statusLabel(RewardImage image) {
    return switch (image.moderationStatus) {
      'approved' => image.selectedAsTag ? '已作为榜单头像' : '已通过',
      'rejected' => '已拒绝',
      _ => '待审核',
    };
  }

  Color _statusColor(ColorScheme colorScheme, RewardImage image) {
    return switch (image.moderationStatus) {
      'approved' => colorScheme.primary,
      'rejected' => colorScheme.error,
      _ => colorScheme.tertiary,
    };
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
