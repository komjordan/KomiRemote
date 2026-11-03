import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:math';
import 'dart:typed_data';

import 'package:desktop_multi_window/desktop_multi_window.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:window_manager/window_manager.dart';
import 'package:window_size/window_size.dart' as window_size;

import '../../common.dart';
import '../../main.dart' show kWindowId;
import '../../models/platform_model.dart';

/// Fail closed when TCC is denied, restricted, unavailable or the channel fails.
/// Only user actions call this; receiving state/frame events never asks for access.
Future<bool> komiEnsureMediaPermission({bool camera = false, bool microphone = false}) async {
  if (!isMacOS) return true;
  try {
    if (camera) {
      final status = await kMacOSPermChannel.invokeMethod<int>('canRecordCamera');
      if (status == null || status < 0 ||
          (status == 0 && await kMacOSPermChannel.invokeMethod<bool>(
              'requestRecordCamera') != true)) {
        showToast('Caméra non autorisée. Vérifiez Réglages Système > Confidentialité et sécurité > Caméra.');
        return false;
      }
    }
    if (microphone) {
      final status = await osxCanRecordAudio();
      if (status == PermissionAuthorizeType.denied ||
          (status == PermissionAuthorizeType.undetermined && !await osxRequestAudio())) {
        showToast('Microphone non autorisé. Vérifiez Réglages Système > Confidentialité et sécurité > Microphone.');
        return false;
      }
    }
    return true;
  } catch (error) {
    debugPrint('Media permission unavailable: $error');
    showToast('Impossible de vérifier les autorisations caméra/microphone.');
    return false;
  }
}

/// Owned by the session/CM isolate, never by the rendering window. No global
/// singleton call: each connection has its own state, frames and OS window.
class KomiVideoCallUi extends ChangeNotifier {
  static final Map<String, KomiVideoCallUi> _calls = {};
  static const windowKind = 'komi-video-call-v23';
  static const rpcPrefix = 'komi-video/';

  static KomiVideoCallUi forSession(dynamic sessionId) => _calls.putIfAbsent(
      'session:$sessionId',
      () => KomiVideoCallUi._('session:$sessionId', (command) {
            bind.sessionGetCommonSync(
                sessionId: sessionId, key: 'komi_video_call', param: command);
          }));

  static KomiVideoCallUi forHost(int id) => _calls.putIfAbsent(
      'cm:$id',
      () => KomiVideoCallUi._('cm:$id', (cmd) {
            final name = cmd == 'accept' || cmd == 'refuse'
                ? 'komi-video-call-answer'
                : cmd == 'camera-on' || cmd == 'camera-off'
                    ? 'komi-video-camera'
                    : 'komi-video-call';
            return bind.cmSwitchPermission(
                connId: id,
                name: name,
                enabled:
                    cmd == 'request' || cmd == 'accept' || cmd == 'camera-on');
          }));

  static Future<void> releaseSession(dynamic id) async {
    await _calls.remove('session:$id')?._finish(sendHangup: true);
  }

  static Future<void> releaseHost(int id) async {
    await _calls.remove('cm:$id')?._finish(sendHangup: true);
  }

  final String key;
  final FutureOr<void> Function(String) _send;
  String phase = 'idle';
  String mode = 'video';
  bool localOn = false;
  bool remoteOn = false;
  String _localPath = '', _remotePath = '';
  int _localSeq = -1, _remoteSeq = -1;
  WindowController? _window;
  Future<void>? _opening;
  Timer? _lease;
  DateTime _lastPoll = DateTime.now();
  String _token = '';
  int _generation = 0;

  KomiVideoCallUi._(this.key, this._send);

  bool get visible => phase == 'outgoing' || phase == 'active';
  bool get inCall =>
      phase == 'outgoing' || phase == 'active' || phase == 'incoming';
  bool get showingCamera => mode == 'show-camera' && inCall;

  int _permissionEpoch = 0;
  String callId = '';

