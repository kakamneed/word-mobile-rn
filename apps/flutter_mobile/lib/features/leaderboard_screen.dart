import 'dart:io';

import 'package:flutter/material.dart';

import '../sdk/sdk.dart';
import '../supabase/leaderboard_service.dart' as cloud;
import '../supabase/reward_image_vote_service.dart' as cloud_images;
import '../widgets/crocodile_frame_animation.dart';

class LeaderboardScreen extends StatefulWidget {
  const LeaderboardScreen({
    super.key,
    required this.sdk,
  });

  final WordSdk sdk;

  @override
  State<LeaderboardScreen> createState() => _LeaderboardScreenState();
}

@visibleForTesting
const leaderboardImageVotesKey = Key('leaderboard-image-votes');

@visibleForTesting
const leaderboardVoteButtonKey = Key('leaderboard-vote-button');

enum LeaderboardPeriod {
  weekly('weekly', '\u5468\u699c'),
  monthly('monthly', '\u6708\u699c'),
  allTime('all_time', '\u603b\u699c');

  const LeaderboardPeriod(this.wireName, this.label);

  final String wireName;
  final String label;
}

enum LeaderboardMetric {
  totalQuestions('totalQuestions', '\u9898\u6570\u699c'),
  accuracy('accuracy', '\u6b63\u786e\u7387\u699c'),
  mixedAccuracy('mixedAccuracy', '\u6df7\u5408\u6d4b\u8bd5\u6b63\u786e\u7387\u699c'),
  currentStreak('currentStreak', '\u8fde\u7eed\u699c');

  const LeaderboardMetric(this.wireName, this.label);

  final String wireName;
  final String label;

  bool get usesPeriod => this != LeaderboardMetric.currentStreak;
}

class _LeaderboardScreenState extends State<LeaderboardScreen> {
  LeaderboardMetric _metric = LeaderboardMetric.totalQuestions;
  LeaderboardPeriod _period = LeaderboardPeriod.weekly;
  bool _showImageVotes = false;
  bool _loading = true;
  bool _refreshingSummary = false;
  String? _message;
  List<LocalLeaderboardEntry> _entries = const [];
  List<RewardImage> _rewardImages = const [];
  List<cloud_images.CloudRewardImageEntry> _cloudRewardImages = const [];
  final Set<int> _localVotedImageIds = <int>{};
  final Set<String> _cloudVotedImageIds = <String>{};
  bool _usingCloudImageVotes = false;

  @override
  void initState() {
    super.initState();
    _load(refreshSummary: true);
  }

  Future<void> _load({
    bool refreshSummary = false,
    bool showFullLoading = true,
  }) async {
    setState(() {
      if (showFullLoading) _loading = true;
      _message = null;
    });
    try {
      if (_showImageVotes) {
        await _loadLocalImageVotes();
        return;
      }
      if (await _loadCloudLeaderboard(refreshSummary: refreshSummary)) {
        return;
      }
      if (refreshSummary) {
        await _refreshMyLocalSummary();
      }
      final queryPeriod =
          _metric.usesPeriod ? _period : LeaderboardPeriod.allTime;
      final entries = await widget.sdk.rewardImages.getLocalLeaderboard(
        metric: _metric.wireName,
        period: queryPeriod.wireName,
        periodStart: _periodStart(DateTime.now(), queryPeriod),
      );
      setState(() {
        _entries = entries;
        _rewardImages = const [];
        _cloudRewardImages = const [];
        _usingCloudImageVotes = false;
        _message = entries.isEmpty
            ? '\u672c\u5730\u6392\u884c\u699c\u6682\u65e0\u6570\u636e\u3002'
            : null;
      });
    } catch (error) {
      setState(() {
        _entries = const [];
        _message = '\u6392\u884c\u699c\u52a0\u8f7d\u5931\u8d25\uff1a$error';
      });
    } finally {
      if (mounted) {
        setState(() {
          _loading = false;
        });
      }
    }
  }

