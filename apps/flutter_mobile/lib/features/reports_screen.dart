import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../sdk/sdk.dart';

class ReportsScreen extends StatefulWidget {
  const ReportsScreen({super.key, required this.sdk});

  final WordSdk sdk;

  @override
  State<ReportsScreen> createState() => _ReportsScreenState();
}

class _ReportsScreenState extends State<ReportsScreen> {
  ReportsOverview? _reports;
  Map<String, dynamic>? _selectedDay;
  String? _selectedMode;
  bool _loading = true;
  String? _error;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final reports = await widget.sdk.reports.getReportsOverview();
      final dailySeries = reports.dailySeries.whereType<Map>().map((e) => e.cast<String, dynamic>()).toList(growable: false);
      final modeBreakdown = reports.modeBreakdown.whereType<Map>().map((e) => e.cast<String, dynamic>()).toList(growable: false);
      if (!mounted) return;
      setState(() {
        _reports = reports;
        _selectedDay = dailySeries.isNotEmpty ? dailySeries.last : null;
        _selectedMode = modeBreakdown.isNotEmpty ? '${modeBreakdown.first['mode'] ?? ''}' : null;
      });
    } catch (error) {
      if (!mounted) return;
      setState(() {
        _error = error.toString();
      });
    } finally {
      if (mounted) {
        setState(() {
          _loading = false;
        });
      }
    }
  }

  String _modeLabel(String mode) {
    return switch (mode) {
      'newWord' => '新词学习',
      'review' => '复习',
      'mixedTest' => '混合测试',
      'wrongWordReinforcement' => '错词强化',
      'rootAffix' => '词根词缀',
      _ => mode,
    };
  }

  Color _modeColor(BuildContext context, String mode) {
    return switch (mode) {
      'newWord' => const Color(0xFF34C759),
      'review' => const Color(0xFF007AFF),
      'mixedTest' => const Color(0xFFFF9500),
      'wrongWordReinforcement' => const Color(0xFFFF3B30),
      'rootAffix' => const Color(0xFF8E44AD),
      _ => Theme.of(context).colorScheme.primary,
    };
  }

  @override
  Widget build(BuildContext context) {
    final reports = _reports;

    return Scaffold(
      appBar: AppBar(title: const Text('报告')),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : _error != null
              ? _ReportsMessage(message: _error!, onRetry: _load)
              : reports == null
                  ? _ReportsMessage(message: '还没有可展示的学习报告。', onRetry: _load)
                  : RefreshIndicator(
                      onRefresh: _load,
                      child: ListView(
                        padding: const EdgeInsets.all(16),
                        children: [
                          _StreakHero(reports: reports),
                          _MetricGrid(reports: reports),
                          _SectionCard(
                            title: '每日正确率趋势',
                            subtitle: '横向滑动浏览不同日期，点选节点查看当天正确率、题量和时长。',
                            child: Column(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: [
                                _DailyLineChart(
                                  data: reports.dailySeries.whereType<Map>().map((e) => e.cast<String, dynamic>()).toList(growable: false),
                                  selectedDate: _selectedDay?['date'] as String?,
                                  lineColor: Theme.of(context).colorScheme.primary,
                                  onSelect: (item) => setState(() => _selectedDay = item),
                                ),
                                const SizedBox(height: 12),
                                _DailyDetailCard(day: _selectedDay),
                              ],
                            ),
                          ),
                          _SectionCard(
                            title: '按模式查看',
                            subtitle: '点开某个模式后，会显示该模式自己的趋势图和建议。',
                            child: Column(
                              children: reports.modeBreakdown
                                  .whereType<Map>()
                                  .map((entry) => entry.cast<String, dynamic>())
                                  .map(
                                    (entry) => Padding(
                                      padding: const EdgeInsets.only(bottom: 12),
                                      child: _ModeBreakdownCard(
                                        entry: entry,
                                        label: _modeLabel('${entry['mode'] ?? ''}'),
                                        color: _modeColor(
                                          context,
                                          '${entry['mode'] ?? ''}',
                                        ),
                                        selected: _selectedMode == '${entry['mode'] ?? ''}',
                                        series: ((reports.modeSeries['${entry['mode'] ?? ''}'] as List?) ?? const [])
                                            .whereType<Map>()
                                            .map((e) => e.cast<String, dynamic>())
                                            .toList(growable: false),
                                        onTap: () {
                                          final mode = '${entry['mode'] ?? ''}';
                                          setState(() {
                                            _selectedMode = _selectedMode == mode ? null : mode;
                                          });
                                        },
                                      ),
                                    ),
                                  )
                                  .toList(growable: false),
                            ),
                          ),
                        ],
                      ),
                    ),
    );
  }
}

