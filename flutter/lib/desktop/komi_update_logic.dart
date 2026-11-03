// Pure helpers for the KomiRemote update notification. No I/O here so the
// rules (host allow-list, version ordering, 24h throttle) are unit tested.
import 'dart:convert';

const String kKomiUpdateHost = 'komjordan.fr';
const String kKomiUpdateManifestUrl = 'https://komjordan.fr/komi-version.json';

/// Version of this build. Keep in sync with flutter/pubspec.yaml (1.5.1+71).
const String kKomiCurrentVersion = '1.5.1';
const int kKomiCurrentBuild = 85;

const Duration kKomiUpdateCheckInterval = Duration(hours: 24);
const Duration kKomiUpdateTimeout = Duration(seconds: 6);
const int kKomiUpdateMaxBytes = 16 * 1024;

class KomiUpdateInfo {
  final String version;
  final int build;
  final Uri url;
  final String notes;

  const KomiUpdateInfo(this.version, this.build, this.url, this.notes);
}

/// Only https://komjordan.fr/... (default port, no credentials) is accepted,
/// both for the manifest and for the download page it points to.
bool komiIsAllowedUpdateUri(Uri? uri) {
  if (uri == null) return false;
  return uri.scheme == 'https' &&
      uri.host.toLowerCase() == kKomiUpdateHost &&
      (!uri.hasPort || uri.port == 443) &&
      uri.userInfo.isEmpty &&
      uri.fragment.isEmpty;
}

List<int>? _versionParts(String version) {
  if (!RegExp(r'^\d{1,6}(\.\d{1,6}){0,3}$').hasMatch(version)) return null;
  final parts = version.split('.').map(int.parse).toList();
  while (parts.length < 4) {
    parts.add(0);
  }
  return parts;
}

/// Returns >0 when (aVersion, aBuild) is newer than (bVersion, bBuild),
/// 0 when equal and <0 when older. Returns null if a version is malformed.
int? komiCompareVersions(
    String aVersion, int aBuild, String bVersion, int bBuild) {
  final a = _versionParts(aVersion);
  final b = _versionParts(bVersion);
  if (a == null || b == null) return null;
  for (var i = 0; i < 4; i++) {
    if (a[i] != b[i]) return a[i].compareTo(b[i]);
  }
  return aBuild.compareTo(bBuild);
}

/// Parses komi-version.json. Any unexpected shape returns null (silent).
///
/// Windows reads the original top-level fields (version, build, url, notes).
/// macOS ([mac] true) only reacts when `mac_url` is present; `mac_version`,
/// `mac_build` and `mac_notes` optionally override the shared values. Older
/// Windows builds simply ignore the extra `mac_*` keys.
KomiUpdateInfo? komiParseUpdateManifest(String body, {bool mac = false}) {
  try {
    if (body.length > kKomiUpdateMaxBytes) return null;
    final data = jsonDecode(body);
    if (data is! Map) return null;
    final version =
        mac && data.containsKey('mac_version') ? data['mac_version'] : data['version'];
    final build =
        mac && data.containsKey('mac_build') ? data['mac_build'] : data['build'];
    final url = mac ? data['mac_url'] : data['url'];
    final notes =
        mac && data.containsKey('mac_notes') ? data['mac_notes'] : data['notes'];
    if (version is! String || build is! int || url is! String) return null;
    if (_versionParts(version) == null || build < 0) return null;
    final uri = Uri.tryParse(url);
    if (!komiIsAllowedUpdateUri(uri)) return null;
    return KomiUpdateInfo(version, build, uri!, notes is String ? notes : '');
  } catch (_) {
    return null;
  }
}

/// The update to announce, or null when the manifest is invalid or not newer.
KomiUpdateInfo? komiNewerUpdate(String body,
    {String currentVersion = kKomiCurrentVersion,
    int currentBuild = kKomiCurrentBuild,
    bool mac = false}) {
  final info = komiParseUpdateManifest(body, mac: mac);
  if (info == null) return null;
  final cmp =
      komiCompareVersions(info.version, info.build, currentVersion, currentBuild);
  return (cmp != null && cmp > 0) ? info : null;
}

/// At most one check per 24h. A last-check time in the future (clock changed)
/// allows a new check.
bool komiUpdateCheckDue(int? lastCheckMs, int nowMs) {
  if (lastCheckMs == null || lastCheckMs <= 0) return true;
  if (lastCheckMs > nowMs) return true;
  return nowMs - lastCheckMs >= kKomiUpdateCheckInterval.inMilliseconds;
}
