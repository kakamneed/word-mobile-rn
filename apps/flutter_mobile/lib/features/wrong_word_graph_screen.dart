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

  Future<void> _moveNode(
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
                        onMoveNode: _moveNode,
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
                  nodes: graph.nodes,
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
    required this.onMoveNode,
  });

  final WrongWordGraph graph;
  final String? selectedNodeId;
  final String? savingNodeId;
  final ValueChanged<WrongWordGraphNode> onSelect;
  final void Function(WrongWordGraphNode node, WrongWordGraphPosition position)
  onMoveNode;

  static const Size _sceneSize = Size(1800, 1000);

  @override
  State<_GraphCanvas> createState() => _GraphCanvasState();
}

class _GraphCanvasState extends State<_GraphCanvas> {
  final TransformationController _controller = TransformationController();
  final GlobalKey _viewerKey = GlobalKey();
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
    final box = _viewerKey.currentContext?.findRenderObject() as RenderBox?;
    if (box == null) return false;
    final viewportOffset = box.globalToLocal(globalOffset);
    final viewportSize = box.size;
    if (viewportOffset.dx < 0 ||
        viewportOffset.dy < 0 ||
        viewportOffset.dx > viewportSize.width ||
        viewportOffset.dy > viewportSize.height) {
      final center = Offset(viewportSize.width / 2, viewportSize.height / 2);
      final sceneCenter = _controller.toScene(center);
      widget.onMoveNode(
        node,
        _positionFromSceneOffset(sceneCenter, _GraphCanvas._sceneSize),
      );
      return true;
    }
    final sceneOffset = _controller.toScene(viewportOffset);
    widget.onMoveNode(
      node,
      _positionFromSceneOffset(sceneOffset, _GraphCanvas._sceneSize),
    );
    return true;
  }

  @override
  Widget build(BuildContext context) {
    final placedNodes = widget.graph.nodes
        .where((node) => node.isUserPlaced)
        .toList(growable: false);
    final selectedNode = _selectedNode(placedNodes, widget.selectedNodeId);
    final selectedEdgeCount = selectedNode == null
        ? 0
        : widget.graph.edges
              .where(
                (edge) =>
                    edge.sourceNodeId == selectedNode.id ||
                    edge.targetNodeId == selectedNode.id,
              )
              .length;
    return DragTarget<WrongWordGraphNode>(
      hitTestBehavior: HitTestBehavior.opaque,
      onAcceptWithDetails: (details) {
        placeNodeFromGlobal(details.data, details.offset);
      },
      builder: (context, candidateData, rejectedData) {
        final hover = candidateData.isNotEmpty;
        return ColoredBox(
          color: _GraphColors.space,
          child: InteractiveViewer(
            key: _viewerKey,
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
                  if (placedNodes.isEmpty)
                    const Positioned.fill(child: _GraphEmptySpaceHint()),
                  for (final node in placedNodes)
                    _GraphStarNode(
                      node: node,
                      selected: node.id == widget.selectedNodeId,
                      saving: node.id == widget.savingNodeId,
                      sceneSize: _GraphCanvas._sceneSize,
                      scale: _scale,
                      onTap: () => widget.onSelect(node),
                    ),
                  if (selectedNode != null)
                    Positioned(
                      left: 28,
                      bottom: 28,
                      child: _GraphNodeDetailPanel(
                        node: selectedNode,
                        relationCount: selectedEdgeCount,
                      ),
                    ),
                ],
              ),
            ),
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
  const _GraphNodeDetailPanel({
    required this.node,
    required this.relationCount,
  });

  final WrongWordGraphNode node;
  final int relationCount;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Material(
      elevation: 12,
      color: _GraphColors.panel.withValues(alpha: 0.94),
      borderRadius: BorderRadius.circular(14),
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 360),
        child: Padding(
          padding: const EdgeInsets.all(18),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                node.word,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: theme.textTheme.headlineSmall?.copyWith(
                  color: _GraphColors.text,
                  fontWeight: FontWeight.w700,
                ),
              ),
              if (node.primaryGloss.trim().isNotEmpty) ...[
                const SizedBox(height: 8),
                Text(
                  node.primaryGloss,
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                  style: theme.textTheme.bodyLarge?.copyWith(
                    color: _GraphColors.muted,
                  ),
                ),
              ],
              const SizedBox(height: 14),
              Wrap(
                spacing: 8,
                runSpacing: 8,
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
                    value: relationCount.toString(),
                  ),
                  if (node.lastWrongAt != null && node.lastWrongAt!.isNotEmpty)
                    _GraphMetricChip(
                      label: '\u6700\u8fd1',
                      value: node.lastWrongAt!,
                    ),
                ],
              ),
              if (node.sources.isNotEmpty) ...[
                const SizedBox(height: 10),
                Text(
                  '\u6765\u6e90\uff1a${node.sources.join(' / ')}',
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: theme.textTheme.labelMedium?.copyWith(
                    color: _GraphColors.muted,
                  ),
                ),
              ],
              const SizedBox(height: 14),
              FilledButton.icon(
                onPressed: () {
                  ScaffoldMessenger.of(context).showSnackBar(
                    SnackBar(
                      content: Text('\u51c6\u5907\u590d\u4e60 ${node.word}'),
                    ),
                  );
                },
                icon: const Icon(Icons.play_arrow_rounded),
                label: const Text('\u5f00\u59cb\u590d\u4e60'),
              ),
            ],
          ),
        ),
      ),
    );
  }
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

