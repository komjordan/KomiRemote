import 'dart:io';

import 'package:crypto/crypto.dart';
import 'package:flutter_hbb/common.dart';

/// True only for the KomiRemote-branded desktop client.
bool get isKomiRemote => appName == 'KomiRemote';

/// SHA-256 of the executable that is actually running. This identifies the
/// concrete Windows build instead of the device-key fingerprint.
Future<String> buildExecutableSha256({String? executablePath}) async {
  final path = executablePath ?? Platform.resolvedExecutable;
  return (await sha256.bind(File(path).openRead()).first).toString();
}
