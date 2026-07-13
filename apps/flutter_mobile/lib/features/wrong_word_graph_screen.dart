import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../sdk/sdk.dart';

class WrongWordGraphScreen extends StatefulWidget {
  const WrongWordGraphScreen({super.key, required this.sdk});

  final WordSdk sdk;

  @override
  State<WrongWordGraphScreen> createState() => _WrongWordGraphScreenState();
}

class _WrongWordGraphScreenState extends State<WrongWordGraphScreen> {
  WrongWordGraph? _graph;
  String? _error;
  bool _loading = true;
  String? _selectedNodeId;
  String? _savingNodeId;
  final GlobalKey<_GraphCanvasState> _graphCanvasKey =
      GlobalKey<_GraphCanvasState>();

  @override
  void initState() {
    super.initState();
    SystemChrome.setPreferredOrientations(const [
      DeviceOrientation.landscapeLeft,
      DeviceOrientation.landscapeRight,
    ]);
    _load();
  }

  @override
  void dispose() {
    SystemChrome.setPreferredOrientations(DeviceOrientation.values);
    super.dispose();
  }

  Future<void> _load() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final graph = await widget.sdk.wrongWords.getWrongWordGraph();
      if (!mounted) return;
      setState(() {
        _graph = graph;
        _selectedNodeId = null;
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

  void _previewMoveNode(
    WrongWordGraphNode node,
    WrongWordGraphPosition position,
  ) {
    final graph = _graph;
    if (graph == null) return;
    final updatedNode = node.copyWith(position: position, isUserPlaced: true);
    setState(() {
      _graph = graph.copyWith(
        nodes: graph.nodes
            .map((item) => item.id == node.id ? updatedNode : item)
            .toList(growable: false),
      );
    });
  }

  Future<void> _commitMoveNode(
    WrongWordGraphNode node,
    WrongWordGraphPosition position,
  ) async {
    final graph = _graph;
    if (graph == null) return;
    final updatedNode = node.copyWith(position: position, isUserPlaced: true);
    setState(() {
      _selectedNodeId = node.id;
      _savingNodeId = node.id;
      _graph = graph.copyWith(
        nodes: graph.nodes
            .map((item) => item.id == node.id ? updatedNode : item)
            .toList(growable: false),
      );
    });
    try {
      final saved = await widget.sdk.wrongWords.saveWrongWordGraphPosition(
        entryId: node.entryId,
        entryKind: node.entryKind,
        position: position,
      );
      if (!mounted) return;
      final latest = _graph;
      if (latest == null) return;
      setState(() {
        _graph = latest.copyWith(
          nodes: latest.nodes
              .map(
                (item) => item.id == node.id
                    ? item.copyWith(
                        position: position,
                        isUserPlaced: true,
                        positionUpdatedAt: saved.positionUpdatedAt,
                      )
                    : item,
              )
              .toList(growable: false),
        );
      });
    } catch (error) {
      if (!mounted) return;
      setState(() {
        _error = error.toString();
      });
      await _load();
    } finally {
      if (mounted) {
        setState(() {
          _savingNodeId = null;
        });
      }
    }
  }

  void _clearSelection() {
    if (_selectedNodeId == null) return;
    setState(() {
      _selectedNodeId = null;
    });
  }

  @override
  Widget build(BuildContext context) {
    final graph = _graph;
    return Scaffold(
      backgroundColor: _GraphColors.space,
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : _error != null
          ? _GraphMessage(message: _error!, onRetry: _load)
          : graph == null || graph.nodes.isEmpty
          ? _GraphMessage(
              message:
                  '\u8fd8\u6ca1\u6709\u53ef\u7528\u7684\u9519\u8bcd\u8282\u70b9',
              onRetry: _load,
            )
          : Row(
              children: [
                Expanded(
                  child: Stack(
                    children: [
                      _GraphCanvas(
                        key: _graphCanvasKey,
                        graph: graph,
                        selectedNodeId: _selectedNodeId,
                        savingNodeId: _savingNodeId,
                        onSelect: (node) {
                          setState(() {
                            _selectedNodeId = node.id;
                          });
                        },
                        onPreviewMoveNode: _previewMoveNode,
                        onCommitMoveNode: _commitMoveNode,
                        onClearSelection: _clearSelection,
                      ),
                      SafeArea(
                        child: Padding(
                          padding: const EdgeInsets.all(10),
                          child: DecoratedBox(
                            decoration: BoxDecoration(
                              color: Colors.black.withValues(alpha: 0.38),
                              shape: BoxShape.circle,
                              border: Border.all(
                                color: Colors.white.withValues(alpha: 0.12),
                              ),
                            ),
                            child: IconButton(
                              tooltip: '\u9000\u51fa',
                              onPressed: () => Navigator.of(context).maybePop(),
                              color: Colors.white,
                              icon: const Icon(Icons.arrow_back_rounded),
                            ),
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
                _GraphRail(
                  nodes: [...graph.nodes]
                    ..sort((a, b) {
                      final byWrongCount = b.wrongCountTotal.compareTo(
                        a.wrongCountTotal,
                      );
                      if (byWrongCount != 0) return byWrongCount;
                      return a.word.compareTo(b.word);
                    }),
                  selectedNodeId: _selectedNodeId,
                  savingNodeId: _savingNodeId,
                  onDragEnd: (node, offset) {
                    return _graphCanvasKey.currentState?.placeNodeFromGlobal(
                          node,
                          offset,
                        ) ??
                        false;
                  },
                ),
              ],
            ),
    );
  }
}

class _GraphCanvas extends StatefulWidget {
  const _GraphCanvas({
    super.key,
    required this.graph,
    required this.selectedNodeId,
    required this.savingNodeId,
    required this.onSelect,
    required this.onPreviewMoveNode,
    required this.onCommitMoveNode,
    required this.onClearSelection,
  });

  final WrongWordGraph graph;
  final String? selectedNodeId;
  final String? savingNodeId;
  final ValueChanged<WrongWordGraphNode> onSelect;
  final void Function(WrongWordGraphNode node, WrongWordGraphPosition position)
  onPreviewMoveNode;
  final void Function(WrongWordGraphNode node, WrongWordGraphPosition position)
  onCommitMoveNode;
  final VoidCallback onClearSelection;

  static const Size _sceneSize = Size(1800, 1000);

  @override
  State<_GraphCanvas> createState() => _GraphCanvasState();
}

class _GraphCanvasState extends State<_GraphCanvas> {
  final TransformationController _controller = TransformationController();
  final GlobalKey _viewportKey = GlobalKey();
  double _scale = 1;

  @override
  void initState() {
    super.initState();
    _controller.addListener(_syncScale);
  }

  @override
  void dispose() {
    _controller.removeListener(_syncScale);
    _controller.dispose();
    super.dispose();
  }

  void _syncScale() {
    final nextScale = _controller.value.getMaxScaleOnAxis();
    if ((nextScale - _scale).abs() < 0.01) return;
    setState(() {
      _scale = nextScale;
    });
  }

  bool placeNodeFromGlobal(WrongWordGraphNode node, Offset globalOffset) {
    final box = _viewportKey.currentContext?.findRenderObject() as RenderBox?;
    if (box == null) return false;
    final viewportOffset = box.globalToLocal(globalOffset);
    final viewportSize = box.size;
    if (viewportOffset.dx < 0 ||
        viewportOffset.dy < 0 ||
        viewportOffset.dx > viewportSize.width ||
        viewportOffset.dy > viewportSize.height) {
      return false;
    }
    final sceneOffset = _controller.toScene(viewportOffset);
    _commitNodeAtSceneOffset(node, sceneOffset);
    return true;
  }

  bool previewNodeFromGlobal(WrongWordGraphNode node, Offset globalOffset) {
    final box = _viewportKey.currentContext?.findRenderObject() as RenderBox?;
    if (box == null) return false;
    final viewportOffset = box.globalToLocal(globalOffset);
    final viewportSize = box.size;
    if (viewportOffset.dx < 0 ||
        viewportOffset.dy < 0 ||
        viewportOffset.dx > viewportSize.width ||
        viewportOffset.dy > viewportSize.height) {
      return false;
    }
    _previewNodeAtSceneOffset(node, _controller.toScene(viewportOffset));
    return true;
  }

  WrongWordGraphPosition _positionForSceneOffset(
    WrongWordGraphNode node,
    Offset sceneOffset,
  ) {
    final placedNodes = widget.graph.nodes
        .where((item) => item.isUserPlaced)
        .toList(growable: false);
    final separatedOffset = _resolveSeparatedSceneOffset(
      target: sceneOffset,
      movingNode: node,
      nodes: placedNodes,
      sceneSize: _GraphCanvas._sceneSize,
    );
    return _positionFromSceneOffset(separatedOffset, _GraphCanvas._sceneSize);
  }

  void _previewNodeAtSceneOffset(WrongWordGraphNode node, Offset sceneOffset) {
    widget.onPreviewMoveNode(node, _positionForSceneOffset(node, sceneOffset));
  }

  void _commitNodeAtSceneOffset(WrongWordGraphNode node, Offset sceneOffset) {
    widget.onCommitMoveNode(node, _positionForSceneOffset(node, sceneOffset));
  }

  void _focusNode(WrongWordGraphNode node) {
    final box = _viewportKey.currentContext?.findRenderObject() as RenderBox?;
    if (box == null) return;
    const targetScale = 1.85;
    final nodeOffset = _nodeOffset(node, _GraphCanvas._sceneSize);
    final center = box.size.center(Offset.zero);
    final x = center.dx - nodeOffset.dx * targetScale;
    final y = center.dy - nodeOffset.dy * targetScale;
    _controller.value = Matrix4.identity()
      ..setEntry(0, 0, targetScale)
      ..setEntry(1, 1, targetScale)
      ..setEntry(0, 3, x)
      ..setEntry(1, 3, y);
  }

  @override
  Widget build(BuildContext context) {
    final placedNodes = widget.graph.nodes
        .where((node) => node.isUserPlaced)
        .toList(growable: false);
    final selectedNode = _selectedNode(placedNodes, widget.selectedNodeId);
    final selectedRelations = selectedNode == null
        ? const <_GraphRelationSummary>[]
        : _relationSummaries(selectedNode, placedNodes, widget.graph.edges);

    return DragTarget<WrongWordGraphNode>(
      hitTestBehavior: HitTestBehavior.opaque,
      onAcceptWithDetails: (details) {
        placeNodeFromGlobal(details.data, details.offset);
      },
      builder: (context, candidateData, rejectedData) {
        final hover = candidateData.isNotEmpty;
        return ColoredBox(
          key: _viewportKey,
          color: _GraphColors.space,
          child: Stack(
            children: [
              InteractiveViewer(
                transformationController: _controller,
                constrained: false,
                clipBehavior: Clip.none,
                minScale: 0.25,
                maxScale: 3.4,
                boundaryMargin: const EdgeInsets.all(1100),
                child: SizedBox(
                  width: _GraphCanvas._sceneSize.width,
                  height: _GraphCanvas._sceneSize.height,
                  child: Stack(
                    clipBehavior: Clip.none,
                    children: [
                      CustomPaint(
                        size: _GraphCanvas._sceneSize,
                        painter: _GraphBackgroundPainter(
                          nodes: placedNodes,
                          edges: widget.graph.edges,
                          selectedNodeId: widget.selectedNodeId,
                          dropTargetActive: hover,
                        ),
                      ),
                      Positioned.fill(
                        child: GestureDetector(
                          behavior: HitTestBehavior.opaque,
                          onTap: widget.onClearSelection,
                        ),
                      ),
                      if (placedNodes.isEmpty)
                        const Positioned.fill(child: _GraphEmptySpaceHint()),
                      for (final node in placedNodes)
                        _GraphStarNode(
                          node: node,
                          selected: node.id == widget.selectedNodeId,
                          saving: node.id == widget.savingNodeId,
                          sceneSize: _GraphCanvas._sceneSize,
                          scale: _scale,
                          onTap: () {
                            widget.onSelect(node);
                            _focusNode(node);
                          },
                          onPreviewFromGlobal: (offset) =>
                              previewNodeFromGlobal(node, offset),
                          onCommitFromGlobal: (offset) =>
                              placeNodeFromGlobal(node, offset),
                          onMoveStart: widget.onClearSelection,
                        ),
                    ],
                  ),
                ),
              ),
              if (selectedNode != null)
                Positioned(
                  left: 84,
                  bottom: 20,
                  child: _GraphNodeDetailPanel(
                    node: selectedNode,
                    relations: selectedRelations,
                  ),
                ),
            ],
          ),
        );
      },
    );
  }
}

WrongWordGraphNode? _selectedNode(
  List<WrongWordGraphNode> nodes,
  String? selectedNodeId,
) {
  if (selectedNodeId == null) return null;
  for (final node in nodes) {
    if (node.id == selectedNodeId) return node;
  }
  return null;
}

class _GraphColors {
  const _GraphColors._();

  static const space = Color(0xFF080A0F);
  static const panel = Color(0xFF10131A);
  static const panelBorder = Color(0xFF272D38);
  static const text = Color(0xFFEDEFF7);
  static const muted = Color(0xFF9AA3B2);
  static const accent = Color(0xFF5FE1A1);
  static const hot = Color(0xFFFFFFFF);
}

class _GraphNodeDetailPanel extends StatelessWidget {
  const _GraphNodeDetailPanel({required this.node, required this.relations});

  final WrongWordGraphNode node;
  final List<_GraphRelationSummary> relations;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Material(
      elevation: 10,
      color: _GraphColors.panel.withValues(alpha: 0.94),
      borderRadius: BorderRadius.circular(12),
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 300),
        child: Padding(
          padding: const EdgeInsets.all(14),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                node.word,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: theme.textTheme.titleLarge?.copyWith(
                  color: _GraphColors.text,
                  fontWeight: FontWeight.w700,
                ),
              ),
              if (node.primaryGloss.trim().isNotEmpty) ...[
                const SizedBox(height: 6),
                Text(
                  node.primaryGloss,
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: _GraphColors.muted,
                  ),
                ),
              ],
              const SizedBox(height: 10),
              Wrap(
                spacing: 6,
                runSpacing: 6,
                children: [
                  _GraphMetricChip(
                    label: '\u4eca\u65e5\u9519',
                    value: node.wrongCountToday.toString(),
                  ),
                  _GraphMetricChip(
                    label: '\u603b\u9519\u8bef',
                    value: node.wrongCountTotal.toString(),
                  ),
                  _GraphMetricChip(
                    label: '\u5173\u7cfb',
                    value: relations.length.toString(),
                  ),
                  if (node.lastWrongAt != null && node.lastWrongAt!.isNotEmpty)
                    _GraphMetricChip(
                      label: '\u6700\u8fd1',
                      value: _formatGraphDate(node.lastWrongAt!),
                    ),
                ],
              ),
              if (relations.isNotEmpty) ...[
                const SizedBox(height: 10),
                for (final relation in relations.take(4))
                  Padding(
                    padding: const EdgeInsets.only(bottom: 6),
                    child: _GraphRelationLine(summary: relation),
                  ),
              ],
              if (node.sources.isNotEmpty) ...[
                const SizedBox(height: 4),
                Text(
                  '\u6765\u6e90\uff1a${node.sources.join(' / ')}',
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: theme.textTheme.labelSmall?.copyWith(
                    color: _GraphColors.muted,
                  ),
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }
}

class _GraphRelationSummary {
  const _GraphRelationSummary({
    required this.color,
    required this.label,
    required this.detail,
  });

  final Color color;
  final String label;
  final String detail;
}

class _GraphRelationLine extends StatelessWidget {
  const _GraphRelationLine({required this.summary});

  final _GraphRelationSummary summary;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Padding(
          padding: const EdgeInsets.only(top: 5),
          child: DecoratedBox(
            decoration: BoxDecoration(
              color: summary.color,
              shape: BoxShape.circle,
            ),
            child: const SizedBox(width: 7, height: 7),
          ),
        ),
        const SizedBox(width: 7),
        Expanded(
          child: Text(
            '${summary.label}：${summary.detail}',
            maxLines: 2,
            overflow: TextOverflow.ellipsis,
            style: theme.textTheme.labelSmall?.copyWith(
              color: _GraphColors.text.withValues(alpha: 0.86),
              height: 1.25,
            ),
          ),
        ),
      ],
    );
  }
}

List<_GraphRelationSummary> _relationSummaries(
  WrongWordGraphNode selectedNode,
  List<WrongWordGraphNode> nodes,
  List<WrongWordGraphEdge> edges,
) {
  final nodeById = {for (final node in nodes) node.id: node};
  final summaries = <_GraphRelationSummary>[];
  for (final edge in edges) {
    if (edge.sourceNodeId != selectedNode.id &&
        edge.targetNodeId != selectedNode.id) {
      continue;
    }
    final otherId = edge.sourceNodeId == selectedNode.id
        ? edge.targetNodeId
        : edge.sourceNodeId;
    final otherWord = nodeById[otherId]?.word ?? '';
    final evidence = edge.evidence.whereType<Map<String, dynamic>>().toList(
      growable: false,
    );
    summaries.add(
      _GraphRelationSummary(
        color: _relationColor(edge.relationType),
        label: _relationLabel(edge.relationType),
        detail: _relationDetail(edge.relationType, evidence, otherWord),
      ),
    );
  }
  summaries.sort((a, b) => a.label.compareTo(b.label));
  return summaries;
}

Color _relationColor(String relationType) {
  switch (relationType) {
    case 'synonym':
      return const Color(0xFF53B87A);
    case 'coOccurrence':
      return const Color(0xFFD75B5B);
    case 'rootFamily':
      return const Color(0xFF8B5CC6);
    case 'similarForm':
      return const Color(0xFF8C929E);
    default:
      return _GraphColors.muted;
  }
}

String _relationLabel(String relationType) {
  switch (relationType) {
    case 'synonym':
      return '重叠释义';
    case 'coOccurrence':
      return '同篇 AI 短文';
    case 'rootFamily':
      return '相同词根词缀';
    case 'similarForm':
      return '形近拼写';
    default:
      return '关系';
  }
}

String _relationDetail(
  String relationType,
  List<Map<String, dynamic>> evidence,
  String otherWord,
) {
  String field(String key) {
    for (final item in evidence) {
      final value = item[key];
      if (value is String && value.trim().isNotEmpty) return value.trim();
      if (value is num) return value.toString();
    }
    return '';
  }

  final withOther = otherWord.isEmpty ? '' : ' · $otherWord';
  switch (relationType) {
    case 'synonym':
      final meaning = field('meaning');
      return meaning.isEmpty ? '释义重叠$withOther' : '$meaning$withOther';
    case 'coOccurrence':
      final title = field('passageTitle');
      return title.isEmpty ? '同一篇 AI 短文$withOther' : '$title$withOther';
    case 'rootFamily':
      final family = field(
        'family',
      ).replaceFirst('prefix:', '前缀 ').replaceFirst('suffix:', '后缀 ');
      return family.isEmpty ? '同源构词$withOther' : '$family$withOther';
    case 'similarForm':
      return '拼写或词形接近$withOther';
    default:
      return otherWord;
  }
}

String _formatGraphDate(String raw) {
  final parsed = DateTime.tryParse(raw);
  if (parsed == null) {
    return raw.replaceFirst('T', ' ').split('.').first;
  }
  final local = parsed.toLocal();
  String two(int value) => value.toString().padLeft(2, '0');
  return '${local.year}-${two(local.month)}-${two(local.day)} ${two(local.hour)}:${two(local.minute)}';
}

class _GraphMetricChip extends StatelessWidget {
  const _GraphMetricChip({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
      decoration: BoxDecoration(
        color: Colors.white.withValues(alpha: 0.06),
        borderRadius: BorderRadius.circular(999),
        border: Border.all(color: Colors.white.withValues(alpha: 0.10)),
      ),
      child: Text(
        '$label $value',
        style: theme.textTheme.labelMedium?.copyWith(
          color: _GraphColors.text,
          fontWeight: FontWeight.w600,
        ),
      ),
    );
  }
}

class _GraphStarNode extends StatefulWidget {
  const _GraphStarNode({
    required this.node,
    required this.selected,
    required this.saving,
    required this.sceneSize,
    required this.scale,
    required this.onTap,
    required this.onPreviewFromGlobal,
    required this.onCommitFromGlobal,
    required this.onMoveStart,
  });

  final WrongWordGraphNode node;
  final bool selected;
  final bool saving;
  final Size sceneSize;
  final double scale;
  final VoidCallback onTap;
  final bool Function(Offset globalOffset) onPreviewFromGlobal;
  final bool Function(Offset globalOffset) onCommitFromGlobal;
  final VoidCallback onMoveStart;

  @override
  State<_GraphStarNode> createState() => _GraphStarNodeState();
}

class _GraphStarNodeState extends State<_GraphStarNode> {
  bool _moving = false;

  @override
  Widget build(BuildContext context) {
    final offset = _nodeOffset(widget.node, widget.sceneSize);
    final errorWeight = (widget.node.wrongCountTotal.clamp(0, 12) / 12)
        .toDouble();
    final urgencyWeight = widget.node.urgencyScore.clamp(0.0, 1.0);
    final diameter = 10.0 + errorWeight * 14.0 + urgencyWeight * 8.0;
    final labelThreshold = (1.10 - errorWeight * 0.58).clamp(0.46, 1.10);
    final showLabel =
        widget.node.wrongCountTotal >= 8 ||
        widget.scale >= labelThreshold ||
        widget.selected;
    final hitSize = math.max(52.0, diameter + 28.0);
    final theme = Theme.of(context);
    return Positioned(
      left: offset.dx - hitSize / 2,
      top: offset.dy - hitSize / 2,
      child: GestureDetector(
        behavior: HitTestBehavior.translucent,
        onTap: widget.onTap,
        onLongPressStart: (details) {
          setState(() {
            _moving = true;
          });
          widget.onMoveStart();
          widget.onPreviewFromGlobal(details.globalPosition);
        },
        onLongPressMoveUpdate: (details) {
          widget.onPreviewFromGlobal(details.globalPosition);
        },
        onLongPressEnd: (details) {
          widget.onCommitFromGlobal(details.globalPosition);
          if (mounted) {
            setState(() {
              _moving = false;
            });
          }
        },
        child: SizedBox(
          width: hitSize,
          height: hitSize + (showLabel ? 30 : 0),
          child: Stack(
            clipBehavior: Clip.none,
            alignment: Alignment.topCenter,
            children: [
              Positioned(
                top: (hitSize - diameter) / 2,
                child: _StarDot(
                  diameter: diameter,
                  brightness: 0.48 + errorWeight * 0.46,
                  selected: widget.selected || _moving,
                ),
              ),
              if (widget.saving)
                Positioned(
                  top: (hitSize - 18) / 2,
                  child: const SizedBox(
                    width: 18,
                    height: 18,
                    child: CircularProgressIndicator(
                      strokeWidth: 2,
                      color: _GraphColors.accent,
                    ),
                  ),
                ),
              if (showLabel)
                Positioned(
                  top: hitSize - 3,
                  child: ConstrainedBox(
                    constraints: const BoxConstraints(maxWidth: 150),
                    child: Text(
                      widget.node.word,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      textAlign: TextAlign.center,
                      style: theme.textTheme.labelLarge?.copyWith(
                        color: Colors.white.withValues(
                          alpha: widget.selected ? 1 : 0.84,
                        ),
                        fontWeight: widget.selected
                            ? FontWeight.w800
                            : FontWeight.w600,
                        shadows: const [
                          Shadow(color: Colors.black, blurRadius: 6),
                        ],
                      ),
                    ),
                  ),
                ),
            ],
          ),
        ),
      ),
    );
  }
}

class _StarDot extends StatelessWidget {
  const _StarDot({
    required this.diameter,
    required this.brightness,
    required this.selected,
  });

  final double diameter;
  final double brightness;
  final bool selected;

  @override
  Widget build(BuildContext context) {
    final color = Color.lerp(
      const Color(0xFF8F98A6),
      _GraphColors.hot,
      brightness,
    )!;
    return DecoratedBox(
      decoration: BoxDecoration(
        shape: BoxShape.circle,
        color: color.withValues(alpha: brightness),
        boxShadow: [
          BoxShadow(
            color: color.withValues(alpha: selected ? 0.72 : 0.38),
            blurRadius: selected ? 26 : 12,
            spreadRadius: selected ? 6 : 1.5,
          ),
          if (selected)
            BoxShadow(
              color: _GraphColors.accent.withValues(alpha: 0.44),
              blurRadius: 38,
              spreadRadius: 8,
            ),
        ],
      ),
      child: SizedBox(width: diameter, height: diameter),
    );
  }
}

class _GraphEmptySpaceHint extends StatelessWidget {
  const _GraphEmptySpaceHint();

  @override
  Widget build(BuildContext context) {
    return Center(
      child: IgnorePointer(
        child: Text(
          '\u4ece\u53f3\u4fa7\u62d6\u5165\u5355\u8bcd',
          style: Theme.of(context).textTheme.titleMedium?.copyWith(
            color: Colors.white.withValues(alpha: 0.34),
            fontWeight: FontWeight.w600,
          ),
        ),
      ),
    );
  }
}

class _GraphBackgroundPainter extends CustomPainter {
  const _GraphBackgroundPainter({
    required this.nodes,
    required this.edges,
    required this.selectedNodeId,
    required this.dropTargetActive,
  });

  final List<WrongWordGraphNode> nodes;
  final List<WrongWordGraphEdge> edges;
  final String? selectedNodeId;
  final bool dropTargetActive;

  @override
  void paint(Canvas canvas, Size size) {
    canvas.drawRect(Offset.zero & size, Paint()..color = _GraphColors.space);

    final dustPaint = Paint()..color = Colors.white.withValues(alpha: 0.08);
    for (var i = 0; i < 140; i++) {
      final x = (i * 97 % size.width.toInt()).toDouble();
      final y = (i * 53 % size.height.toInt()).toDouble();
      final radius = 0.6 + (i % 5) * 0.22;
      canvas.drawCircle(Offset(x, y), radius, dustPaint);
    }

    final nodeById = {for (final node in nodes) node.id: node};
    for (final edge in edges) {
      final edgeColor = _edgeColor(edge.relationType);
      if (edgeColor == null) continue;
      final source = nodeById[edge.sourceNodeId];
      final target = nodeById[edge.targetNodeId];
      if (source == null || target == null) continue;
      final connectedToSelected =
          selectedNodeId == null ||
          source.id == selectedNodeId ||
          target.id == selectedNodeId;
      final paint = Paint()
        ..color = edgeColor.withValues(alpha: connectedToSelected ? 0.34 : 0.08)
        ..strokeWidth = 1 + edge.weight.clamp(0.0, 1.0) * 2.4
        ..strokeCap = StrokeCap.round;
      canvas.drawLine(
        _nodeOffset(source, size),
        _nodeOffset(target, size),
        paint,
      );
    }

    if (dropTargetActive) {
      final paint = Paint()
        ..style = PaintingStyle.stroke
        ..strokeWidth = 4
        ..color = _GraphColors.accent.withValues(alpha: 0.34);
      canvas.drawRRect(
        RRect.fromRectAndRadius(Offset.zero & size, const Radius.circular(18)),
        paint,
      );
    }

    WrongWordGraphNode? selected;
    for (final node in nodes) {
      if (node.id == selectedNodeId) {
        selected = node;
        break;
      }
    }
    if (selected == null) return;

    final selectedOffset = _nodeOffset(selected, size);
    final haloPaint = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = 2
      ..color = _GraphColors.accent.withValues(alpha: 0.24);
    canvas.drawCircle(selectedOffset, 112, haloPaint);
  }

  Color? _edgeColor(String relationType) {
    switch (relationType) {
      case 'similarForm':
        return const Color(0xFF8C929E);
      case 'synonym':
        return const Color(0xFF53B87A);
      case 'coOccurrence':
        return const Color(0xFFD75B5B);
      case 'rootFamily':
        return const Color(0xFF8B5CC6);
      default:
        return null;
    }
  }

  @override
  bool shouldRepaint(covariant _GraphBackgroundPainter oldDelegate) =>
      oldDelegate.nodes != nodes ||
      oldDelegate.edges != edges ||
      oldDelegate.selectedNodeId != selectedNodeId ||
      oldDelegate.dropTargetActive != dropTargetActive;
}

class _GraphRail extends StatelessWidget {
  const _GraphRail({
    required this.nodes,
    required this.selectedNodeId,
    required this.savingNodeId,
    required this.onDragEnd,
  });

  final List<WrongWordGraphNode> nodes;
  final String? selectedNodeId;
  final String? savingNodeId;
  final bool Function(WrongWordGraphNode node, Offset globalOffset) onDragEnd;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      width: 112,
      decoration: const BoxDecoration(
        color: _GraphColors.panel,
        border: Border(left: BorderSide(color: _GraphColors.panelBorder)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(14, 14, 10, 10),
            child: Text(
              '\u9519\u8bcd',
              style: theme.textTheme.titleMedium?.copyWith(
                color: _GraphColors.text,
                fontWeight: FontWeight.w700,
              ),
            ),
          ),
          Expanded(
            child: ListView.separated(
              padding: const EdgeInsets.fromLTRB(8, 0, 8, 12),
              itemCount: nodes.length,
              separatorBuilder: (context, index) => const SizedBox(height: 6),
              itemBuilder: (context, index) {
                final node = nodes[index];
                return _GraphRailTile(
                  node: node,
                  selected: node.id == selectedNodeId,
                  saving: node.id == savingNodeId,
                  onDragEnd: onDragEnd,
                );
              },
            ),
          ),
        ],
      ),
    );
  }
}

class _GraphRailTile extends StatefulWidget {
  const _GraphRailTile({
    required this.node,
    required this.selected,
    required this.saving,
    required this.onDragEnd,
  });

  final WrongWordGraphNode node;
  final bool selected;
  final bool saving;
  final bool Function(WrongWordGraphNode node, Offset globalOffset) onDragEnd;

  @override
  State<_GraphRailTile> createState() => _GraphRailTileState();
}

class _GraphRailTileState extends State<_GraphRailTile> {
  Offset? _pointerDownGlobalPosition;
  Offset? _lastPointerGlobalPosition;
  OverlayEntry? _dragOverlay;
  bool _dragging = false;

  void _showDragOverlay(Offset position) {
    _lastPointerGlobalPosition = position;
    if (_dragOverlay == null) {
      _dragOverlay = OverlayEntry(builder: _buildDragOverlay);
      Overlay.of(context).insert(_dragOverlay!);
    } else {
      _dragOverlay!.markNeedsBuild();
    }
  }

  Widget _buildDragOverlay(BuildContext context) {
    final position = _lastPointerGlobalPosition ?? Offset.zero;
    return Positioned(
      left: position.dx - 44,
      top: position.dy - 24,
      child: IgnorePointer(
        child: Material(
          color: Colors.transparent,
          child: DecoratedBox(
            decoration: BoxDecoration(
              color: _GraphColors.panel.withValues(alpha: 0.92),
              borderRadius: BorderRadius.circular(8),
              border: Border.all(color: Colors.white.withValues(alpha: 0.12)),
              boxShadow: [
                BoxShadow(
                  color: Colors.black.withValues(alpha: 0.38),
                  blurRadius: 16,
                  offset: const Offset(0, 8),
                ),
              ],
            ),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
              child: Text(
                widget.node.word,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: const TextStyle(
                  color: _GraphColors.text,
                  fontWeight: FontWeight.w700,
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }

  void _removeDragOverlay() {
    _dragOverlay?.remove();
    _dragOverlay = null;
  }

  void _clearPointer() {
    _removeDragOverlay();
    if (!mounted) return;
    setState(() {
      _dragging = false;
      _pointerDownGlobalPosition = null;
      _lastPointerGlobalPosition = null;
    });
  }

  @override
  void dispose() {
    _removeDragOverlay();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final tile = AnimatedContainer(
      duration: const Duration(milliseconds: 120),
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 10),
      decoration: BoxDecoration(
        color: widget.selected
            ? Colors.white.withValues(alpha: 0.08)
            : Colors.transparent,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Row(
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  widget.node.word,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: const TextStyle(
                    color: _GraphColors.text,
                    fontWeight: FontWeight.w600,
                  ),
                ),
                const SizedBox(height: 5),
                Text(
                  '${widget.node.wrongCountTotal}x',
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: const TextStyle(color: _GraphColors.muted),
                ),
              ],
            ),
          ),
          if (widget.saving)
            const SizedBox(
              width: 18,
              height: 18,
              child: CircularProgressIndicator(strokeWidth: 2),
            ),
        ],
      ),
    );

    return Listener(
      key: ValueKey('wrong-word-graph-rail-word:${widget.node.id}'),
      behavior: HitTestBehavior.opaque,
      onPointerDown: (event) {
        setState(() {
          _pointerDownGlobalPosition = event.position;
          _lastPointerGlobalPosition = event.position;
          _dragging = false;
        });
      },
      onPointerMove: (event) {
        final down = _pointerDownGlobalPosition;
        if (down == null) return;
        final delta = event.position - down;
        if (delta.dx < -8 || _dragging) {
          if (!_dragging) {
            setState(() {
              _dragging = true;
            });
          }
          _showDragOverlay(event.position);
        }
      },
      onPointerCancel: (_) => _clearPointer(),
      onPointerUp: (event) {
        final down = _pointerDownGlobalPosition;
        final endOffset = _lastPointerGlobalPosition ?? event.position;
        final wasHorizontalDrag =
            down != null && (endOffset.dx - down.dx) < -18;
        if (wasHorizontalDrag) {
          widget.onDragEnd(widget.node, endOffset);
        }
        _clearPointer();
      },
      child: Opacity(opacity: _dragging ? 0.42 : 1, child: tile),
    );
  }
}

class _GraphMessage extends StatelessWidget {
  const _GraphMessage({required this.message, required this.onRetry});

  final String message;
  final Future<void> Function() onRetry;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Text(
            message,
            textAlign: TextAlign.center,
            style: const TextStyle(color: _GraphColors.text),
          ),
          const SizedBox(height: 12),
          FilledButton.tonal(
            onPressed: onRetry,
            child: const Text('\u91cd\u8bd5'),
          ),
        ],
      ),
    );
  }
}

