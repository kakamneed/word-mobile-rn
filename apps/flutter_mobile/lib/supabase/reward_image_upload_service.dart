library;

import 'dart:io';
import 'dart:typed_data';

import 'package:flutter_image_compress/flutter_image_compress.dart';
import 'package:path_provider/path_provider.dart';
import 'package:supabase_flutter/supabase_flutter.dart';

import 'supabase_auth_service.dart';
import 'supabase_config.dart';

const rewardImageTargetMaxDimension = 540;
const rewardImageTargetQuality = 60;
const rewardImageSoftMaxBytes = 64 * 1024;
const rewardImageBucketId = 'reward-images';
const avatarImageBucketId = 'avatars';

class CompressedRewardImage {
  const CompressedRewardImage({
    required this.path,
    required this.extension,
    required this.mimeType,
    required this.byteLength,
    required this.originalByteLength,
    required this.quality,
    required this.usedFallbackFormat,
  });

  final String path;
  final String extension;
  final String mimeType;
  final int byteLength;
  final int originalByteLength;
  final int quality;
  final bool usedFallbackFormat;

  CompressedRewardImage copyWith({String? path}) {
    return CompressedRewardImage(
      path: path ?? this.path,
      extension: extension,
      mimeType: mimeType,
      byteLength: byteLength,
      originalByteLength: originalByteLength,
      quality: quality,
      usedFallbackFormat: usedFallbackFormat,
    );
  }
}

class CloudRewardImageUpload {
  const CloudRewardImageUpload({
    required this.imageId,
    required this.storagePath,
    required this.publicUrl,
    required this.originalFilename,
    required this.mimeType,
  });

  final String imageId;
  final String storagePath;
  final String publicUrl;
  final String originalFilename;
  final String mimeType;
}

class RewardImageUploadService {
  RewardImageUploadService({SupabaseAuthService? authService})
      : _authService = authService ?? SupabaseAuthService();

  final SupabaseAuthService _authService;

  bool get isConfigured => SupabaseConfig.isConfigured;

  Future<CompressedRewardImage> compressForRewardUpload({
    required String sourcePath,
  }) async {
    final sourceFile = File(sourcePath);
    final originalByteLength = await sourceFile.length();
    final directory = await getTemporaryDirectory();
    final outputDirectory = Directory('${directory.path}/reward_uploads');
    if (!await outputDirectory.exists()) {
      await outputDirectory.create(recursive: true);
    }

    final stamp = DateTime.now().microsecondsSinceEpoch;
    final webp = await _compressCandidates(
      sourcePath: sourcePath,
      outputDirectory: outputDirectory,
      stamp: stamp,
      extension: '.webp',
      mimeType: 'image/webp',
      format: CompressFormat.webp,
      qualities: const [60, 50, 40],
      originalByteLength: originalByteLength,
      usedFallbackFormat: false,
    );
    if (webp != null) return webp;

    final jpeg = await _compressCandidates(
      sourcePath: sourcePath,
      outputDirectory: outputDirectory,
      stamp: stamp,
      extension: '.jpg',
      mimeType: 'image/jpeg',
      format: CompressFormat.jpeg,
      qualities: const [60, 50, 40],
      originalByteLength: originalByteLength,
      usedFallbackFormat: true,
    );
    if (jpeg != null) return jpeg;

    throw StateError('Image compression failed.');
  }

  Future<CloudRewardImageUpload> uploadCompressedRewardImage({
    required CompressedRewardImage image,
    required String originalFilename,
  }) async {
    if (!isConfigured) {
      throw StateError('Supabase environment is not configured.');
    }
    await _authService.ensureInitialized();
    final client = _authService.client;
    final user = client.auth.currentUser;
    if (user == null) {
      throw StateError('Sign in before uploading reward images.');
    }

    final upload = await uploadCompressedImageToBucket(
      image: image,
      bucketId: rewardImageBucketId,
      pathPrefix: 'reward',
      upsert: false,
    );

    final row =
        await client
            .from('reward_images')
            .insert({
              'owner_id': user.id,
              'storage_path': upload.storagePath,
              'public_url': upload.publicUrl,
              'original_filename': originalFilename,
              'mime_type': image.mimeType,
              'moderation_status': 'pending',
              'selected_as_tag': false,
              'is_withdrawn': false,
            })
            .select(
              'image_id, storage_path, public_url, original_filename, mime_type',
            )
            .single();

    return CloudRewardImageUpload(
      imageId: row['image_id'] as String,
      storagePath: row['storage_path'] as String,
      publicUrl: row['public_url'] as String? ?? upload.publicUrl,
      originalFilename: row['original_filename'] as String? ?? originalFilename,
      mimeType: row['mime_type'] as String? ?? image.mimeType,
    );
  }

  Future<({String storagePath, String publicUrl})> uploadCompressedImageToBucket({
    required CompressedRewardImage image,
    required String bucketId,
    required String pathPrefix,
    bool upsert = true,
  }) async {
    if (!isConfigured) {
      throw StateError('Supabase environment is not configured.');
    }
    await _authService.ensureInitialized();
    final client = _authService.client;
    final user = client.auth.currentUser;
    if (user == null) {
      throw StateError('Sign in before uploading images.');
    }

    final bytes = await File(image.path).readAsBytes();
    final storagePath =
        '${user.id}/${pathPrefix}_${DateTime.now().millisecondsSinceEpoch}${image.extension}';

    await client.storage
        .from(bucketId)
        .uploadBinary(
          storagePath,
          Uint8List.fromList(bytes),
          fileOptions: FileOptions(
            cacheControl: '3600',
            contentType: image.mimeType,
            upsert: upsert,
          ),
        );

    final publicUrl = client.storage.from(bucketId).getPublicUrl(storagePath);
    return (storagePath: storagePath, publicUrl: publicUrl);
  }

  Future<CompressedRewardImage?> _compressCandidates({
    required String sourcePath,
    required Directory outputDirectory,
    required int stamp,
    required String extension,
    required String mimeType,
    required CompressFormat format,
    required List<int> qualities,
    required int originalByteLength,
    required bool usedFallbackFormat,
  }) async {
    CompressedRewardImage? smallest;
    for (final quality in qualities) {
      final targetPath =
          '${outputDirectory.path}/reward_${stamp}_q$quality$extension';
      final compressed = await FlutterImageCompress.compressAndGetFile(
        sourcePath,
        targetPath,
        minWidth: rewardImageTargetMaxDimension,
        minHeight: rewardImageTargetMaxDimension,
        quality: quality,
        format: format,
        keepExif: false,
      );
      if (compressed == null) continue;
      final file = File(compressed.path);
      if (!await file.exists()) continue;
      final byteLength = await file.length();
      final candidate = CompressedRewardImage(
        path: compressed.path,
        extension: extension,
        mimeType: mimeType,
        byteLength: byteLength,
        originalByteLength: originalByteLength,
        quality: quality,
        usedFallbackFormat: usedFallbackFormat,
      );
      if (byteLength <= rewardImageSoftMaxBytes) return candidate;
      if (smallest == null || byteLength < smallest.byteLength) {
        smallest = candidate;
      }
    }
    return smallest;
  }
}