  Future<bool> _loadCloudLeaderboard({required bool refreshSummary}) async {
    final service = cloud.LeaderboardService();
    if (!service.isConfigured || !await service.isSignedIn) return false;
    try {
      final reports = refreshSummary
          ? await widget.sdk.reports.getReportsOverview()
          : null;
      final cloudMetric = cloud.LeaderboardMetric.values.byName(_metric.name);
      final cloudPeriod = cloud.LeaderboardPeriod.values.byName(_period.name);
      if (reports != null) {
        await service.refreshFromReports(
          reports,
          metric: cloudMetric,
          period: cloudPeriod,
        );
      }
      final entries = await service.fetchLeaderboard(
        metric: cloudMetric,
        period: cloudPeriod,
      );
      setState(() {
        _entries = entries.map(_cloudEntryToLocalShape).toList(growable: false);
        _rewardImages = const [];
        _message = entries.isEmpty
            ? '\u4e91\u7aef\u6392\u884c\u699c\u6682\u65e0\u6570\u636e\u3002'
            : null;
      });
      return true;
    } catch (_) {
      return false;
    }
  }

  LocalLeaderboardEntry _cloudEntryToLocalShape(cloud.LeaderboardEntry entry) {
    return LocalLeaderboardEntry(
      rank: entry.rank,
      isCurrentUser: entry.isCurrentUser,
      userId: entry.userId,
      displayName: entry.displayName,
      totalQuestions: entry.totalQuestions,
      correctCount: entry.correctCount,
      accuracyPercent: entry.accuracyPercent,
      mixedTestTotalQuestions: entry.mixedTestTotalQuestions,
      mixedTestCorrectCount: entry.mixedTestCorrectCount,
      mixedTestAccuracyPercent: entry.mixedTestAccuracyPercent,
      currentStreakDays: entry.currentStreakDays,
      updatedAt: entry.updatedAt?.toIso8601String() ?? '',
      avatarUrl: entry.avatarUrl,
    );
  }

  Future<void> _loadLocalImageVotes() async {
    if (await _loadCloudImageVotes()) return;
    final images = await widget.sdk.rewardImages.listImages(publicOnly: true);
    setState(() {
      _rewardImages = [...images]
        ..sort((a, b) => b.voteCount.compareTo(a.voteCount));
      _cloudRewardImages = const [];
      _usingCloudImageVotes = false;
      _entries = const [];
      _message = images.isEmpty
          ? '\u6682\u65e0\u5df2\u5ba1\u6838\u901a\u8fc7\u7684\u672c\u5730\u56fe\u7247\u3002'
          : null;
    });
  }

  Future<bool> _loadCloudImageVotes() async {
    final service = cloud_images.RewardImageVoteService();
    if (!service.isConfigured) return false;
    try {
      final images = await service.fetchVoteLeaderboard(
        weekStart: _periodStart(DateTime.now(), LeaderboardPeriod.weekly),
      );
      setState(() {
        _cloudRewardImages = images
            .map(
              (image) => image.copyWith(
                votedByMe:
                    image.votedByMe || _cloudVotedImageIds.contains(image.imageId),
              ),
            )
            .toList(growable: false);
        _rewardImages = const [];
        _usingCloudImageVotes = true;
        _entries = const [];
        _message = images.isEmpty
            ? '\u4e91\u7aef\u56fe\u7247\u7968\u9009\u699c\u6682\u65e0\u6570\u636e\u3002'
            : null;
      });
      return true;
    } catch (_) {
      return false;
    }
  }

