import 'package:flutter_hbb/desktop/komi_update_logic.dart';
import 'package:flutter_test/flutter_test.dart';

String manifest({String version = '1.5.1', int build = 70, String url = 'https://komjordan.fr/komi.html'}) =>
    '{"version":"$version","build":$build,"url":"$url","sha256":"00","notes":"n"}';

void main() {
  test('only https komjordan.fr URLs are allowed', () {
    expect(komiIsAllowedUpdateUri(Uri.parse('https://komjordan.fr/komi.html')), isTrue);
    expect(komiIsAllowedUpdateUri(Uri.parse('http://komjordan.fr/komi.html')), isFalse);
    expect(komiIsAllowedUpdateUri(Uri.parse('https://evil.example/komi.html')), isFalse);
    expect(komiIsAllowedUpdateUri(Uri.parse('https://komjordan.fr.evil.example/')), isFalse);
    expect(komiIsAllowedUpdateUri(Uri.parse('https://user@komjordan.fr/')), isFalse);
    expect(komiIsAllowedUpdateUri(Uri.parse('https://komjordan.fr:8443/')), isFalse);
    expect(komiIsAllowedUpdateUri(Uri.parse(kKomiUpdateManifestUrl)), isTrue);
  });

  test('version comparison uses version then build', () {
    expect(komiCompareVersions('1.5.1', 71, '1.5.1', 70)! > 0, isTrue);
    expect(komiCompareVersions('1.5.2', 1, '1.5.1', 70)! > 0, isTrue);
    expect(komiCompareVersions('1.5.1', 70, '1.5.1', 70), 0);
    expect(komiCompareVersions('1.5.0', 99, '1.5.1', 70)! < 0, isTrue);
    expect(komiCompareVersions('1.10.0', 0, '1.9.9', 0)! > 0, isTrue);
    expect(komiCompareVersions('abc', 1, '1.5.1', 70), isNull);
  });

  test('only a newer valid manifest is announced', () {
    expect(komiNewerUpdate(manifest()), isNull);
    expect(komiNewerUpdate(manifest(build: 69)), isNull);
    final info = komiNewerUpdate(manifest(build: 71));
    expect(info, isNotNull);
    expect(info!.version, '1.5.1');
    expect(info.url.toString(), 'https://komjordan.fr/komi.html');
    expect(komiNewerUpdate(manifest(version: '1.6.0', build: 1))!.version, '1.6.0');
    expect(komiNewerUpdate(manifest(build: 71, url: 'https://evil.example/x.exe')), isNull);
    expect(komiNewerUpdate(manifest(build: 71, url: 'http://komjordan.fr/komi.html')), isNull);
    expect(komiNewerUpdate('not json'), isNull);
    expect(komiNewerUpdate('[]'), isNull);
    expect(komiNewerUpdate('{"version":"1.6.0","build":"1","url":"https://komjordan.fr/"}'), isNull);
  });

  test('check runs at most once per 24h', () {
    const day = 24 * 60 * 60 * 1000;
    const now = 1000 * day;
    expect(komiUpdateCheckDue(null, now), isTrue);
    expect(komiUpdateCheckDue(now - day + 1, now), isFalse);
    expect(komiUpdateCheckDue(now - day, now), isTrue);
    expect(komiUpdateCheckDue(now + 5000, now), isTrue);
  });
  test('macOS reads optional mac_* fields, Windows ignores them', () {
    const shared =
        '"version":"1.5.1","build":72,"url":"https://komjordan.fr/KomiRemote.exe","sha256":"00","notes":"w"';
    // No mac_url: macOS stays silent, Windows unchanged.
    expect(komiNewerUpdate('{$shared}', currentBuild: 71, mac: true), isNull);
    expect(komiNewerUpdate('{$shared}', currentBuild: 71)!.url.toString(),
        'https://komjordan.fr/KomiRemote.exe');
    // mac_url only: shared version/build/notes are reused.
    final m1 = komiNewerUpdate('{$shared,"mac_url":"https://komjordan.fr/komi.html"}',
        currentBuild: 71, mac: true);
    expect(m1!.url.toString(), 'https://komjordan.fr/komi.html');
    expect(m1.build, 72);
    // mac_build overrides: a Windows-only bump does not reach macOS.
    final body = '{$shared,"mac_url":"https://komjordan.fr/komi.html","mac_build":71,"mac_notes":"m"}';
    expect(komiNewerUpdate(body, currentBuild: 71, mac: true), isNull);
    expect(komiNewerUpdate(body, currentBuild: 71)!.url.toString(),
        'https://komjordan.fr/KomiRemote.exe');
    expect(komiNewerUpdate(body, currentBuild: 70, mac: true)!.notes, 'm');
    // Same URL rules for mac_url, and wrong types are rejected.
    expect(komiNewerUpdate('{$shared,"mac_url":"https://evil.example/k.dmg"}',
        currentBuild: 71, mac: true), isNull);
    expect(komiNewerUpdate('{$shared,"mac_url":"https://komjordan.fr/","mac_build":"73"}',
        currentBuild: 71, mac: true), isNull);
  });
}
