import 'package:flutter/material.dart';

class CrocodileFrameAnimation extends StatefulWidget {
  const CrocodileFrameAnimation({
    super.key,
    required this.frames,
    this.frameDuration = const Duration(milliseconds: 120),
    this.width,
    this.height,
    this.fit = BoxFit.contain,
    this.semanticLabel,
  });

  final List<String> frames;
  final Duration frameDuration;
  final double? width;
  final double? height;
  final BoxFit fit;
  final String? semanticLabel;

  static const rollFrames = <String>[
    'assets/crocodile/roll/crocodile_frame_01.png',
    'assets/crocodile/roll/crocodile_frame_02.png',
    'assets/crocodile/roll/crocodile_frame_03.png',
    'assets/crocodile/roll/crocodile_frame_04.png',
    'assets/crocodile/roll/crocodile_frame_05.png',
    'assets/crocodile/roll/crocodile_frame_06.png',
    'assets/crocodile/roll/crocodile_frame_07.png',
    'assets/crocodile/roll/crocodile_frame_08.png',
  ];

  static const crawlFrames = <String>[
    'assets/crocodile/crawl/crocodile_crawl_frame_01.png',
    'assets/crocodile/crawl/crocodile_crawl_frame_02.png',
    'assets/crocodile/crawl/crocodile_crawl_frame_03.png',
    'assets/crocodile/crawl/crocodile_crawl_frame_04.png',
    'assets/crocodile/crawl/crocodile_crawl_frame_05.png',
    'assets/crocodile/crawl/crocodile_crawl_frame_06.png',
  ];

  @override
  State<CrocodileFrameAnimation> createState() =>
      _CrocodileFrameAnimationState();
}

class _CrocodileFrameAnimationState extends State<CrocodileFrameAnimation>
    with SingleTickerProviderStateMixin {
  late AnimationController _controller;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(
      vsync: this,
      duration: _durationFor(widget.frames, widget.frameDuration),
    )..repeat();
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    for (final frame in widget.frames) {
      precacheImage(AssetImage(frame), context);
    }
  }

  @override
  void didUpdateWidget(covariant CrocodileFrameAnimation oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.frames.length != widget.frames.length ||
        oldWidget.frameDuration != widget.frameDuration) {
      _controller.duration = _durationFor(widget.frames, widget.frameDuration);
      _controller.repeat();
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    if (widget.frames.isEmpty) return const SizedBox.shrink();

    return Semantics(
      image: true,
      label: widget.semanticLabel,
      child: AnimatedBuilder(
        animation: _controller,
        builder: (context, _) {
          final index = (_controller.value * widget.frames.length).floor() %
              widget.frames.length;
          return Image.asset(
            widget.frames[index],
            width: widget.width,
            height: widget.height,
            fit: widget.fit,
            gaplessPlayback: true,
            filterQuality: FilterQuality.medium,
          );
        },
      ),
    );
  }
}

Duration _durationFor(List<String> frames, Duration frameDuration) {
  final frameCount = frames.isEmpty ? 1 : frames.length;
  return Duration(milliseconds: frameDuration.inMilliseconds * frameCount);
}

class CrocodileLoadingAnimation extends StatelessWidget {
  const CrocodileLoadingAnimation({
    super.key,
    this.label = 'Loading...',
    this.width = 172,
    this.height = 132,
  });

  final String label;
  final double width;
  final double height;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          CrocodileFrameAnimation(
            frames: CrocodileFrameAnimation.rollFrames,
            width: width,
            height: height,
            frameDuration: const Duration(milliseconds: 130),
            semanticLabel: label,
          ),
          const SizedBox(height: 12),
          Text(label),
        ],
      ),
    );
  }
}

class CrocodileRefreshContainer extends StatelessWidget {
  const CrocodileRefreshContainer({
    super.key,
    required this.refreshing,
    required this.child,
    this.label = '加载中',
  });

  final bool refreshing;
  final Widget child;
  final String label;

  @override
  Widget build(BuildContext context) {
    return Stack(
      children: [
        child,
        Positioned(
          top: 12,
          left: 0,
          right: 0,
          child: IgnorePointer(
            child: AnimatedOpacity(
              opacity: refreshing ? 1 : 0,
              duration: const Duration(milliseconds: 140),
              child: Center(child: CrocodileRefreshPopup(label: label)),
            ),
          ),
        ),
      ],
    );
  }
}

class CrocodileRefreshPopup extends StatelessWidget {
  const CrocodileRefreshPopup({super.key, this.label = '加载中'});

  final String label;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Material(
      color: Colors.transparent,
      child: Container(
        padding: const EdgeInsets.fromLTRB(14, 8, 16, 8),
        decoration: BoxDecoration(
          color: colorScheme.surface.withValues(alpha: 0.94),
          borderRadius: BorderRadius.circular(24),
          border: Border.all(
            color: colorScheme.primary.withValues(alpha: 0.10),
          ),
          boxShadow: [
            BoxShadow(
              color: Colors.black.withValues(alpha: 0.08),
              blurRadius: 18,
              offset: const Offset(0, 8),
            ),
          ],
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            const CrocodileFrameAnimation(
              frames: CrocodileFrameAnimation.crawlFrames,
              width: 92,
              height: 46,
              frameDuration: Duration(milliseconds: 115),
              semanticLabel: 'Refreshing',
            ),
            const SizedBox(width: 8),
            Text(
              label,
              style: Theme.of(context).textTheme.labelLarge?.copyWith(
                color: colorScheme.onSurface,
                fontWeight: FontWeight.w700,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class CrocodileRefreshIndicator extends StatefulWidget {
  const CrocodileRefreshIndicator({
    super.key,
    required this.onRefresh,
    required this.child,
    this.label = '加载中',
  });

  final Future<void> Function() onRefresh;
  final Widget child;
  final String label;

  @override
  State<CrocodileRefreshIndicator> createState() =>
      _CrocodileRefreshIndicatorState();
}

class _CrocodileRefreshIndicatorState extends State<CrocodileRefreshIndicator> {
  bool _refreshing = false;
  bool _dragCanRefresh = true;

  Future<void> _refresh() async {
    if (!_dragCanRefresh) return;
    if (mounted) {
      setState(() {
        _refreshing = true;
      });
    }
    try {
      await widget.onRefresh();
    } finally {
      _dragCanRefresh = true;
      if (mounted) {
        setState(() {
          _refreshing = false;
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return CrocodileRefreshContainer(
      refreshing: _refreshing,
      label: widget.label,
      child: NotificationListener<ScrollNotification>(
        onNotification: (notification) {
          if (notification.depth != 0) return false;
          if (notification is ScrollStartNotification &&
              notification.dragDetails != null) {
            _dragCanRefresh = notification.metrics.extentBefore <= 24.0;
          } else if (notification is OverscrollNotification &&
              notification.dragDetails != null &&
              notification.metrics.extentBefore <= 0.0 &&
              notification.overscroll < 0) {
            _dragCanRefresh = true;
          } else if (notification is ScrollEndNotification) {
            if (!_refreshing) _dragCanRefresh = true;
          }
          return false;
        },
        child: RefreshIndicator(
          onRefresh: _refresh,
          color: Colors.transparent,
          backgroundColor: Colors.transparent,
          elevation: 0,
          displacement: 96,
          edgeOffset: 12,
          notificationPredicate: (notification) =>
              notification.depth == 0 && _dragCanRefresh,
          strokeWidth: 0.01,
          child: widget.child,
        ),
      ),
    );
  }
}