  Future<bool> command(String cmd) async {
    final epoch = ++_permissionEpoch;
    final expectedPhase = phase;
    final expectedCallId = callId;
    final camera = cmd == 'request' || cmd == 'show-mine' ||
        cmd == 'camera-on' || (cmd == 'accept' && mode != 'show-camera');
    final microphone = (cmd == 'request' || cmd == 'accept') &&
        !(cmd == 'accept' && mode == 'show-camera');
    if (camera || microphone) {
      if (!await komiEnsureMediaPermission(camera: camera, microphone: microphone)) {
        return false;
      }
      // TCC can remain open longer than the invitation. Never revive a closed,
      // replaced or disconnected call after a late OS permission response.
      if (epoch != _permissionEpoch || phase != expectedPhase ||
          callId != expectedCallId || !identical(_calls[key], this)) return false;
    }
    if (cmd == 'accept' && phase != 'incoming') return false;
    if (cmd == 'camera-on' && phase != 'active') return false;
    if (cmd == 'request' || cmd == 'show-mine') {
      mode = cmd == 'show-mine' ? 'show-camera' : 'video';
      // Open preview immediately. The engine must capture locally but MUST NOT
      // send camera frames until the peer explicitly accepts.
      apply('outgoing', true, false);
    }
    _dispatch(cmd);
    if (cmd == 'hangup' || cmd == 'refuse') {
      unawaited(_finish());
    }
    return true;
  }

  void _dispatch(String cmd) {
    unawaited(Future<void>.sync(() => _send(cmd)).catchError((Object e) {
      debugPrint('Video command $cmd failed: $e');
      unawaited(_finish());
    }));
  }

  void apply(String next, bool local, bool remote, {String? nextMode, String? nextCallId}) {
    if (nextCallId != null) callId = nextCallId;
    phase = next.isEmpty ? 'idle' : next;
    if (nextMode != null && nextMode.isNotEmpty) mode = nextMode;
    if (phase == 'idle' || phase == 'ended') mode = 'video';
    localOn = local;
    remoteOn = remote;
    if (!local) {
      _localPath = '';
      _localSeq = -1;
    }
    if (!remote) {
      _remotePath = '';
      _remoteSeq = -1;
    }
    if (phase == 'idle' || phase == 'ended') {
      unawaited(_finish());
      return;
    }
    if (visible) unawaited(_ensureWindow());
    notifyListeners();
  }

  void frame(String path, int seq, int stream) {
    if (!inCall || path.isEmpty) return;
    // Media contract: stream 0 = local preview, 1/2 = remote camera.
    if (stream == 0) {
      _localPath = path;
      _localSeq = seq;
    } else if (stream == 1 || stream == 2) {
      _remotePath = path;
      _remoteSeq = seq;
    }
  }

  Future<void> _ensureWindow() async {
    if (_window != null || _opening != null || !visible) return;
    final generation = _generation;
    _token =
        '${DateTime.now().microsecondsSinceEpoch}-${Random.secure().nextInt(1 << 32)}';
    final opening = _open(generation);
    _opening = opening;
    try {
      await opening;
    } catch (e) {
      debugPrint('Video window failed: $e');
      // A failed preview must not leave a hidden camera capturing indefinitely.
      if (generation == _generation) {
        await _finish(sendHangup: true);
        showToast('Impossible d’ouvrir la fenêtre vidéo. Appel interrompu.');
      }
    } finally {
      _opening = null;
      // End + a new request may have raced an outstanding createWindow.
      if (visible && _window == null && generation != _generation) {
        unawaited(_ensureWindow());
      }
    }
  }

  Future<void> _open(int generation) async {
    final window = await DesktopMultiWindow.createWindow(jsonEncode({
      'kind': windowKind,
      'owner': kWindowId ?? 0,
      'key': key,
      'token': _token,
    }));
    if (generation != _generation || !visible) {
      await window.close();
      return;
    }
    _window = window;
    await window.setTitle('Komi Remote — Vidéo');
    await window.showTitleBar(true);
    await window.setFrame(const Rect.fromLTWH(120, 180, 480, 320));
    if (key.startsWith('cm:')) {
      // Keep the acceptance/controls window above the independent video window.
      final owner = await windowManager.getBounds();
      final screen = await window_size.getCurrentScreen();
      final area = screen?.visibleFrame;
      final width = area == null ? 480.0 : min(480.0, area.width);
      final height = area == null ? 320.0 : min(320.0, area.height);
      final x = area == null
          ? owner.left
          : owner.left.clamp(area.left, area.right - width).toDouble();
      final y = area == null
          ? owner.bottom + 8
          : (owner.bottom + 8).clamp(area.top, area.bottom - height).toDouble();
      await window.setFrame(Rect.fromLTWH(x, y, width, height));
    } else {
      await window.center();
    }
    if (generation != _generation || !visible) return;
    await window.show();
    // Do not focus over an acceptance dialog or steal remote keyboard input.
    _lastPoll = DateTime.now();
    _lease?.cancel();
    _lease = Timer.periodic(const Duration(seconds: 2), (_) {
      if (DateTime.now().difference(_lastPoll) > const Duration(seconds: 12)) {
        unawaited(_finish(sendHangup: true));
      }
    });
  }

