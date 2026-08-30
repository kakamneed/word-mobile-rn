import 'package:flutter/widgets.dart';

/// Keeps visited pages alive while only laying out the currently active page.
class ActivePageStack extends StatefulWidget {
  const ActivePageStack({
    required this.index,
    required this.children,
    super.key,
  }) : assert(children.length > 0),
       assert(index >= 0 && index < children.length);

  final int index;
  final List<Widget> children;

  @override
  State<ActivePageStack> createState() => _ActivePageStackState();
}

class _ActivePageStackState extends State<ActivePageStack> {
  late final PageController _controller;

  @override
  void initState() {
    super.initState();
    _controller = PageController(initialPage: widget.index);
  }

  @override
  void didUpdateWidget(covariant ActivePageStack oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.index == oldWidget.index) return;

    if (_controller.hasClients) {
      _controller.jumpToPage(widget.index);
      return;
    }

    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted && _controller.hasClients) {
        _controller.jumpToPage(widget.index);
      }
    });
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return PageView.builder(
      controller: _controller,
      physics: const NeverScrollableScrollPhysics(),
      itemCount: widget.children.length,
      itemBuilder: (context, index) =>
          _KeepAlivePage(key: ValueKey(index), child: widget.children[index]),
    );
  }
}

class _KeepAlivePage extends StatefulWidget {
  const _KeepAlivePage({required this.child, super.key});

  final Widget child;

  @override
  State<_KeepAlivePage> createState() => _KeepAlivePageState();
}

class _KeepAlivePageState extends State<_KeepAlivePage>
    with AutomaticKeepAliveClientMixin<_KeepAlivePage> {
  @override
  bool get wantKeepAlive => true;

  @override
  Widget build(BuildContext context) {
    super.build(context);
    return widget.child;
  }
}