class _StreakHero extends StatelessWidget {
  const _StreakHero({required this.reports});

  final ReportsOverview reports;

  @override
  Widget build(BuildContext context) {
    final currentStreak = reports.streakInfo['currentStreak'] ?? 0;
    final longestStreak = reports.streakInfo['longestStreak'] ?? 0;
    return Card(
      margin: const EdgeInsets.only(bottom: 16),
      color: Theme.of(context).colorScheme.primary,
      child: Padding(
        padding: const EdgeInsets.all(20),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              '$currentStreak 天连续学习',
              style: Theme.of(context).textTheme.headlineSmall?.copyWith(
                    color: Colors.white,
                    fontWeight: FontWeight.w700,
                  ),
            ),
            const SizedBox(height: 8),
            Text(
              '最长连续 $longestStreak 天。报告页现在会把每日趋势和模式趋势一起拉平展示。',
              style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                    color: Colors.white.withValues(alpha: 0.88),
                  ),
            ),
          ],
        ),
      ),
    );
  }
}

class _MetricGrid extends StatelessWidget {
  const _MetricGrid({required this.reports});

  final ReportsOverview reports;

  @override
  Widget build(BuildContext context) {
    final items = [
      ('学习天数', '${reports.totalStudyDays}'),
      ('已学词数', '${reports.totalWordsLearned}'),
      ('总答题数', '${reports.totalQuestionsAnswered}'),
      ('整体正确率', '${reports.overallAccuracy.round()}%'),
    ];

    return GridView.builder(
      shrinkWrap: true,
      physics: const NeverScrollableScrollPhysics(),
      itemCount: items.length,
      padding: const EdgeInsets.only(bottom: 16),
      gridDelegate: const SliverGridDelegateWithFixedCrossAxisCount(
        crossAxisCount: 2,
        crossAxisSpacing: 12,
        mainAxisSpacing: 12,
        childAspectRatio: 1.6,
      ),
      itemBuilder: (context, index) {
        final item = items[index];
        final highlight = index == 3;
        final colorScheme = Theme.of(context).colorScheme;
        return Card(
          color: highlight ? colorScheme.primary.withValues(alpha: 0.10) : null,
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                Text(
                  item.$2,
                  style: Theme.of(context).textTheme.headlineSmall?.copyWith(
                        color: highlight ? colorScheme.primary : null,
                        fontWeight: FontWeight.w700,
                      ),
                ),
                const SizedBox(height: 6),
                Text(item.$1, style: Theme.of(context).textTheme.bodyMedium?.copyWith(color: Colors.black54)),
              ],
            ),
          ),
        );
      },
    );
  }
}

class _DailyLineChart extends StatelessWidget {
  const _DailyLineChart({
    required this.data,
    required this.selectedDate,
    required this.lineColor,
    required this.onSelect,
    this.compact = false,
  });

  final List<Map<String, dynamic>> data;
  final String? selectedDate;
  final Color lineColor;
  final ValueChanged<Map<String, dynamic>> onSelect;
  final bool compact;

