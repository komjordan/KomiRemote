import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_hbb/desktop/pages/server_page_theme.dart';

double contrast(Color a, Color b) {
  final x = a.computeLuminance();
  final y = b.computeLuminance();
  return ((x > y ? x : y) + .05) / ((x > y ? y : x) + .05);
}

void main() {
  for (final brightness in Brightness.values) {
    // Mirrors the existing KomiRemote blue primary / green secondary, without
    // importing common.dart (which initializes native/session dependencies).
    final theme = ThemeData(
      brightness: brightness,
      colorScheme: ColorScheme.fromSeed(
        seedColor: Colors.blue,
        brightness: brightness,
      ).copyWith(primary: Colors.blue, secondary: const Color(0xFF4FBB85)),
    );
    final colors = ServerPageColors(theme);

    test('acceptance text and icons have AA contrast in $brightness', () {
      expect(contrast(colors.acceptBackground, colors.acceptForeground),
          greaterThanOrEqualTo(4.5));
      expect(contrast(colors.rejectBackground, colors.rejectForeground),
          greaterThanOrEqualTo(4.5));
      expect(contrast(colors.surface, colors.cancelForeground),
          greaterThanOrEqualTo(4.5));
      expect(contrast(colors.headerBackground, colors.headerForeground),
          greaterThanOrEqualTo(4.5));
      expect(contrast(colors.elevateBackground, colors.elevateForeground),
          greaterThanOrEqualTo(4.5));
      // Derivation is scoped: the existing navigation palette is untouched.
      expect(theme.colorScheme.primary, Colors.blue);
      expect(theme.colorScheme.secondary, const Color(0xFF4FBB85));
    });

    test('accept with privileges is clearly distinct from accept in $brightness',
        () {
      final accept = HSVColor.fromColor(colors.acceptBackground);
      final elevate = HSVColor.fromColor(colors.elevateBackground);
      var hueDelta = (accept.hue - elevate.hue).abs();
      if (hueDelta > 180) hueDelta = 360 - hueDelta;
      expect(colors.elevateBackground, isNot(colors.acceptBackground));
      expect(hueDelta, greaterThanOrEqualTo(60));
    });

    for (final enabled in [true, false]) {
      for (final canModify in [true, false]) {
        testWidgets(
            'camera/keyboard contrast $brightness enabled=$enabled '
            'canModify=$canModify', (tester) async {
          var taps = 0;
          final foreground = colors.permissionForeground(enabled, canModify);
          final background = colors.permissionBackground(enabled, canModify);
          expect(contrast(background, foreground), greaterThanOrEqualTo(4.5));
          await tester.pumpWidget(MaterialApp(
            theme: theme,
            home: Scaffold(
              body: Container(
                color: background,
                child: InkWell(
                  onTap: canModify ? () => taps++ : null,
                  child: Row(children: [
                    Icon(Icons.videocam_rounded, color: foreground),
                    Icon(Icons.keyboard, color: foreground),
                  ]),
                ),
              ),
            ),
          ));
          expect(tester.widget<Icon>(find.byIcon(Icons.videocam_rounded)).color,
              foreground);
          expect(tester.widget<Icon>(find.byIcon(Icons.keyboard)).color,
              foreground);
          await tester.tap(find.byIcon(Icons.videocam_rounded));
          expect(taps, canModify ? 1 : 0);
          expect(tester.takeException(), isNull);
        });
      }
    }
  }
}