  Future<void> _voteForImage(RewardImage image) async {
    try {
      await widget.sdk.rewardImages.vote(imageId: image.id);
      if (mounted) {
        setState(() {
          _localVotedImageIds.add(image.id);
        });
      }
      await _load(refreshSummary: false, showFullLoading: false);
    } catch (error) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('\u6295\u7968\u5931\u8d25\uff1a$error')),
      );
    }
  }

  Future<void> _voteForCloudImage(
    cloud_images.CloudRewardImageEntry image,
  ) async {
    try {
      final service = cloud_images.RewardImageVoteService();
      final voteResult = await service.vote(
        imageId: image.imageId,
        weekStart: _periodStart(DateTime.now(), LeaderboardPeriod.weekly),
      );
      if (mounted) {
        setState(() {
          _cloudVotedImageIds.add(image.imageId);
          _cloudRewardImages = _cloudRewardImages
              .map(
                (entry) => entry.imageId == image.imageId
                    ? entry.copyWith(
                        voteCount: voteResult.voteCount,
                        votedByMe: true,
                      )
                    : entry,
              )
              .toList(growable: false);
        });
      }
      await _load(refreshSummary: false, showFullLoading: false);
    } catch (error) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('\u6295\u7968\u5931\u8d25\uff1a$error')),
      );
    }
  }

  Future<void> _refreshMyLocalSummary() async {
    setState(() {
      _refreshingSummary = true;
    });
    try {
      final reports = await widget.sdk.reports.getReportsOverview();
      final currentStreak =
          (reports.streakInfo['currentStreak'] as num?)?.toInt() ?? 0;
      for (final period in LeaderboardPeriod.values) {
        await _refreshOneLocalSummary(reports, currentStreak, period);
      }
    } finally {
      if (mounted) {
        setState(() {
          _refreshingSummary = false;
        });
      }
    }
  }

  Future<void> _refreshOneLocalSummary(
    ReportsOverview reports,
    int currentStreak,
    LeaderboardPeriod period,
  ) async {
    final periodStart = _periodStart(DateTime.now(), period);
    final totalQuestions = reports.totalQuestionsAnswered;
    final correctCount = ((reports.overallAccuracy / 100.0) * totalQuestions)
        .round()
        .clamp(0, totalQuestions);
    final mixed = _modeTotal(reports, 'mixedTest');
    await widget.sdk.rewardImages.refreshLocalSummary(
      userKey: 'local',
      displayName: 'Local learner',
      metricPeriod: period.wireName,
      periodStart: periodStart,
      totalQuestions: totalQuestions,
      correctCount: correctCount,
      mixedTestTotalQuestions: mixed.$1,
      mixedTestCorrectCount: mixed.$2,
      currentStreakDays: currentStreak,
    );
  }

  (int, int) _modeTotal(ReportsOverview reports, String mode) {
    for (final row in reports.modeBreakdown.whereType<Map>()) {
      if ('${row['mode'] ?? ''}' == mode) {
        return (
          (row['totalQuestions'] as num?)?.toInt() ?? 0,
          (row['correctCount'] as num?)?.toInt() ?? 0,
        );
      }
    }
    return (0, 0);
  }

  String _periodStart(DateTime now, LeaderboardPeriod period) {
    final local = DateTime(now.year, now.month, now.day);
    final start = switch (period) {
      LeaderboardPeriod.weekly =>
        local.subtract(Duration(days: local.weekday - DateTime.monday)),
      LeaderboardPeriod.monthly => DateTime(local.year, local.month),
      LeaderboardPeriod.allTime => DateTime(1970),
    };
    return start.toIso8601String().split('T').first;
  }

  Future<void> _seedLocalDemo() async {
    await widget.sdk.rewardImages.seedLocalDemo();
    await _load(refreshSummary: false, showFullLoading: false);
  }

  @override
  Widget build(BuildContext context) {
    final showPeriodSelector = !_showImageVotes && _metric.usesPeriod;
    return Scaffold(
      appBar: AppBar(
        title: const Text('\u6392\u884c\u699c'),
        actions: [
          IconButton(
            tooltip: '\u751f\u6210\u672c\u5730\u6f14\u793a\u6570\u636e',
            onPressed: _loading ? null : _seedLocalDemo,
            icon: const Icon(Icons.science_outlined),
          ),
          IconButton(
            tooltip: '\u5237\u65b0',
            onPressed: _loading ? null : () => _load(refreshSummary: true),
            icon: const Icon(Icons.refresh),
          ),
        ],
      ),
      body: Column(
        children: [
          _MetricSelector(
            selected: _metric,
            showImageVotes: _showImageVotes,
            onSelected: (metric) {
              setState(() {
                _metric = metric;
                _showImageVotes = false;
              });
              _load(refreshSummary: true);
            },
            onImageVotesSelected: () {
              setState(() {
                _showImageVotes = true;
              });
              _load(refreshSummary: false);
            },
          ),
          if (showPeriodSelector)
            _PeriodSelector(
              selected: _period,
              onSelected: (period) {
                setState(() {
                  _period = period;
                });
                _load(refreshSummary: true);
              },
            ),
          if (!_showImageVotes)
            _LeaderboardScopeLabel(metric: _metric, period: _period),
          if (_refreshingSummary) const LinearProgressIndicator(),
          Expanded(
            child: _loading
                ? const CrocodileLoadingAnimation(
                    label: '\u52a0\u8f7d\u4e2d...',
                  )
                : _showImageVotes
                    ? _LocalImageVoteList(
                        images: _rewardImages,
                        cloudImages: _cloudRewardImages,
                        usingCloud: _usingCloudImageVotes,
                        message: _message,
                        localVotedImageIds: _localVotedImageIds,
                        onVote: _voteForImage,
                        onCloudVote: _voteForCloudImage,
                        onRefresh: _loadLocalImageVotes,
                      )
                    : _entries.isEmpty
                        ? _EmptyLeaderboard(message: _message)
                        : CrocodileRefreshIndicator(
                            onRefresh: () => _load(
                              refreshSummary: true,
                              showFullLoading: false,
                            ),
                            child: ListView.separated(
                              padding: const EdgeInsets.all(16),
                              itemBuilder: (context, index) {
                                final entry = _entries[index];
                                return _LeaderboardTile(
                                  entry: entry,
                                  metric: _metric,
                                );
                              },
                              separatorBuilder: (context, index) =>
                                  const SizedBox(height: 8),
                              itemCount: _entries.length,
                            ),
                          ),
          ),
        ],
      ),
    );
  }
}

