import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_hbb/desktop/widgets/remote_toolbar_compact.dart';

void main() {
  testWidgets('compact toolbar has four ordered 32px controls', (tester) async {
    const controls = RemoteToolbarCompactControls.actionKeys;
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(
        body: Center(
          child: RemoteToolbarCompactControls(
            isHorizontal: true,
            moveHandle: const Icon(Icons.drag_indicator),
            fullscreenButton: const Icon(Icons.fullscreen),
            expandButton: const Icon(Icons.expand_more),
            hangUpButton: const Icon(Icons.call_end),
          ),
        ),
      ),
    ));

    expect(controls, hasLength(4));
    final positions = <double>[];
    for (final key in controls) {
      expect(find.byKey(key), findsOneWidget);
      expect(tester.getSize(find.byKey(key)), const Size(32, 32));
      positions.add(tester.getCenter(find.byKey(key)).dx);
    }
    expect(positions, orderedEquals(positions.toList()..sort()));
    expect(RemoteToolbarCompactControls.iconSize, 20);
    expect(tester.getSize(find.byType(RemoteToolbarCompactControls)),
        const Size(148, 44));
    expect(RemoteToolbarCompactControls.crossAxisExtent, 44);
  });

  test('auto-collapse requires an idle, unpinned expanded toolbar on the view',
      () {
    bool allowed({
      bool pinned = false,
      bool dragging = false,
      bool pointerOverToolbar = false,
      bool cursorOverImage = true,
      int openMenuCount = 0,
      bool expanded = true,
    }) =>
        shouldAutoCollapseRemoteToolbar(
          expanded: expanded,
          pinned: pinned,
          dragging: dragging,
          pointerOverToolbar: pointerOverToolbar,
          cursorOverImage: cursorOverImage,
          openMenuCount: openMenuCount,
        );

    expect(allowed(), isTrue);
    expect(allowed(pinned: true), isFalse);
    expect(allowed(dragging: true), isFalse);
    expect(allowed(pointerOverToolbar: true), isFalse);
    expect(allowed(cursorOverImage: false), isFalse);
    expect(allowed(openMenuCount: 1), isFalse);
    expect(allowed(expanded: false), isFalse);
  });

  test('nested toolbar menus hold the auto-hide guard until all close', () {
    var idleNotifications = 0;
    final activity =
        RemoteToolbarMenuActivity(onIdle: () => idleNotifications++);

    activity.menuOpened();
    activity.menuOpened();
    expect(activity.openMenuCount, 2);
    activity.menuClosed();
    expect(activity.openMenuCount, 1);
    expect(idleNotifications, 0);
    activity.menuClosed();
    expect(activity.openMenuCount, 0);
    expect(idleNotifications, 1);
    activity.menuClosed();
    expect(idleNotifications, 1);
  });
}
