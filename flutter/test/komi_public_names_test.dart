import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_hbb/desktop/komi_public_names.dart';

void main() {
  test('native management status requires matching namespace and local ID', () {
    const namespace = 'rd.example.org';
    const id = '123456789';
    final good = <String, dynamic>{
      'has_identity': true,
      'namespace': namespace,
      'alias': 'alice1'
    };
    expect(confirmedKomiStatus(good, namespace), 'alice1');
    for (final bad in [
      {...good, 'namespace': 'other.example'},
      {...good, 'has_identity': false},
      {...good, 'alias': 'admin'},
      {'has_identity': true, 'namespace': namespace},
    ]) {
      expect(confirmedKomiStatus(bad, namespace), isNull);
    }
    // A bare public lookup must never be promoted to a local status.
    expect(
        confirmedKomiStatus(
            {'alias': 'alice1', 'rustdesk_id': id, 'server': namespace},
            namespace),
        isNull);
  });

  test('registry IDs and aliases match backend grammar', () {
    expect(validKomiId('1'), isTrue);
    expect(validKomiId('alice1'), isTrue);
    expect(validKomiId('123 456'), isFalse);
    expect(validKomiAlias('alice1'), isTrue);
    for (final name in ['joe', 'papa', 'popo']) {
      expect(validKomiAlias(name), isTrue, reason: name);
    }
    expect(validKomiAlias('jo'), isFalse);
    expect(validKomiAlias('admin'), isFalse);
    expect(validKomiAlias('Alice1'), isFalse);
  });

  test('numeric, custom server and known alphanumeric peer bypass lookup',
      () async {
    for (final id in [
      '123 456',
      '123456',
      '111@myserver',
      '1.2.3.4',
      'KnownPeer'
    ]) {
      expect(
          await resolveKomiConnectionId(id,
              isKomi: true, isKnownPeer: (s) => s == 'KnownPeer'),
          id);
    }
    expect(
        await resolveKomiConnectionId('native:newPeer',
            isKomi: true, isKnownPeer: (_) => false),
        'newPeer');
    expect(
        await resolveKomiConnectionId('bare',
            isKomi: false, isKnownPeer: (_) => false),
        'bare');
  });

  test('unconfigured, short and ambiguous names fail closed', () async {
    for (final name in ['alice1', 'popo', 'UpperCase']) {
      await expectLater(
          resolveKomiConnectionId(name,
              isKomi: true, isKnownPeer: (_) => false),
          throwsA(isA<PublicNameException>()));
    }
  });

  test('registry maps only a pinned namespace response to native ID', () async {
    final server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
    final subscription = server.listen((request) async {
      request.response.headers.contentType = ContentType.json;
      request.response.write(jsonEncode({
        'alias': 'alice1',
        'rustdesk_id': '123456789',
        'server': request.uri.path == '/resolve/alice1'
            ? 'rd.example.org'
            : 'evil.example',
        'public_key': base64.encode(List<int>.filled(32, 1)),
      }));
      await request.response.close();
    });
    try {
      final registry = KomiNameRegistry(
          Uri.parse('http://127.0.0.1:${server.port}/'), 'rd.example.org');
      expect(
          await resolveKomiConnectionId('alice1',
              isKomi: true,
              isKnownPeer: (_) => false,
              registry: registry,
              verifyActiveServer: (_) async {}),
          '123456789');
      final wrong = KomiNameRegistry(
          Uri.parse('http://127.0.0.1:${server.port}/'), 'other.example');
      await expectLater(
          wrong.resolve('alice1'), throwsA(isA<PublicNameException>()));
    } finally {
      await subscription.cancel();
      await server.close(force: true);
    }
  });

  test('original change-ID dialog maps submit to register, rename or no-op',
      () {
    expect(komiNameDialogOperation(null, 'joe'), 'register');
    expect(komiNameDialogOperation('joe', 'papa'), 'rename');
    expect(komiNameDialogOperation('papa', 'papa'), isNull);
  });

  test('komiActivePublicName notifies listeners on update', () {
    String? captured;
    void listener() {
      captured = komiActivePublicName.value;
    }
    komiActivePublicName.addListener(listener);
    try {
      komiActivePublicName.value = 'papa';
      expect(captured, 'papa');
      komiActivePublicName.value = null;
      expect(captured, isNull);
    } finally {
      komiActivePublicName.removeListener(listener);
    }
  });
}
