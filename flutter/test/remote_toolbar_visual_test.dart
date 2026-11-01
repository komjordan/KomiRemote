import 'dart:io';
import 'dart:ui' as ui;
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_hbb/desktop/widgets/remote_toolbar_compact.dart';

void main() {
  setUpAll(() async {
    final font = FontLoader('MaterialIcons');
    font.addFont(rootBundle.load('fonts/MaterialIcons-Regular.otf'));
    await font.load();
  });
  for (final brightness in [Brightness.light, Brightness.dark]) {
    testWidgets('compact controls visual ${brightness.name}', (tester) async {
      final key = GlobalKey();
      final scheme = brightness == Brightness.light
          ? const ColorScheme.light(secondary: Color(0xff4fbb85))
          : const ColorScheme.dark(secondary: Color(0xff4fbb85));
      Widget action(String tip, IconData icon, {bool red = false}) =>
          RemoteToolbarActionButton(
              tooltip: tip,
              onPressed: () {},
              backgroundColor: red ? const Color(0xffff5252) : scheme.secondary,
              hoverColor:
                  red ? const Color(0xffc62828) : const Color(0xff2e8b57),
              child: Icon(icon, size: 20));
      await tester.pumpWidget(MaterialApp(
          theme: ThemeData(colorScheme: scheme),
          home: Scaffold(
              body: Center(
                  child: RepaintBoundary(
            key: key,
            child: ColoredBox(
                color: scheme.surface,
                child: Padding(
                  padding: const EdgeInsets.all(12),
                  child: RemoteToolbarCompactControls(
                      isHorizontal: true,
                      moveHandle: action('Move', Icons.drag_indicator),
                      fullscreenButton: action('Fullscreen', Icons.fullscreen),
                      expandButton: action('Expand', Icons.expand_more),
                      hangUpButton: action('Hang up', Icons.close, red: true)),
                )),
          )))));
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      expect(find.byType(Scrollable), findsNothing);
      await tester.runAsync(() async {
        final image = await (key.currentContext!.findRenderObject()
                as RenderRepaintBoundary)
            .toImage(pixelRatio: 3);
        final bytes = await image.toByteData(format: ui.ImageByteFormat.png);
        await File('C:/kbuild/logs/komi14-toolbar-${brightness.name}.png')
            .writeAsBytes(bytes!.buffer.asUint8List());
        image.dispose();
      });
    });
  }
  testWidgets('move action still participates in dragging', (tester) async {
    var started = 0;
    await tester.pumpWidget(MaterialApp(
        home: Scaffold(
            body: Center(
      child: Draggable<int>(
        data: 1,
        onDragStarted: () => started++,
        feedback: const SizedBox(width: 32, height: 32),
        child: RemoteToolbarActionButton(
            tooltip: 'Move',
            onPressed: () {},
            backgroundColor: Colors.green,
            hoverColor: Colors.greenAccent,
            child: const Icon(Icons.drag_indicator)),
      ),
    ))));
    await tester.drag(
        find.byType(RemoteToolbarActionButton), const Offset(90, 0));
    await tester.pumpAndSettle();
    expect(started, 1);
    expect(tester.takeException(), isNull);
  });
}