class _LeaderboardTile extends StatelessWidget {
  const _LeaderboardTile({required this.entry, required this.metric});

  final LocalLeaderboardEntry entry;
  final LeaderboardMetric metric;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final highlight = entry.isCurrentUser;
    final imageProvider = _avatarImageProvider(entry);
    return Card(
      color: highlight ? colorScheme.primaryContainer : null,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
        child: Row(
          children: [
            SizedBox(
              width: 66,
              height: 66,
              child: Stack(
                clipBehavior: Clip.none,
                children: [
                  Positioned.fill(
                    child: CircleAvatar(
                      radius: 30,
                      foregroundImage: imageProvider,
                      backgroundColor: highlight
                          ? colorScheme.primary.withValues(alpha: 0.18)
                          : colorScheme.surfaceContainerHighest,
                      foregroundColor: colorScheme.onSurfaceVariant,
                      child: imageProvider == null
                          ? Text(
                              _avatarInitial(entry.displayName),
                              style: Theme.of(context)
                                  .textTheme
                                  .titleLarge
                                  ?.copyWith(
                                    color: highlight
                                        ? colorScheme.primary
                                        : colorScheme.onSurfaceVariant,
                                    fontWeight: FontWeight.w700,
                                  ),
                            )
                          : null,
                    ),
                  ),
                  Positioned(
                    left: -2,
                    bottom: -2,
                    child: Container(
                      constraints: const BoxConstraints(
                        minWidth: 24,
                        minHeight: 24,
                      ),
                      padding: const EdgeInsets.symmetric(horizontal: 6),
                      alignment: Alignment.center,
                      decoration: BoxDecoration(
                        color: highlight
                            ? colorScheme.primary
                            : colorScheme.surface,
                        borderRadius: BorderRadius.circular(999),
                        border: Border.all(
                          color: highlight
                              ? colorScheme.primaryContainer
                              : colorScheme.outlineVariant,
                          width: 1.5,
                        ),
                      ),
                      child: Text(
                        '${entry.rank}',
                        style: Theme.of(context).textTheme.labelSmall?.copyWith(
                              color: highlight
                                  ? colorScheme.onPrimary
                                  : colorScheme.onSurfaceVariant,
                              fontWeight: FontWeight.w700,
                            ),
                      ),
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(width: 14),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    entry.displayName,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                  const SizedBox(height: 4),
                  Text(
                    _summaryText(entry),
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                  ),
                ],
              ),
            ),
            const SizedBox(width: 12),
            Text(
              _metricValue(entry, metric),
              style: Theme.of(context).textTheme.titleMedium,
            ),
          ],
        ),
      ),
    );
  }

