import 'package:flutter/material.dart';

class RemoteToolbarCompactControls extends StatelessWidget {
  // Same footprint as the expanded toolbar's buttons: 32x32 targets with
  // 2px (main axis) / 6px (cross axis) spacing.
  static const double hitTargetSize = 32;
  static const double iconSize = 20;
  static const double mainAxisInset = 4;
  static const double crossAxisInset = 6;
  static const double gap = 4;
  static const double mainAxisExtent =
      mainAxisInset * 2 + hitTargetSize * 4 + gap * 3; // 148
  static const double crossAxisExtent =
      crossAxisInset * 2 + hitTargetSize; // 44

  static const ValueKey<String> moveKey =
      ValueKey<String>('remote-toolbar-mini-move');
  static const ValueKey<String> fullscreenKey =
      ValueKey<String>('remote-toolbar-mini-fullscreen');
  static const ValueKey<String> expandKey =
      ValueKey<String>('remote-toolbar-mini-expand');
  static const ValueKey<String> hangUpKey =
      ValueKey<String>('remote-toolbar-mini-hangup');
  static const List<Key> actionKeys = <Key>[
    moveKey,
    fullscreenKey,
    expandKey,
    hangUpKey,
  ];

  final bool isHorizontal;
  final Widget moveHandle;
  final Widget fullscreenButton;
  final Widget expandButton;
  final Widget hangUpButton;

  const RemoteToolbarCompactControls({
    super.key,
    required this.isHorizontal,
    required this.moveHandle,
    required this.fullscreenButton,
    required this.expandButton,
    required this.hangUpButton,
  });

  @override
  Widget build(BuildContext context) {
    // Four 32px controls + three 4px gaps + two 4px insets (148) by
    // 32px + two 6px insets (44), matching the expanded toolbar's height.
    // Own the actual dimensions; no MenuBar or scroll view inside a control.
    return SizedBox(
      width: isHorizontal ? mainAxisExtent : crossAxisExtent,
      height: isHorizontal ? crossAxisExtent : mainAxisExtent,
      child: Padding(
        padding: isHorizontal
            ? const EdgeInsets.symmetric(
                horizontal: mainAxisInset, vertical: crossAxisInset)
            : const EdgeInsets.symmetric(
                horizontal: crossAxisInset, vertical: mainAxisInset),
        child: Flex(
          direction: isHorizontal ? Axis.horizontal : Axis.vertical,
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.center,
          children: <Widget>[
            _control(moveKey, moveHandle),
            const SizedBox(width: gap, height: gap),
            _control(fullscreenKey, fullscreenButton),
            const SizedBox(width: gap, height: gap),
            _control(expandKey, expandButton),
            const SizedBox(width: gap, height: gap),
            _control(hangUpKey, hangUpButton),
          ],
        ),
      ),
    );
  }

  Widget _control(Key key, Widget child) => SizedBox(
        key: key,
        width: hitTargetSize,
        height: hitTargetSize,
        child: child,
      );
}

/// A menu-free, fixed-size control shared by compact and expanded toolbars.
/// Keeping the action itself at 32px (like the toolbar's other buttons)
/// avoids MenuBar's minimum intrinsic width.
class RemoteToolbarActionButton extends StatelessWidget {
  /// White glyphs stay legible on the green and red toolbar fills in both
  /// light and dark themes (onSecondary/onError can be black there).
  static const Color defaultForeground = Colors.white;

  final String tooltip;
  final Widget child;
  final VoidCallback? onPressed;
  final Color backgroundColor;
  final Color hoverColor;
  final Color? foregroundColor;
  final EdgeInsetsGeometry margin;

  const RemoteToolbarActionButton({
    super.key,
    required this.tooltip,
    required this.child,
    required this.onPressed,
    required this.backgroundColor,
    required this.hoverColor,
    this.foregroundColor,
    this.margin = EdgeInsets.zero,
  });

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final foreground = foregroundColor ?? defaultForeground;
    const size = Size.square(RemoteToolbarCompactControls.hitTargetSize);
    final button = Tooltip(
      message: tooltip,
      child: SizedBox.square(
        dimension: RemoteToolbarCompactControls.hitTargetSize,
        child: TextButton(
          onPressed: onPressed,
          style: ButtonStyle(
            minimumSize: const WidgetStatePropertyAll(size),
            maximumSize: const WidgetStatePropertyAll(size),
            padding: const WidgetStatePropertyAll(EdgeInsets.zero),
            tapTargetSize: MaterialTapTargetSize.shrinkWrap,
            visualDensity: VisualDensity.standard,
            shape: WidgetStatePropertyAll(
                RoundedRectangleBorder(borderRadius: BorderRadius.circular(8))),
            foregroundColor: WidgetStateProperty.resolveWith((states) =>
                states.contains(WidgetState.disabled)
                    ? scheme.onSurface.withOpacity(0.38)
                    : foreground),
            backgroundColor: WidgetStateProperty.resolveWith((states) {
              if (states.contains(WidgetState.disabled)) {
                return scheme.onSurface.withOpacity(0.12);
              }
              return states.contains(WidgetState.hovered) ||
                      states.contains(WidgetState.pressed)
                  ? hoverColor
                  : backgroundColor;
            }),
            // The hover fill itself is the feedback; no extra tinted overlay.
            overlayColor: const WidgetStatePropertyAll(Colors.transparent),
          ),
          child: IconTheme.merge(
            data: const IconThemeData(
                size: RemoteToolbarCompactControls.iconSize),
            child: child,
          ),
        ),
      ),
    );
    if (margin == EdgeInsets.zero) return button;
    return Padding(padding: margin, child: button);
  }
}

bool shouldAutoCollapseRemoteToolbar({
  required bool expanded,
  required bool pinned,
  required bool dragging,
  required bool pointerOverToolbar,
  required bool cursorOverImage,
  required int openMenuCount,
}) {
  return expanded &&
      !pinned &&
      !dragging &&
      !pointerOverToolbar &&
      cursorOverImage &&
      openMenuCount == 0;
}

class RemoteToolbarMenuActivity {
  final VoidCallback onIdle;
  int _openMenuCount = 0;

  RemoteToolbarMenuActivity({required this.onIdle});

  int get openMenuCount => _openMenuCount;

  void menuOpened() {
    _openMenuCount++;
  }

  void menuClosed() {
    if (_openMenuCount == 0) return;
    _openMenuCount--;
    if (_openMenuCount == 0) onIdle();
  }
}
