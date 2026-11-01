import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_hbb/common/widgets/peer_cursor_name_tag.dart';

double contrast(Color a, Color b) {
  final x = Color.alphaBlend(a, Colors.black).computeLuminance();
  final y = b.computeLuminance();
  return ((x > y ? x : y) + .05) / ((x > y ? y : x) + .05);
}

void main() {
  test('controller names are sanitized and capped', () {
    expect(sanitizePeerControllerName(null), '');
    expect(sanitizePeerControllerName(42), '');
    expect(sanitizePeerControllerName('  kom \n\t x '), 'kom x');
    expect(sanitizePeerControllerName('\u0007'), '');
    expect(sanitizePeerControllerName('é' * 80).runes.length,
        kPeerCursorNameMaxChars);
  });

  test('tag text is legible', () {
    expect(contrast(kPeerCursorNameTagBackground, kPeerCursorNameTagForeground),
        greaterThanOrEqualTo(4.5));
  });

  testWidgets('tag never takes pointer input and hides when empty',
      (tester) async {
    var taps = 0;
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(
        body: Stack(fit: StackFit.expand, children: [
          Positioned.fill(
              child: GestureDetector(
                  behavior: HitTestBehavior.opaque,
                  onTap: () => taps++)),
          const PeerCursorNameTag(name: 'kom', hotspot: Offset(100, 100)),
        ]),
      ),
    ));
    expect(find.byType(IgnorePointer), findsWidgets);
    expect(
        find.descendant(
            of: find.byType(PeerCursorNameTag),
            matching: find.byType(IgnorePointer)),
        findsOneWidget);
    await tester.tapAt(const Offset(120, 125));
    expect(taps, 1);
    await tester.pumpWidget(const MaterialApp(
        home: PeerCursorNameTag(name: '', hotspot: Offset.zero)));
    expect(find.byType(CustomPaint).evaluate().where((e) =>
        (e.widget as CustomPaint).painter is PeerCursorNameTagPainter), isEmpty);
    expect(tester.takeException(), isNull);
  });
}