  String _avatarInitial(String displayName) {
    final trimmed = displayName.trim();
    if (trimmed.isEmpty) return '?';
    return trimmed.characters.first.toUpperCase();
  }

  ImageProvider? _avatarImageProvider(LocalLeaderboardEntry entry) {
    final avatarUrl = entry.avatarUrl?.trim();
    if (avatarUrl != null && avatarUrl.isNotEmpty) {
      return NetworkImage(avatarUrl);
    }
    return _tagImageProvider(entry.tagImage);
  }

  ImageProvider? _tagImageProvider(RewardImage? image) {
    if (image == null) return null;
    final file = File(image.localPath);
    if (!file.existsSync()) return null;
    return FileImage(file);
  }

  String _summaryText(LocalLeaderboardEntry entry) {
    return '\u9898\u6570 ${entry.totalQuestions} | \u6b63\u786e\u7387 ${entry.accuracyPercent.toStringAsFixed(1)}% | \u6df7\u6d4b ${entry.mixedTestAccuracyPercent.toStringAsFixed(1)}% | \u8fde\u7eed ${entry.currentStreakDays}\u5929';
  }

  String _metricValue(LocalLeaderboardEntry entry, LeaderboardMetric metric) {
    return switch (metric) {
      LeaderboardMetric.totalQuestions => '${entry.totalQuestions}',
      LeaderboardMetric.accuracy =>
        '${entry.accuracyPercent.toStringAsFixed(1)}%',
      LeaderboardMetric.mixedAccuracy =>
        '${entry.mixedTestAccuracyPercent.toStringAsFixed(1)}%',
      LeaderboardMetric.currentStreak =>
        '${entry.currentStreakDays}\u5929',
    };
  }
}

class _MetricSelector extends StatelessWidget {
  const _MetricSelector({
    required this.selected,
    required this.showImageVotes,
    required this.onSelected,
    required this.onImageVotesSelected,
  });

  final LeaderboardMetric selected;
  final bool showImageVotes;
  final ValueChanged<LeaderboardMetric> onSelected;
  final VoidCallback onImageVotesSelected;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      height: 56,
      child: ListView.separated(
        scrollDirection: Axis.horizontal,
        padding: const EdgeInsets.fromLTRB(16, 10, 16, 8),
        itemBuilder: (context, index) {
          if (index == LeaderboardMetric.values.length) {
            return ChoiceChip(
              key: leaderboardImageVotesKey,
              label: const Text(
                '\u56fe\u7247\u7968\u9009\u699c',
                style: TextStyle(fontSize: 13),
              ),
              selected: showImageVotes,
              onSelected: (_) => onImageVotesSelected(),
              avatar:
                  showImageVotes ? const Icon(Icons.check, size: 16) : null,
              visualDensity: VisualDensity.compact,
            );
          }
          final metric = LeaderboardMetric.values[index];
          return ChoiceChip(
            label: Text(
              metric.label,
              style: const TextStyle(fontSize: 13),
            ),
            selected: !showImageVotes && selected == metric,
            onSelected: (_) => onSelected(metric),
            avatar: !showImageVotes && selected == metric
                ? const Icon(Icons.check, size: 16)
                : null,
            visualDensity: VisualDensity.compact,
          );
        },
        separatorBuilder: (context, index) => const SizedBox(width: 8),
        itemCount: LeaderboardMetric.values.length + 1,
      ),
    );
  }
}

