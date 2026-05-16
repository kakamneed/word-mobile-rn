library;

import 'package:supabase_flutter/supabase_flutter.dart';

import 'reward_image_upload_service.dart';
import 'supabase_auth_service.dart';
import 'supabase_config.dart';

class CloudProfileAvatar {
  const CloudProfileAvatar({
    required this.storagePath,
    required this.publicUrl,
    required this.mimeType,
  });

  final String storagePath;
  final String publicUrl;
  final String mimeType;
}

class ProfileAvatarService {
  ProfileAvatarService({
    SupabaseAuthService? authService,
    RewardImageUploadService? uploadService,
  })  : _authService = authService ?? SupabaseAuthService(),
        _uploadService = uploadService ?? RewardImageUploadService();

  final SupabaseAuthService _authService;
  final RewardImageUploadService _uploadService;

  bool get isConfigured => SupabaseConfig.isConfigured;

  Future<CloudProfileAvatar> uploadAvatar({
    required String sourcePath,
  }) async {
    if (!isConfigured) {
      throw StateError('Supabase environment is not configured.');
    }
    await _authService.ensureInitialized();
    final client = _authService.client;
    final user = client.auth.currentUser;
    if (user == null) {
      throw StateError('Sign in before uploading avatar.');
    }

    final compressed = await _uploadService.compressForRewardUpload(
      sourcePath: sourcePath,
    );
    final upload = await _uploadService.uploadCompressedImageToBucket(
      image: compressed,
      bucketId: avatarImageBucketId,
      pathPrefix: 'avatar',
      upsert: true,
    );

    await client.from('profiles').upsert(
      {
        'user_id': user.id,
        'avatar_storage_path': upload.storagePath,
        'avatar_url': upload.publicUrl,
        'avatar_mime_type': compressed.mimeType,
      },
      onConflict: 'user_id',
    );
    await client.auth.updateUser(
      UserAttributes(
        data: {
          'avatar_url': upload.publicUrl,
          'avatar_storage_path': upload.storagePath,
        },
      ),
    );

    return CloudProfileAvatar(
      storagePath: upload.storagePath,
      publicUrl: upload.publicUrl,
      mimeType: compressed.mimeType,
    );
  }
}
