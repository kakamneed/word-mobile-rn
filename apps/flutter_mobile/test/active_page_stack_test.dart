import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_mobile/widgets/active_page_stack.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('window resize only lays out the active page', (tester) async {
    final layoutCounts = [0, 0, 0];

    Widget buildStack(int index) {
      return MaterialApp(
        home: ActivePageStack(
          index: index,
          children: [
            for (var page = 0; page < layoutCounts.length; page += 1)
              _LayoutProbe(
                onLayout: () => layoutCounts[page] += 1,
                child: ColoredBox(color: Colors.primaries[page]),
              ),
          ],
        ),
      );
    }

    await tester.pumpWidget(buildStack(0));
    final countsBeforeResize = List<int>.of(layoutCounts);

    tester.view.physicalSize = const Size(800, 900);
    addTearDown(tester.view.resetPhysicalSize);
    await tester.pump();

    expect(layoutCounts[0], greaterThan(countsBeforeResize[0]));
    expect(layoutCounts[1], countsBeforeResize[1]);
    expect(layoutCounts[2], countsBeforeResize[2]);
  });

  testWidgets('visited pages keep their state across index changes', (
    tester,
  ) async {
    Widget buildStack(int index) {
      return MaterialApp(
        home: ActivePageStack(
          index: index,
          children: const [
            _StatefulPage(key: ValueKey('first'), label: 'first'),
            _StatefulPage(key: ValueKey('second'), label: 'second'),
          ],
        ),
      );
    }

    await tester.pumpWidget(buildStack(0));
    await tester.tap(find.text('first: 0'));
    await tester.pump();
    expect(find.text('first: 1'), findsOneWidget);

    await tester.pumpWidget(buildStack(1));
    await tester.pump();
    expect(find.text('second: 0'), findsOneWidget);

    await tester.pumpWidget(buildStack(0));
    await tester.pump();
    expect(find.text('first: 1'), findsOneWidget);
  });
}

class _LayoutProbe extends SingleChildRenderObjectWidget {
  const _LayoutProbe({required this.onLayout, required super.child});

  final VoidCallback onLayout;

  @override
  RenderObject createRenderObject(BuildContext context) {
    return _RenderLayoutProbe(onLayout);
  }

  @override
  void updateRenderObject(
    BuildContext context,
    covariant _RenderLayoutProbe renderObject,
  ) {
    renderObject.onLayout = onLayout;
  }
}

class _RenderLayoutProbe extends RenderProxyBox {
  _RenderLayoutProbe(this.onLayout);

  VoidCallback onLayout;

  @override
  void performLayout() {
    onLayout();
    super.performLayout();
  }
}

class _StatefulPage extends StatefulWidget {
  const _StatefulPage({required this.label, super.key});

  final String label;

  @override
  State<_StatefulPage> createState() => _StatefulPageState();
}

class _StatefulPageState extends State<_StatefulPage> {
  var _count = 0;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: TextButton(
        onPressed: () => setState(() => _count += 1),
        child: Text('${widget.label}: $_count'),
      ),
    );
  }
}