class _LocalImageVoteList extends StatelessWidget {
  const _LocalImageVoteList({
    required this.images,
    required this.cloudImages,
    required this.usingCloud,
    required this.message,
    required this.localVotedImageIds,
    required this.onVote,
    required this.onCloudVote,
    required this.onRefresh,
  });

  final List<RewardImage> images;
  final List<cloud_images.CloudRewardImageEntry> cloudImages;
  final bool usingCloud;
  final String? message;
  final Set<int> localVotedImageIds;
  final ValueChanged<RewardImage> onVote;
  final ValueChanged<cloud_images.CloudRewardImageEntry> onCloudVote;
  final Future<void> Function() onRefresh;

  @override
  Widget build(BuildContext context) {
    if (usingCloud) {
      if (cloudImages.isEmpty) {
        return _EmptyLeaderboard(message: message);
      }
      return CrocodileRefreshIndicator(
        onRefresh: onRefresh,
        child: ListView.separated(
          padding: const EdgeInsets.all(16),
          itemBuilder: (context, index) {
            final image = cloudImages[index];
            return _CloudImageVoteTile(
              image: image,
              onVote: image.votedByMe ? null : () => onCloudVote(image),
            );
          },
          separatorBuilder: (context, index) => const SizedBox(height: 8),
          itemCount: cloudImages.length,
        ),
      );
    }
    if (images.isEmpty) {
      return _EmptyLeaderboard(message: message);
    }
    return CrocodileRefreshIndicator(
      onRefresh: onRefresh,
      child: ListView.separated(
        padding: const EdgeInsets.all(16),
        itemBuilder: (context, index) {
          final image = images[index];
          return _LocalImageVoteTile(
            rank: index + 1,
            image: image,
            onVote: localVotedImageIds.contains(image.id)
                ? null
                : () => onVote(image),
          );
        },
        separatorBuilder: (context, index) => const SizedBox(height: 8),
        itemCount: images.length,
      ),
    );
  }
}

class _CloudImageVoteTile extends StatelessWidget {
  const _CloudImageVoteTile({
    required this.image,
    required this.onVote,
  });

  final cloud_images.CloudRewardImageEntry image;
  final VoidCallback? onVote;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final imageUrl = image.publicUrl;
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Row(
          children: [
            CircleAvatar(child: Text('${image.rank}')),
            const SizedBox(width: 12),
            ClipRRect(
              borderRadius: BorderRadius.circular(10),
              child: imageUrl.isNotEmpty
                  ? Image.network(
                      imageUrl,
                      width: 72,
                      height: 72,
                      fit: BoxFit.cover,
                      errorBuilder: (context, error, stackTrace) => Container(
                        width: 72,
                        height: 72,
                        color: colorScheme.surfaceContainerHighest,
                        child: const Icon(Icons.broken_image_outlined),
                      ),
                    )
                  : Container(
                      width: 72,
                      height: 72,
                      color: colorScheme.surfaceContainerHighest,
                      child: const Icon(Icons.broken_image_outlined),
                    ),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    image.displayName,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                  const SizedBox(height: 4),
                  Text('\u672c\u5468 ${image.voteCount} \u7968'),
                ],
              ),
            ),
            FilledButton.icon(
              key: leaderboardVoteButtonKey,
              onPressed: onVote,
              icon: Icon(
                image.votedByMe
                    ? Icons.check_circle_outline
                    : Icons.how_to_vote_outlined,
              ),
              label: Text(image.votedByMe ? '\u5df2\u6295\u7968' : '\u6295\u7968'),
            ),
          ],
        ),
      ),
    );
  }
}

class _LocalImageVoteTile extends StatelessWidget {
  const _LocalImageVoteTile({
    required this.rank,
    required this.image,
    required this.onVote,
  });

