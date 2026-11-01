import 'package:flutter/material.dart';

/// Color pairs used only by the connection manager / acceptance window.
/// Do not install this scheme globally: navigation keeps its blue/green colors.
class ServerPageColors {
  ServerPageColors(ThemeData theme)
      : _surfaceScheme = theme.colorScheme,
        _actionScheme = ColorScheme.fromSeed(
          // KomiRemote's semantic accent is secondary; primary is navigation blue.
          seedColor: theme.colorScheme.secondary,
          brightness: theme.brightness,
        ),
        _elevateScheme = ColorScheme.fromSeed(
          // Amber: "accept with privileges" must never look like plain accept.
          seedColor: elevateSeed,
          brightness: theme.brightness,
        );

  static const Color elevateSeed = Color(0xFFF59E0B);

  final ColorScheme _surfaceScheme;
  final ColorScheme _actionScheme;
  final ColorScheme _elevateScheme;

  Color get surface => _surfaceScheme.surface;
  Color get onSurface => _surfaceScheme.onSurface;
  Color get outline => _surfaceScheme.outline;
  Color get shadow => _surfaceScheme.shadow;
  Color get acceptBackground => _actionScheme.primary;
  Color get acceptForeground => _actionScheme.onPrimary;
  Color get elevateBackground => _elevateScheme.primary;
  Color get elevateForeground => _elevateScheme.onPrimary;
  Color get rejectBackground => _surfaceScheme.error;
  Color get rejectForeground => _surfaceScheme.onError;
  Color get cancelForeground => _surfaceScheme.onSurface;
  Color get headerBackground => _actionScheme.primaryContainer;
  Color get headerForeground => _actionScheme.onPrimaryContainer;

  Color permissionBackground(bool enabled, bool canModify) => enabled
      ? (canModify ? _actionScheme.primary : _actionScheme.primaryContainer)
      : _surfaceScheme.surfaceVariant;

  // Read-only permissions still communicate an ON/OFF state. Keep them legible
  // with container/on-container pairs, not opacity over an unknown background.
  Color permissionForeground(bool enabled, bool canModify) => enabled
      ? (canModify ? _actionScheme.onPrimary : _actionScheme.onPrimaryContainer)
      : _surfaceScheme.onSurfaceVariant;
}
