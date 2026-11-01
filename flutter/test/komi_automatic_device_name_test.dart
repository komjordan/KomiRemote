import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_hbb/desktop/komi_device_name.dart';

void main() {
  test('automatic home name uses the sanitized hostname', () {
    expect(komiAutomaticDeviceName('  DESKTOP-KJ01  '), 'DESKTOP-KJ01');
    expect(komiAutomaticDeviceName(' Équipe\n  principale\u0007 '),
        'Équipe principale');
  });

  test('missing hostname uses the same neutral name as the native engine', () {
    for (final hostname in ['', '  ', '\u0000\n\t\u007f']) {
      expect(komiAutomaticDeviceName(hostname), 'KomiRemote');
    }
  });

  test('automatic home name is bounded by Unicode code points', () {
    final hostname = List.filled(80, '😀').join();
    final result = komiAutomaticDeviceName(hostname);
    expect(result, List.filled(64, '😀').join());
    expect(result.runes.length, kMaxKomiDeviceNameCharacters);
  });
}