Offset _resolveSeparatedSceneOffset({
  required Offset target,
  required WrongWordGraphNode movingNode,
  required List<WrongWordGraphNode> nodes,
  required Size sceneSize,
}) {
  const minDistance = 96.0;
  var candidate = Offset(
    target.dx.clamp(80, sceneSize.width - 80).toDouble(),
    target.dy.clamp(60, sceneSize.height - 60).toDouble(),
  );
  final occupied = nodes
      .where((node) => node.id != movingNode.id)
      .map((node) => _nodeOffset(node, sceneSize))
      .toList(growable: false);

  for (var attempt = 0; attempt < 24; attempt++) {
    final tooClose = occupied.any(
      (offset) => (offset - candidate).distance < minDistance,
    );
    if (!tooClose) return candidate;
    final angle = attempt * 2.399963229728653;
    final radius = minDistance * (1 + attempt ~/ 8);
    candidate = Offset(
      target.dx + math.cos(angle) * radius,
      target.dy + math.sin(angle) * radius,
    );
    candidate = Offset(
      candidate.dx.clamp(80, sceneSize.width - 80).toDouble(),
      candidate.dy.clamp(60, sceneSize.height - 60).toDouble(),
    );
  }
  return candidate;
}

Offset _nodeOffset(WrongWordGraphNode node, Size sceneSize) {
  final x = (node.position.x * 260) + sceneSize.width / 2;
  final y = (node.position.y * 260) + sceneSize.height / 2;
  return Offset(
    x.clamp(80, sceneSize.width - 80).toDouble(),
    y.clamp(60, sceneSize.height - 60).toDouble(),
  );
}

WrongWordGraphPosition _positionFromSceneOffset(Offset offset, Size sceneSize) {
  final x = ((offset.dx - sceneSize.width / 2) / 260).clamp(-3.0, 3.0);
  final y = ((offset.dy - sceneSize.height / 2) / 260).clamp(-3.0, 3.0);
  return WrongWordGraphPosition(x: x.toDouble(), y: y.toDouble(), z: 0.5);
}
