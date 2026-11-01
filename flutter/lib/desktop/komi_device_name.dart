const String kOptionKomiDeviceName = 'komi-device-name';
const int kMaxKomiDeviceNameCharacters = 64;

/// Preserve Unicode letters and accents, reject control characters, normalize
/// whitespace, and limit by Unicode code points rather than UTF-16 units.
String sanitizeKomiDeviceName(String value) {
  final withoutControls =
      value.replaceAll(RegExp(r'[\u0000-\u001F\u007F-\u009F]'), ' ');
  final normalized = withoutControls.trim().replaceAll(RegExp(r'\s+'), ' ');
  return String.fromCharCodes(
      normalized.runes.take(kMaxKomiDeviceNameCharacters));
}

/// No confirmed naming service is available. Never use the legacy local alias
/// as a public name; keep the fallback aligned with the native advertised name.
String komiAutomaticDeviceName(String hostname) {
  final cleanHostname = sanitizeKomiDeviceName(hostname);
  return cleanHostname.isNotEmpty ? cleanHostname : 'KomiRemote';
}

String komiRecentPrimaryName({
  required String alias,
  required String hostname,
  required String id,
}) {
  final cleanAlias = alias.trim();
  if (cleanAlias.isNotEmpty) return cleanAlias;
  final cleanHostname = hostname.trim();
  if (cleanHostname.isNotEmpty) return cleanHostname;
  return id;
}