class _GraphStarNode extends StatelessWidget {
  const _GraphStarNode({
    required this.node,
    required this.selected,
    required this.saving,
    required this.sceneSize,
    required this.scale,
    required this.onTap,
  });

  final WrongWordGraphNode node;
  final bool selected;
  final bool saving;
  final Size sceneSize;
  final double scale;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final offset = _nodeOffset(node, sceneSize);
    final errorWeight = (node.wrongCountTotal.clamp(0, 12) / 12).toDouble();
    final urgencyWeight = node.urgencyScore.clamp(0.0, 1.0);
    final diameter = 10.0 + errorWeight * 14.0 + urgencyWeight * 8.0;
    final labelThreshold = (1.72 - errorWeight * 0.7).clamp(0.92, 1.72);
    final showLabel = scale >= labelThreshold || selected;
    final hitSize = math.max(44.0, diameter + 22.0);
    final theme = Theme.of(context);
    return Positioned(
      left: offset.dx - hitSize / 2,
      top: offset.dy - hitSize / 2,
      child: GestureDetector(
        behavior: HitTestBehavior.translucent,
        onTap: onTap,
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
                  selected: selected,
                ),
              ),
              if (saving)
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
                      node.word,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      textAlign: TextAlign.center,
                      style: theme.textTheme.labelLarge?.copyWith(
                        color: Colors.white.withValues(
                          alpha: selected ? 1 : 0.84,
                        ),
                        fontWeight: selected
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
  bool _dragging = false;
  bool _placedDuringDrag = false;

  void _clearPointer() {
    if (!mounted) return;
    setState(() {
      _dragging = false;
      _pointerDownGlobalPosition = null;
      _lastPointerGlobalPosition = null;
      _placedDuringDrag = false;
    });
  }

  @override
  Widget build(BuildContext context) {
    final tile = ListTile(
      dense: true,
      selected: widget.selected,
      contentPadding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
      selectedTileColor: Colors.white.withValues(alpha: 0.08),
      trailing: widget.saving
          ? const SizedBox(
              width: 18,
              height: 18,
              child: CircularProgressIndicator(strokeWidth: 2),
            )
          : null,
      onTap: () => widget.onDragEnd(widget.node, Offset.zero),
      title: Text(
        widget.node.word,
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
        style: const TextStyle(
          color: _GraphColors.text,
          fontWeight: FontWeight.w600,
        ),
      ),
      subtitle: Text(
        '${widget.node.wrongCountTotal}x',
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
        style: const TextStyle(color: _GraphColors.muted),
      ),
    );

    return Listener(
      key: ValueKey('wrong-word-graph-rail-word:${widget.node.id}'),
      behavior: HitTestBehavior.opaque,
      onPointerDown: (event) {
        setState(() {
          _pointerDownGlobalPosition = event.position;
          _lastPointerGlobalPosition = event.position;
          _placedDuringDrag = false;
          _dragging = false;
        });
      },
      onPointerMove: (event) {
        final down = _pointerDownGlobalPosition;
        if (down == null) return;
        final delta = event.position - down;
        setState(() {
          _lastPointerGlobalPosition = event.position;
          _dragging = delta.dx.abs() > 8 || delta.dy.abs() > 8;
        });
        if (!_placedDuringDrag && delta.dx < -18) {
          _placedDuringDrag = widget.onDragEnd(widget.node, event.position);
        }
      },
      onPointerCancel: (_) => _clearPointer(),
      onPointerUp: (event) {
        final down = _pointerDownGlobalPosition;
        final endOffset = _lastPointerGlobalPosition ?? event.position;
        final wasHorizontalDrag =
            down != null && (endOffset.dx - down.dx) < -18;
        final shouldPlace = wasHorizontalDrag && !_placedDuringDrag;
        _clearPointer();
        if (shouldPlace) {
          widget.onDragEnd(widget.node, endOffset);
        }
      },
      child: Opacity(opacity: _dragging ? 0.56 : 1, child: tile),
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
