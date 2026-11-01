import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_hbb/desktop/komi_device_name.dart';

void main() {
  test('device name preserves accents and normalizes whitespace', () {
    expect(sanitizeKomiDeviceName('  Équipe   d’Axel  '), 'Équipe d’Axel');
  });

  test('device name rejects controls and limits Unicode code points', () {
    expect(sanitizeKomiDeviceName('Komi\nRemote\tPC\u0007'), 'Komi Remote PC');
    expect(
        sanitizeKomiDeviceName(List.filled(80, 'é').join()).runes.length, 64);
  });

  test('recent connection priority is alias, hostname, then numeric id', () {
    expect(
        komiRecentPrimaryName(
            alias: 'Mon alias', hostname: 'poste-distant', id: '250178584'),
        'Mon alias');
    expect(
        komiRecentPrimaryName(
            alias: '', hostname: 'poste-distant', id: '250178584'),
        'poste-distant');
    expect(komiRecentPrimaryName(alias: '', hostname: '', id: '250178584'),
        '250178584');
  });
}