  @override
  Widget build(BuildContext context) {
    if (data.isEmpty) {
      return const Text('暂无趋势数据');
    }

    const stepX = 56.0;
    const edgeInset = stepX / 2;
    const leftGutter = 34.0;
    const rightGutter = 18.0;
    const topGutter = 12.0;
    const bottomGutter = 22.0;
    final chartHeight = compact ? 104.0 : 140.0;
    final plotHeight = chartHeight - topGutter - bottomGutter;
    final plotWidth = math.max(300.0 - leftGutter - rightGutter, data.length * stepX);
    final chartWidth = leftGutter + plotWidth + rightGutter;

    final points = <_ChartPoint>[];
    for (var index = 0; index < data.length; index++) {
      final item = data[index];
      final accuracy = (((item['accuracyPercent'] as num?) ?? (item['accuracy'] as num?) ?? 0)
              .toDouble())
          .clamp(0.0, 100.0)
          .toDouble();
      final x = leftGutter + edgeInset + index * stepX;
      final y = topGutter + plotHeight - (accuracy / 100) * plotHeight;
      points.add(_ChartPoint(item: item, x: x, y: y, accuracy: accuracy));
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        if (!compact)
          _SelectedDailySummary(item: points.firstWhere((point) => point.item['date'] == selectedDate, orElse: () => points.last).item),
        if (!compact) const SizedBox(height: 12),
        SingleChildScrollView(
          scrollDirection: Axis.horizontal,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              SizedBox(
                height: chartHeight,
                width: chartWidth,
                child: Stack(
                  children: [
                    Positioned.fill(
                      child: CustomPaint(
                        painter: _LineChartPainter(
                          points: points,
                          lineColor: lineColor,
                          topGutter: topGutter,
                          leftGutter: leftGutter,
                          plotHeight: plotHeight,
                          plotWidth: plotWidth,
                        ),
                      ),
                    ),
                    for (final point in points)
                      Positioned(
                        left: point.x - 12,
                        top: point.y - 12,
                        width: 24,
                        height: 24,
                        child: GestureDetector(
                          onTap: () => onSelect(point.item),
                          child: Center(
                            child: Container(
                              width: compact ? 10 : 12,
                              height: compact ? 10 : 12,
                              decoration: BoxDecoration(
                                color: _accuracyColor(point.accuracy),
                                shape: BoxShape.circle,
                                border: selectedDate == point.item['date']
                                    ? Border.all(color: Colors.white, width: 2)
                                    : null,
                              ),
                            ),
                          ),
                        ),
                      ),
                  ],
                ),
              ),
              Row(
                children: [
                  const SizedBox(width: leftGutter),
                  for (final point in points)
                    SizedBox(
                      width: stepX,
                      child: Column(
                        children: [
                          Text(
                            '${point.item['date'] ?? '--'}'.replaceFirst(RegExp(r'^\d{4}-'), ''),
                            style: Theme.of(context).textTheme.bodySmall?.copyWith(color: Colors.black54),
                          ),
                          Text(
                            '${point.accuracy.round()}%',
                            style: Theme.of(context).textTheme.bodySmall?.copyWith(fontWeight: FontWeight.w700),
                          ),
                        ],
                      ),
                    ),
                ],
              ),
            ],
          ),
        ),
      ],
    );
  }
}

class _SelectedDailySummary extends StatelessWidget {
  const _SelectedDailySummary({required this.item});

  final Map<String, dynamic> item;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: const Color(0xFFF8F8F8),
        borderRadius: BorderRadius.circular(10),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text('${item['date'] ?? '--'}', style: Theme.of(context).textTheme.titleSmall),
          const SizedBox(height: 4),
          Text(
            '正确率 ${(((item['accuracyPercent'] as num?) ?? (item['accuracy'] as num?) ?? 0).toDouble()).round()}% · 题量 ${item['totalQuestions'] ?? 0} · 答对 ${item['correctCount'] ?? 0}',
          ),
        ],
      ),
    );
  }
}