  final int rank;
  final RewardImage image;
  final VoidCallback? onVote;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final file = File(image.localPath);
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Row(
          children: [
            CircleAvatar(child: Text('$rank')),
            const SizedBox(width: 12),
            ClipRRect(
              borderRadius: BorderRadius.circular(10),
              child: file.existsSync()
                  ? Image.file(file, width: 72, height: 72, fit: BoxFit.cover)
                  : Container(
                      width: 72,
                      height: 72,
                      color: colorScheme.surfaceContainerHighest,
                      child: const Icon(Icons.broken_image_outlined),
                    ),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    image.originalFilename.isEmpty
                        ? '\u672c\u5730\u56fe\u7247'
                        : image.originalFilename,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                  const SizedBox(height: 4),
                  Text('\u672c\u5468 ${image.voteCount} \u7968'),
                ],
              ),
            ),
            FilledButton.icon(
              key: leaderboardVoteButtonKey,
              onPressed: onVote,
              icon: Icon(
                onVote == null
                    ? Icons.check_circle_outline
                    : Icons.how_to_vote_outlined,
              ),
              label: Text(onVote == null ? '\u5df2\u6295\u7968' : '\u6295\u7968'),
            ),
          ],
        ),
      ),
    );
  }
}

class _PeriodSelector extends StatelessWidget {
  const _PeriodSelector({required this.selected, required this.onSelected});

  final LeaderboardPeriod selected;
  final ValueChanged<LeaderboardPeriod> onSelected;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      height: 50,
      child: ListView.separated(
        scrollDirection: Axis.horizontal,
        padding: const EdgeInsets.fromLTRB(16, 6, 16, 8),
        itemBuilder: (context, index) {
          final period = LeaderboardPeriod.values[index];
          return ChoiceChip(
            label: Text(
              period.label,
              style: const TextStyle(fontSize: 13),
            ),
            selected: selected == period,
            onSelected: (_) => onSelected(period),
            visualDensity: VisualDensity.compact,
          );
        },
        separatorBuilder: (context, index) => const SizedBox(width: 8),
        itemCount: LeaderboardPeriod.values.length,
      ),
    );
  }
}

class _LeaderboardScopeLabel extends StatelessWidget {
  const _LeaderboardScopeLabel({required this.metric, required this.period});

  final LeaderboardMetric metric;
  final LeaderboardPeriod period;

  @override
  Widget build(BuildContext context) {
    final text = metric.usesPeriod
        ? '${period.label} \u00b7 ${_periodRangeLabel(period)}'
        : '\u8fde\u7eed\u699c \u00b7 \u6bcf\u65e5\u66f4\u65b0';
    return Padding(
      padding: const EdgeInsets.fromLTRB(20, 0, 20, 4),
      child: Align(
        alignment: Alignment.centerLeft,
        child: Text(
          text,
          style: Theme.of(context).textTheme.bodySmall?.copyWith(
                color: Colors.black54,
                fontWeight: FontWeight.w600,
              ),
        ),
      ),
    );
  }

  String _periodRangeLabel(LeaderboardPeriod period) {
    final now = DateTime.now();
    return switch (period) {
      LeaderboardPeriod.weekly =>
        '${now.year}\u5e74\u7b2c${_weekOfYear(now)}\u5468',
      LeaderboardPeriod.monthly => '${now.year}\u5e74${now.month}\u6708',
      LeaderboardPeriod.allTime => '\u7d2f\u8ba1\u603b\u699c',
    };
  }

  int _weekOfYear(DateTime date) {
    final yearStart = DateTime(date.year, 1, 1);
    final days = date.difference(yearStart).inDays;
    return (days / 7).floor() + 1;
  }
}

class _EmptyLeaderboard extends StatelessWidget {
  const _EmptyLeaderboard({required this.message});

  final String? message;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Text(
          message ?? '\u6682\u65e0\u6392\u884c\u699c\u6570\u636e',
          textAlign: TextAlign.center,
          style: Theme.of(context).textTheme.bodyLarge,
        ),
      ),
    );
  }
}
