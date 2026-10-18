// KomiRemote: lightweight "Matrix" rain painted behind the home page panels.
// Visual only: no network, config or session logic lives here.
import 'dart:async';
import 'dart:math';

import 'package:flutter/material.dart';
import 'package:flutter_hbb/common.dart';
import 'package:get/get.dart';

/// Local (per-user) option. Unset means enabled; "N" means disabled.
const String kOptionKomiMatrixAnimation = 'komi-matrix-animation';

bool get isKomiRemote => appName == 'KomiRemote';

final RxBool komiMatrixEnabled = RxBool(true);
bool _komiMatrixLoaded = false;

void loadKomiMatrixOption() {
  if (_komiMatrixLoaded) return;
  _komiMatrixLoaded = true;
  komiMatrixEnabled.value =
      mainGetLocalBoolOptionSync(kOptionKomiMatrixAnimation);
}

Future<void> setKomiMatrixEnabled(bool enabled) async {
  await mainSetLocalBoolOption(kOptionKomiMatrixAnimation, enabled);
  komiMatrixEnabled.value =
      mainGetLocalBoolOptionSync(kOptionKomiMatrixAnimation);
}

class MatrixRain extends StatefulWidget {
  /// Returns false when the hosting page is not on screen (e.g. another tab).
  final bool Function()? isActive;
  const MatrixRain({Key? key, this.isActive}) : super(key: key);

  @override
  State<MatrixRain> createState() => _MatrixRainState();
}

class _MatrixRainState extends State<MatrixRain> with WidgetsBindingObserver {
  // ~12 fps: enough for a calm effect, far below the display refresh rate.
  static const Duration _frame = Duration(milliseconds: 80);
  static const Duration _idlePoll = Duration(milliseconds: 500);

  final ValueNotifier<int> _repaint = ValueNotifier<int>(0);
  final _Rain _rain = _Rain();
  Timer? _timer;
  bool _tickerEnabled = true;
  bool _appVisible = true;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    _schedule();
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _tickerEnabled = TickerMode.of(context);
    _schedule();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    // hidden/paused: minimized or not shown -> stop animating.
    _appVisible = state == AppLifecycleState.resumed ||
        state == AppLifecycleState.inactive;
    _schedule();
  }

  bool get _active =>
      mounted &&
      _tickerEnabled &&
      _appVisible &&
      (widget.isActive?.call() ?? true);

  void _schedule() {
    _timer?.cancel();
    _timer = null;
    if (!mounted) return;
    final active = _active;
    _timer = Timer(active ? _frame : _idlePoll, () {
      if (!mounted) return;
      if (_active) {
        _rain.step();
        _repaint.value++;
      }
      _schedule();
    });
  }

  @override
  void dispose() {
    _timer?.cancel();
    WidgetsBinding.instance.removeObserver(this);
    _repaint.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return IgnorePointer(
      child: ExcludeSemantics(
        child: RepaintBoundary(
          child: CustomPaint(
            painter: _MatrixPainter(_rain, _repaint),
            size: Size.infinite,
          ),
        ),
      ),
    );
  }
}

class _Drop {
  double head;
  double speed;
  int trail;
  _Drop(this.head, this.speed, this.trail);
}

class _Rain {
  static const String chars =
      'ｱｲｳｴｵｶｷｸｹｺｻｼｽｾｿﾀﾁﾂﾃﾄﾅﾆﾇﾈﾉﾊﾋﾌﾍﾎﾏﾐﾑﾒﾓﾔﾕﾖﾗﾘﾙﾚﾛﾜ0123456789ABCDEF<>=+*:';
  static const double cell = 16.0;
  final Random _rnd = Random();
  List<_Drop> drops = [];
  List<List<int>> glyphs = [];
  int rows = 0;

  void ensure(Size size) {
    final cols = max(1, (size.width / cell).ceil());
    final r = max(1, (size.height / cell).ceil());
    if (cols == drops.length && r == rows) return;
    rows = r;
    drops = List.generate(cols, (_) => _newDrop(initial: true));
    glyphs = List.generate(
        cols, (_) => List.generate(rows, (_) => _rnd.nextInt(chars.length)));
  }

  _Drop _newDrop({bool initial = false}) => _Drop(
        initial ? -_rnd.nextDouble() * rows * 1.5 : -_rnd.nextDouble() * 12,
        0.25 + _rnd.nextDouble() * 0.55,
        8 + _rnd.nextInt(14),
      );

  void step() {
    for (var c = 0; c < drops.length; c++) {
      final d = drops[c];
      d.head += d.speed;
      if (d.head - d.trail > rows) drops[c] = _newDrop();
      if (_rnd.nextInt(10) == 0 && rows > 0) {
        glyphs[c][_rnd.nextInt(rows)] = _rnd.nextInt(chars.length);
      }
    }
  }
}

class _MatrixPainter extends CustomPainter {
  static const int _levels = 6;
  static final Map<int, TextPainter> _cache = {};
  final _Rain rain;

  _MatrixPainter(this.rain, Listenable repaint) : super(repaint: repaint);

  static TextPainter _glyph(int charIndex, int level) {
    final key = charIndex * (_levels + 1) + level;
    return _cache.putIfAbsent(key, () {
      final Color color = level == _levels
          ? const Color(0xFFC8FFE0) // bright head
          : const Color(0xFF4FBB85)
              .withOpacity(0.08 + 0.42 * (level / (_levels - 1)));
      return TextPainter(
        text: TextSpan(
          text: _Rain.chars[charIndex],
          style: TextStyle(
              color: color, fontSize: 13, fontFamily: 'Consolas', height: 1.0),
        ),
        textDirection: TextDirection.ltr,
      )..layout();
    });
  }

  @override
  void paint(Canvas canvas, Size size) {
    rain.ensure(size);
    for (var c = 0; c < rain.drops.length; c++) {
      final d = rain.drops[c];
      final headRow = d.head.floor();
      for (var i = 0; i < d.trail; i++) {
        final row = headRow - i;
        if (row < 0 || row >= rain.rows) continue;
        final level = i == 0
            ? _levels
            : ((_levels - 1) * (1 - i / d.trail)).round().clamp(0, _levels - 1);
        _glyph(rain.glyphs[c][row], level)
            .paint(canvas, Offset(c * _Rain.cell + 2, row * _Rain.cell));
      }
    }
  }

  @override
  bool shouldRepaint(covariant _MatrixPainter oldDelegate) => false;
}