class _DailyDetailCard extends StatelessWidget {
  const _DailyDetailCard({required this.day});

  final Map<String, dynamic>? day;

  @override
  Widget build(BuildContext context) {
    final entry = day;
    if (entry == null) {
      return const Text('点击上方图表节点查看当天分析。');
    }

    final totalQuestions = entry['totalQuestions'] ?? entry['questionsAnswered'] ?? 0;
    final accuracy = ((entry['accuracyPercent'] as num?) ?? (entry['accuracy'] as num?) ?? 0).round();
    final studyTimeMs = entry['studyTimeMs'] ?? entry['totalTimeMs'] ?? 0;

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('${entry['date'] ?? '--'} 当日分析', style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 8),
            Text('答题数：$totalQuestions'),
            Text('正确率：$accuracy%'),
            Text('学习时长：${_formatDurationMs(studyTimeMs)}'),
          ],
        ),
      ),
    );
  }
}

class _ModeBreakdownCard extends StatelessWidget {
  const _ModeBreakdownCard({
    required this.entry,
    required this.label,
    required this.color,
    required this.selected,
    required this.series,
    required this.onTap,
  });

  final Map<String, dynamic> entry;
  final String label;
  final Color color;
  final bool selected;
  final List<Map<String, dynamic>> series;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final accuracy = ((entry['accuracyPercent'] as num?) ?? (entry['accuracy'] as num?) ?? 0).round();
    final total = entry['totalQuestions'] ?? entry['questionsAnswered'] ?? 0;
    final correct = entry['correctCount'] ?? 0;
    final missed = (total as num).toInt() - (correct as num).toInt();

    final colorScheme = Theme.of(context).colorScheme;
    return Card(
      color: selected
          ? colorScheme.primary.withValues(alpha: 0.08)
          : const Color(0xFFF8F8F8),
      child: InkWell(
        borderRadius: BorderRadius.circular(12),
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  Expanded(
                    child: Text(label, style: Theme.of(context).textTheme.titleMedium),
                  ),
                  Text(
                    '$accuracy%',
                    style: Theme.of(context).textTheme.titleLarge?.copyWith(
                          color: color,
                          fontWeight: FontWeight.w700,
                        ),
                  ),
                ],
              ),
              const SizedBox(height: 10),
              Row(
                children: [
                  Text('题目 $total'),
                  const SizedBox(width: 14),
                  Text('正确 $correct'),
                  const SizedBox(width: 14),
                  Text('错题 $missed'),
                ],
              ),
              const SizedBox(height: 12),
              ClipRRect(
                borderRadius: BorderRadius.circular(999),
                child: LinearProgressIndicator(
                  value: (accuracy / 100).clamp(0, 1),
                  minHeight: 8,
                  backgroundColor: Colors.black12,
                  valueColor: AlwaysStoppedAnimation<Color>(color),
                ),
              ),
              if (selected) ...[
                const SizedBox(height: 12),
                Text(
                  _modeAdvice(label, accuracy.toDouble()),
                  style: Theme.of(context).textTheme.bodyMedium?.copyWith(color: Colors.black54),
                ),
                const SizedBox(height: 12),
                _DailyLineChart(
                  data: series,
                  selectedDate: null,
                  lineColor: color,
                  onSelect: (_) {},
                  compact: true,
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }
}

class _LineChartPainter extends CustomPainter {
  const _LineChartPainter({
    required this.points,
    required this.lineColor,
    required this.topGutter,
    required this.leftGutter,
    required this.plotHeight,
    required this.plotWidth,
  });

  final List<_ChartPoint> points;
  final Color lineColor;
  final double topGutter;
  final double leftGutter;
  final double plotHeight;
  final double plotWidth;