  Future<void> _finish(
      {bool sendHangup = false, bool closeWindow = true}) async {
    _permissionEpoch++;
    final wasInCall = inCall;
    phase = 'ended';
    localOn = remoteOn = false;
    _localPath = _remotePath = '';
    _localSeq = _remoteSeq = -1;
    _generation++;
    _token = '';
    _lease?.cancel();
    _lease = null;
    final window = _window;
    _window = null;
    if (sendHangup && wasInCall) _dispatch('hangup');
    notifyListeners();
    if (window != null && closeWindow) {
      try {
        await window.setPreventClose(false);
        await window.close();
      } catch (e) {
        debugPrint('Video window cleanup: $e');
      }
    }
  }

  /// Multiplex through the existing window handler, not a replacement handler.
  static Future<dynamic> handleRpc(MethodCall call, int fromWindowId) async {
    final args = Map<String, dynamic>.from(call.arguments as Map);
    final state = _calls[args['key']];
    // The child engine can start polling before createWindow returns its ID.
    // No frame paths are returned until the native window ID is registered.
    if (state != null &&
        state._token == args['token'] &&
        state._window == null &&
        state.visible) {
      return <String, dynamic>{'waiting': true};
    }
    if (state == null ||
        state._token != args['token'] ||
        state._window?.windowId != fromWindowId ||
        !state.visible) {
      return <String, dynamic>{'ended': true};
    }
    if (call.method == '${rpcPrefix}close') {
      // Native close ends the call, not the remote-control session.
      unawaited(state._finish(sendHangup: true, closeWindow: false));
      return true;
    }
    state._lastPoll = DateTime.now();
    return <String, dynamic>{
      'phase': state.phase,
      'localPath': state._localPath, 'localSeq': state._localSeq,
      // Never disclose remote frames to an unaccepted incoming/outgoing call.
      'remotePath': state.phase == 'active' ? state._remotePath : '',
      'remoteSeq': state._remoteSeq,
    };
  }
}

/// Dedicated engine/window entry point: no session FFI, no control cards.
Future<void> runKomiVideoWindow(Map<String, dynamic> args, int id) async {
  final controller = WindowController.fromWindowId(id);
  await controller.showTitleBar(true);
  await controller.setPreventClose(true);
  runApp(MaterialApp(
    debugShowCheckedModeBanner: false,
    theme: MyTheme.darkTheme,
    home: _VideoSurface(args: args, controller: controller),
  ));
}

class _VideoSurface extends StatefulWidget {
  final Map<String, dynamic> args;
  final WindowController controller;
  const _VideoSurface({required this.args, required this.controller});
  @override
  State<_VideoSurface> createState() => _VideoSurfaceState();
}

class _VideoSurfaceState extends State<_VideoSurface> with MultiWindowListener {
  Timer? _timer;
  bool _polling = false, _closing = false;
  DateTime _lastOwner = DateTime.now();
  Map<String, dynamic> _snapshot = {};

  @override
  void initState() {
    super.initState();
    // Installing a handler is also required for plugin onEvent delivery.
    DesktopMultiWindow.setMethodHandler((call, from) async => null);
    DesktopMultiWindow.addListener(this);
    _timer = Timer.periodic(const Duration(milliseconds: 80), (_) => _poll());
    _poll();
  }

