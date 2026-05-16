import 'dart:io';

import 'package:flutter/material.dart';

import '../supabase/app_update_service.dart';

class AppUpdateGate extends StatefulWidget {
  const AppUpdateGate({
    super.key,
    required this.child,
    AppUpdateService? updateService,
  }) : _updateService = updateService;

  final Widget child;
  final AppUpdateService? _updateService;

  @override
  State<AppUpdateGate> createState() => _AppUpdateGateState();
}

class _AppUpdateGateState extends State<AppUpdateGate> {
  late final AppUpdateService _updateService =
      widget._updateService ?? AppUpdateService();

  bool _checked = false;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _checkForUpdate();
    });
  }

  Future<void> _checkForUpdate() async {
    if (_checked || !Platform.isAndroid) return;
    _checked = true;
    await showAppUpdateCheckDialog(
      context: context,
      updateService: _updateService,
      showNoUpdateMessage: false,
    );
  }

  @override
  Widget build(BuildContext context) {
    return widget.child;
  }
}

Future<void> showAppUpdateCheckDialog({
  required BuildContext context,
  AppUpdateService? updateService,
  bool showNoUpdateMessage = true,
}) async {
  final messenger = ScaffoldMessenger.maybeOf(context);
  if (!Platform.isAndroid) {
    if (showNoUpdateMessage) {
      messenger?.showSnackBar(
        const SnackBar(
          content: Text('Update checks are only available on Android.'),
        ),
      );
    }
    return;
  }

  final service = updateService ?? AppUpdateService();
  try {
    final result = await service.checkLatest();
    if (!context.mounted) return;
    if (!result.hasUpdate || result.release == null) {
      if (showNoUpdateMessage) {
        messenger?.showSnackBar(
          const SnackBar(
            content: Text('You are already on the latest version.'),
          ),
        );
      }
      return;
    }
    await _showUpdateDialog(context, service, result);
  } catch (error) {
    if (!context.mounted || !showNoUpdateMessage) return;
    messenger?.showSnackBar(
      SnackBar(content: Text('Update check failed: $error')),
    );
  }
}

Future<void> _showUpdateDialog(
  BuildContext context,
  AppUpdateService updateService,
  AppUpdateCheckResult result,
) async {
  final release = result.release!;
  var installBusy = false;
  double? downloadProgress;
  String? statusText;

  await showDialog<void>(
    context: context,
    barrierDismissible: !result.forceUpdate,
    builder: (context) => StatefulBuilder(
      builder: (context, setDialogState) {
        Future<void> install() async {
          setDialogState(() {
            installBusy = true;
            statusText = 'Downloading update package...';
            downloadProgress = 0;
          });
          try {
            final download = await updateService.downloadApk(
              release,
              onProgress: (received, total) {
                if (!context.mounted) return;
                setDialogState(() {
                  downloadProgress = total == null || total <= 0
                      ? null
                      : received / total;
                });
              },
            );
            setDialogState(() {
              statusText = 'Download complete. Opening installer...';
              downloadProgress = 1;
            });
            await updateService.requestInstall(download.file);
            if (!context.mounted || result.forceUpdate) return;
            Navigator.of(context).pop();
          } catch (error) {
            if (!context.mounted) return;
            setDialogState(() {
              statusText = 'Update failed: $error';
              downloadProgress = null;
              installBusy = false;
            });
          }
        }

        return AlertDialog(
          icon: const Icon(Icons.system_update_alt),
          title: Text('New version ${release.versionName}'),
          content: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              if (release.releaseNotes.isNotEmpty) Text(release.releaseNotes),
              if (release.releaseNotes.isNotEmpty) const SizedBox(height: 16),
              Text(
                result.forceUpdate
                    ? 'This version must be updated before continuing.'
                    : 'Download and install the new APK now.',
              ),
              if (statusText != null) ...[
                const SizedBox(height: 16),
                LinearProgressIndicator(value: downloadProgress),
                const SizedBox(height: 8),
                Text(statusText!),
              ],
            ],
          ),
          actions: [
            if (!result.forceUpdate)
              TextButton(
                onPressed: !installBusy
                    ? () => Navigator.of(context).pop()
                    : null,
                child: const Text('Later'),
              ),
            FilledButton.icon(
              onPressed: !installBusy ? install : null,
              icon: const Icon(Icons.download),
              label: const Text('Update'),
            ),
          ],
        );
      },
    ),
  );
}
