import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_hbb/desktop/widgets/remote_toolbar_compact.dart';

void main() {
  for (final horizontal in [true, false]) {
    testWidgets('four real action buttons fit a compact bar ($horizontal)',
        (tester) async {
      var taps = 0;
      Widget action(IconData icon) => RemoteToolbarActionButton(
            tooltip: 'Action',
            backgroundColor: Colors.green,
            hoverColor: Colors.greenAccent,
            onPressed: () => taps++,
            child: Icon(icon),
          );
      await tester.pumpWidget(MaterialApp(
        home: Scaffold(
          body: Center(
            child: RemoteToolbarCompactControls(
              isHorizontal: horizontal,
              moveHandle: action(Icons.drag_indicator),
              fullscreenButton: action(Icons.fullscreen),
              expandButton: action(Icons.expand_more),
              hangUpButton: action(Icons.close),
            ),
          ),
        ),
      ));
      final bar = find.byType(RemoteToolbarCompactControls);
      expect(tester.getSize(bar),
          horizontal ? const Size(148, 44) : const Size(44, 148));
      for (final key in RemoteToolbarCompactControls.actionKeys) {
        expect(tester.getSize(find.byKey(key)), const Size(32, 32));
        await tester.tap(find.byKey(key));
      }
      expect(taps, 4);
      expect(find.byType(Scrollable), findsNothing);
      expect(find.byType(MenuBar), findsNothing);
      expect(tester.takeException(), isNull);
    });
  }
  testWidgets('disabled action does not execute its callback', (tester) async {
    await tester.pumpWidget(const MaterialApp(
        home: Scaffold(
      body: RemoteToolbarActionButton(
        tooltip: 'Unavailable',
        backgroundColor: Colors.green,
        hoverColor: Colors.greenAccent,
        onPressed: null,
        child: Icon(Icons.fullscreen),
      ),
    )));
    expect(
        tester.widget<TextButton>(find.byType(TextButton)).onPressed, isNull);
    expect(tester.getSize(find.byType(TextButton)), const Size(32, 32));
    expect(tester.takeException(), isNull);
  });
  testWidgets('margin matches the main toolbar buttons and glyphs are white',
      (tester) async {
    await tester.pumpWidget(MaterialApp(
        theme: ThemeData(
            colorScheme: const ColorScheme.light(
                secondary: Color(0xff4fbb85), onSecondary: Colors.black)),
        home: const Scaffold(
          body: Center(
            child: RemoteToolbarActionButton(
              tooltip: 'Fullscreen',
              backgroundColor: Color(0xff4fbb85),
              hoverColor: Color(0xff2e8b57),
              margin: EdgeInsets.symmetric(horizontal: 2, vertical: 6),
              onPressed: null,
              child: Icon(Icons.fullscreen),
            ),
          ),
        )));
    expect(tester.getSize(find.byType(RemoteToolbarActionButton)),
        const Size(36, 44));
    expect(tester.getSize(find.byType(TextButton)), const Size(32, 32));
    expect(RemoteToolbarActionButton.defaultForeground, Colors.white);
    expect(tester.takeException(), isNull);
  });
  testWidgets('hover fill differs from the base fill', (tester) async {
    const base = Color(0xff4fbb85);
    const hover = Color(0xff2e8b57);
    await tester.pumpWidget(MaterialApp(
        home: Scaffold(
      body: Center(
        child: RemoteToolbarActionButton(
          tooltip: 'Expand',
          backgroundColor: base,
          hoverColor: hover,
          onPressed: () {},
          child: const Icon(Icons.expand_more),
        ),
      ),
    )));
    final style = tester.widget<TextButton>(find.byType(TextButton)).style!;
    expect(style.backgroundColor!.resolve(<WidgetState>{}), base);
    expect(style.backgroundColor!.resolve({WidgetState.hovered}), hover);
    expect(style.foregroundColor!.resolve(<WidgetState>{}), Colors.white);
    expect(tester.takeException(), isNull);
  });
}