  Future<dynamic> _rpc(String method) => DesktopMultiWindow.invokeMethod(
          widget.args['owner'] as int,
          '${KomiVideoCallUi.rpcPrefix}$method',
          {'key': widget.args['key'], 'token': widget.args['token']})
      .timeout(const Duration(seconds: 3));

  Future<void> _poll() async {
    if (_polling || _closing) return;
    _polling = true;
    try {
      final result = Map<String, dynamic>.from(await _rpc('snapshot') as Map);
      _lastOwner = DateTime.now();
      if (result['ended'] == true) {
        await _close(notifyOwner: false);
      } else if (mounted) {
        setState(() => _snapshot = result);
      }
    } catch (e) {
      if (DateTime.now().difference(_lastOwner) > const Duration(seconds: 6)) {
        await _close(notifyOwner: false);
      }
    } finally {
      _polling = false;
    }
  }

  @override
  void onWindowClose() {
    unawaited(_close(notifyOwner: true));
  }

  Future<void> _close({required bool notifyOwner}) async {
    if (_closing) return;
    _closing = true;
    _timer?.cancel();
    try {
      if (notifyOwner) await _rpc('close');
    } catch (e) {
      debugPrint('Video owner unavailable on close: $e');
    } finally {
      try {
        await widget.controller.setPreventClose(false);
        await widget.controller.close();
      } catch (e) {
        // The owner may already have destroyed the native window on hangup.
        debugPrint('Video window already closed: $e');
      }
    }
  }

  @override
  void dispose() {
    _timer?.cancel();
    DesktopMultiWindow.removeListener(this);
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final remote = _snapshot['remotePath'] as String? ?? '';
    final local = _snapshot['localPath'] as String? ?? '';
    return Scaffold(
      backgroundColor: Colors.black,
      body: Stack(fit: StackFit.expand, children: [
        _JpegFrame(
            path: remote.isNotEmpty ? remote : local,
            seq: (remote.isNotEmpty
                    ? _snapshot['remoteSeq']
                    : _snapshot['localSeq']) as int? ??
                -1),
        if (remote.isNotEmpty && local.isNotEmpty)
          Positioned(
              right: 12,
              bottom: 12,
              width: 144,
              height: 108,
              child: _JpegFrame(
                  path: local, seq: _snapshot['localSeq'] as int? ?? -1)),
      ]),
    );
  }
}

/// A key does not invalidate FileImage's path cache. Read fresh JPEG bytes for
/// each sequence, serialize reads and discard stale completions after hangup.
class _JpegFrame extends StatefulWidget {
  final String path;
  final int seq;
  const _JpegFrame({required this.path, required this.seq});
  @override
  State<_JpegFrame> createState() => _JpegFrameState();
}

class _JpegFrameState extends State<_JpegFrame> {
  Uint8List? _bytes;
  bool _reading = false;
  int _revision = 0;

  @override
  void initState() {
    super.initState();
    _refresh();
  }

  @override
  void didUpdateWidget(covariant _JpegFrame oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.path != widget.path || oldWidget.seq != widget.seq) {
      _revision++;
      if (oldWidget.path != widget.path || widget.path.isEmpty) _bytes = null;
      _refresh();
    }
  }

  Future<void> _refresh() async {
    if (_reading || widget.path.isEmpty) return;
    _reading = true;
    final revision = _revision;
    try {
      final bytes = await File(widget.path).readAsBytes();
      if (mounted && revision == _revision && bytes.isNotEmpty) {
        // Evict the preceding MemoryImage; the same file path is never cached.
        if (_bytes != null) await MemoryImage(_bytes!).evict();
        if (mounted && revision == _revision) setState(() => _bytes = bytes);
      }
    } on FileSystemException {
      // An atomic replace/removal may race a read. Retry on the next poll/frame.
    } finally {
      _reading = false;
      if (mounted && revision != _revision) unawaited(_refresh());
    }
  }

  @override
  Widget build(BuildContext context) => _bytes == null
      ? const Center(child: Icon(Icons.videocam_off, color: Colors.white54))
      : Image.memory(_bytes!,
          fit: BoxFit.contain,
          gaplessPlayback: true,
          errorBuilder: (_, __, ___) => const SizedBox.shrink());
}
