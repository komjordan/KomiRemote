import 'package:flutter/material.dart';

/// KomiRemote: how long a received controller name stays visible without a
/// new cursor position carrying it (the host stops sending it after 3 s).
const Duration kPeerCursorNameTagTtl = Duration(seconds: 3);
const int kPeerCursorNameMaxChars = 32;

/// Offset of the tag from the cursor hotspot, below-right of the arrow.
const Offset kPeerCursorNameTagOffset = Offset(14, 18);

/// Dark green fill with white text: legible over any remote image, in both
/// light and dark themes.
const Color kPeerCursorNameTagBackground = Color(0xE61B5E3F);
const Color kPeerCursorNameTagForeground = Colors.white;

/// Accept only a plain string; strip control characters, collapse spaces and
/// cap the length.
String sanitizePeerControllerName(Object? raw) {
  if (raw is! String) return '';
  final cleaned = raw
      .replaceAll(RegExp(r'[\u0000-\u001F\u007F-\u009F]'), ' ')
      .split(RegExp(r'\s+'))
      .where((part) => part.isNotEmpty)
      .join(' ');
  final runes = cleaned.runes.toList();
  if (runes.length <= kPeerCursorNameMaxChars) return cleaned;
  return String.fromCharCodes(runes.take(kPeerCursorNameMaxChars));
}

/// Paints a small label with [name] next to [hotspot] (screen coordinates in
/// the remote view). Purely visual: callers wrap it in [IgnorePointer].
class PeerCursorNameTagPainter extends CustomPainter {
  final String name;
  final Offset hotspot;

  PeerCursorNameTagPainter({required this.name, required this.hotspot});

  @override
  void paint(Canvas canvas, Size size) {
    if (name.isEmpty) return;
    final text = TextPainter(
      text: TextSpan(
        text: name,
        style: const TextStyle(
          color: kPeerCursorNameTagForeground,
          fontSize: 12,
          fontWeight: FontWeight.w600,
          height: 1.2,
        ),
      ),
      textDirection: TextDirection.ltr,
      maxLines: 1,
      ellipsis: '\u2026',
    )..layout(maxWidth: 200);
    final origin = hotspot + kPeerCursorNameTagOffset;
    final rect = RRect.fromRectAndRadius(
      Rect.fromLTWH(origin.dx, origin.dy, text.width + 12, text.height + 6),
      const Radius.circular(4),
    );
    canvas.drawRRect(rect, Paint()..color = kPeerCursorNameTagBackground);
    canvas.drawRRect(
      rect,
      Paint()
        ..style = PaintingStyle.stroke
        ..strokeWidth = 1
        ..color = Colors.white.withOpacity(0.8),
    );
    text.paint(canvas, origin + const Offset(6, 3));
    text.dispose();
  }

  @override
  bool shouldRepaint(PeerCursorNameTagPainter oldDelegate) =>
      oldDelegate.name != name || oldDelegate.hotspot != hotspot;
}

/// Overlay showing who else is moving the remote cursor. Never takes input.
class PeerCursorNameTag extends StatelessWidget {
  final String name;
  final Offset hotspot;

  const PeerCursorNameTag(
      {super.key, required this.name, required this.hotspot});

  @override
  Widget build(BuildContext context) {
    if (name.isEmpty) return const SizedBox.shrink();
    return IgnorePointer(
      child: CustomPaint(
        painter: PeerCursorNameTagPainter(name: name, hotspot: hotspot),
      ),
    );
  }
}
