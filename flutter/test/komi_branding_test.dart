import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_hbb/desktop/komi_branding.dart';

void main() {
  test('build fingerprint is the SHA-256 of the selected executable', () async {
    final directory = await Directory.systemTemp.createTemp('komi-hash-test-');
    final file = File('${directory.path}${Platform.pathSeparator}rustdesk.exe');
    await file.writeAsBytes(utf8.encode('abc'));

    expect(
      await buildExecutableSha256(executablePath: file.path),
      'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad',
    );

    await directory.delete(recursive: true);
  });
}