  @override
  void paint(Canvas canvas, Size size) {
    final axisPaint = Paint()
      ..color = const Color(0xFFD0D7E2)
      ..strokeWidth = 1;
    final guidePaint = Paint()
      ..color = const Color(0xFFE7ECF3)
      ..strokeWidth = 1;
    final linePaint = Paint()
      ..color = lineColor
      ..strokeWidth = 2
      ..style = PaintingStyle.stroke;

    canvas.drawLine(
      Offset(leftGutter, topGutter),
      Offset(leftGutter, topGutter + plotHeight),
      axisPaint,
    );
    canvas.drawLine(
      Offset(leftGutter, topGutter + plotHeight),
      Offset(leftGutter + plotWidth, topGutter + plotHeight),
      axisPaint,
    );
    canvas.drawLine(
      Offset(leftGutter, topGutter),
      Offset(leftGutter + plotWidth, topGutter),
      guidePaint,
    );
    canvas.drawLine(
      Offset(leftGutter, topGutter + plotHeight / 2),
      Offset(leftGutter + plotWidth, topGutter + plotHeight / 2),
      guidePaint,
    );

    final path = Path();
    for (var index = 0; index < points.length; index++) {
      final point = Offset(points[index].x, points[index].y);
      if (index == 0) {
        path.moveTo(point.dx, point.dy);
      } else {
        path.lineTo(point.dx, point.dy);
      }
    }
    canvas.drawPath(path, linePaint);

    final textPainter = TextPainter(textDirection: TextDirection.ltr);
    for (final label in const [
      ('100%', 0.0),
      ('50%', 0.5),
      ('0%', 1.0),
    ]) {
      textPainter.text = TextSpan(
        text: label.$1,
        style: const TextStyle(fontSize: 10, color: Color(0xFF999999)),
      );
      textPainter.layout();
      textPainter.paint(
        canvas,
        Offset(0, topGutter + plotHeight * label.$2 - 8),
      );
    }
  }

  @override
  bool shouldRepaint(covariant _LineChartPainter oldDelegate) {
    return oldDelegate.points != points || oldDelegate.lineColor != lineColor;
  }
}

class _ChartPoint {
  const _ChartPoint({
    required this.item,
    required this.x,
    required this.y,
    required this.accuracy,
  });

  final Map<String, dynamic> item;
  final double x;
  final double y;
  final double accuracy;
}

Color _accuracyColor(double accuracy) {
  if (accuracy < 50) return const Color(0xFFFF3B30);
  if (accuracy < 85) return const Color(0xFFFF9500);
  return const Color(0xFF34C759);
}

String _modeAdvice(String label, double accuracy) {
  if (accuracy >= 85) {
    return '$label 表现稳定，可以继续保持当前节奏。';
  }
  if (accuracy >= 50) {
    return '$label 表现中等，建议继续查漏补缺。';
  }
  return '$label 仍有明显薄弱点，建议放慢速度并加强回顾。';
}

String _formatDurationMs(dynamic ms) {
  final value = (ms as num?)?.toInt() ?? 0;
  final seconds = value ~/ 1000;
  if (seconds < 60) return '${seconds}s';
  final minutes = seconds ~/ 60;
  final remain = seconds % 60;
  return '${minutes}m ${remain}s';
}

class _ReportsMessage extends StatelessWidget {
  const _ReportsMessage({required this.message, required this.onRetry});

  final String message;
  final Future<void> Function() onRetry;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(message, textAlign: TextAlign.center),
            const SizedBox(height: 16),
            FilledButton(onPressed: onRetry, child: const Text('重试')),
          ],
        ),
      ),
    );
  }
}

class _SectionCard extends StatelessWidget {
  const _SectionCard({required this.title, required this.subtitle, required this.child});

  final String title;
  final String subtitle;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return Card(
      margin: const EdgeInsets.only(bottom: 16),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(title, style: Theme.of(context).textTheme.titleLarge),
            const SizedBox(height: 6),
            Text(
              subtitle,
              style: Theme.of(context).textTheme.bodyMedium?.copyWith(color: Colors.black54),
            ),
            const SizedBox(height: 16),
            child,
          ],
        ),
      ),
    );
  }
}
