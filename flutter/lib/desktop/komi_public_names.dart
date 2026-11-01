import 'dart:async';
import 'dart:convert';
import 'dart:ffi';
import 'dart:isolate';
import 'dart:io';
import 'package:ffi/ffi.dart';
import 'package:flutter/foundation.dart';

/// Set BOTH values at build time. The endpoint must be HTTPS and its certificate
/// validated normally; the namespace must exactly match the active ID server.
const komiRegistryUrl = 'https://noms.komjordan.fr/';
const komiRegistryNamespace = '217.154.20.200';

/// Local option the client presents to peers as this user's name (connection
/// prompts, cursor name tags). Mirrors only the confirmed public name; must
/// match komi_cursor_tag::OPTION_KOMI_DISPLAY_NAME in Rust.
const kKomiDisplayNameOption = 'komi-display-name';

final _alias = RegExp(r'^[a-z][a-z0-9_-]{2,15}$');
final _identifier = RegExp(r'^(?:[0-9]{1,32}|[A-Za-z][A-Za-z0-9_-]{5,31})$');
final _digits = RegExp(r'^\d+$');

class PublicNameException implements Exception {
  final String message;
  const PublicNameException(this.message);
  @override
  String toString() => message;
}

class PublicNameRecord {
  final String alias;
  final String rustdeskId;
  final String server;
  final String publicKey;
  const PublicNameRecord(
      this.alias, this.rustdeskId, this.server, this.publicKey);
}

typedef _NamesCall = Pointer<Utf8> Function(Pointer<Utf8>);
typedef _NamesFree = void Function(Pointer<Utf8>);

// Native owns the signing key, TLS requests and validation. Never log payloads.
Map<String, dynamic> _invokeKomiNames(String request) {
  if (!Platform.isWindows)
    throw const PublicNameException('Windows uniquement');
  try {
    final dll = DynamicLibrary.open('librustdesk.dll');
    final call =
        dll.lookupFunction<Pointer<Utf8> Function(Pointer<Utf8>), _NamesCall>(
            'komi_names_call');
    final free = dll.lookupFunction<Void Function(Pointer<Utf8>), _NamesFree>(
        'komi_names_free');
    final input = request.toNativeUtf8();
    try {
      final output = call(input);
      if (output.address == 0)
        throw const PublicNameException('Service indisponible');
      try {
        final text = output.toDartString();
        if (text.length > 4096)
          throw const PublicNameException('Réponse invalide');
        final data = jsonDecode(text);
        if (data is! Map<String, dynamic>)
          throw const PublicNameException('Réponse invalide');
        return data;
      } finally {
        free(output);
      }
    } finally {
      calloc.free(input);
    }
  } catch (e) {
    if (e is PublicNameException) rethrow;
    throw const PublicNameException('Service indisponible');
  }
}

bool validKomiAlias(String name) =>
    _alias.hasMatch(name) &&
    !{'root', 'admin', 'administrator', 'system', 'support', 'komiremote'}
        .contains(name);
bool validKomiId(String id) =>
    RegExp(r'^(?:[0-9]{1,32}|[A-Za-z][A-Za-z0-9_-]{5,31})$').hasMatch(id);

/// A directory lookup alone never confirms a locally controlled name.
String? confirmedKomiName(
    Map<String, dynamic> response, String namespace, String id) {
  final alias = response['alias'];
  if (response['server'] != namespace ||
      response['rustdesk_id'] != id ||
      alias is! String ||
      !validKomiAlias(alias)) return null;
  return alias;
}

/// Native status has already validated the record's ID and management key
/// against the exact ID passed to status(id); no bare directory data is accepted.
String? confirmedKomiStatus(Map<String, dynamic> status, String namespace) {
  if (status['has_identity'] != true || status['namespace'] != namespace)
    return null;
  final alias = status['alias'];
  return alias is String && validKomiAlias(alias) ? alias : null;
}

/// Maps a submission of the original change-ID dialog to a registry operation.
/// Null means "nothing to change" (same name already reserved here).
/// Global reactive notifier for the locally applied / confirmed public name.
/// Updated immediately when a name is reserved, renamed, or released,
/// so the home screen and other widgets reflect changes without polling delays.
final ValueNotifier<String?> komiActivePublicName = ValueNotifier<String?>(null);

String? komiNameDialogOperation(String? current, String requested) {
  if (current == null) return 'register';
  return current == requested ? null : 'rename';
}

class KomiNameManagement {
  final KomiNameRegistry registry;
  const KomiNameManagement(this.registry);

  static KomiNameManagement? configured() {
    final registry = KomiNameRegistry.configured();
    return Platform.isWindows && registry != null
        ? KomiNameManagement(registry)
        : null;
  }

  Future<Map<String, dynamic>> _call(Map<String, String> payload) async {
    final data = await Isolate.run(() => _invokeKomiNames(jsonEncode(payload)));
    if (data['error'] != null) {
      throw PublicNameException(data['error'] is String
          ? data['error'] as String
          : 'Opération impossible');
    }
    return data;
  }

  Future<void> ensureActiveServer() async {
    final result = await _call({
      'operation': 'active-server',
      'namespace': registry.namespace,
    });
    if (result['matched'] != true ||
        result['namespace'] != registry.namespace) {
      throw const PublicNameException('Serveur RustDesk différent du registre');
    }
  }

  Future<Map<String, dynamic>> status(String id) {
    if (!validKomiId(id)) throw const PublicNameException('ID invalide');
    return _call({
      'operation': 'status',
      'namespace': registry.namespace,
      'id': id,
      'url': registry.endpoint.toString(),
    });
  }

  Future<bool> availability(String alias) async {
    if (!validKomiAlias(alias))
      throw const PublicNameException(
          'Nom invalide (3 à 16 caractères minuscules)');
    final data = await _call({
      'operation': 'availability',
      'alias': alias,
      'namespace': registry.namespace,
      'url': registry.endpoint.toString(),
    });
    if (data['alias'] != alias ||
        data['server'] != registry.namespace ||
        data['available'] is! bool)
      throw const PublicNameException('Réponse non fiable');
    return data['available'] as bool;
  }

  Future<Map<String, dynamic>> change(String operation, String alias, String id,
      {String? newAlias}) async {
    if (!validKomiAlias(alias) ||
        !validKomiId(id) ||
        (operation == 'rename' &&
            (newAlias == null || !validKomiAlias(newAlias))) ||
        !{'register', 'rename', 'release'}.contains(operation)) {
      throw const PublicNameException('Nom ou ID invalide');
    }
    final data = await _call({
      'operation': operation,
      'alias': alias,
      'id': id,
      'namespace': registry.namespace,
      'url': registry.endpoint.toString(),
      if (operation == 'rename') 'new_alias': newAlias!,
    });
    if (operation == 'release') {
      if (data['released'] != true)
        throw const PublicNameException('Réponse non fiable');
    } else if (confirmedKomiName(data, registry.namespace, id) !=
        (newAlias ?? alias)) {
      throw const PublicNameException('Réponse non fiable');
    }
    return data;
  }
}

/// Read-only v1 registry client. No claim is treated as proof of peer ownership.
class KomiNameRegistry {
  final Uri endpoint;
  final String namespace;
  const KomiNameRegistry(this.endpoint, this.namespace);

  static KomiNameRegistry? configured() {
    final uri = Uri.tryParse(komiRegistryUrl);
    if (uri == null ||
        uri.scheme != 'https' ||
        uri.host.isEmpty ||
        uri.userInfo.isNotEmpty ||
        uri.hasQuery ||
        uri.hasFragment ||
        uri.path != '' && uri.path != '/' ||
        komiRegistryNamespace.isEmpty) {
      return null;
    }
    return KomiNameRegistry(uri, komiRegistryNamespace);
  }

  Future<Map<String, dynamic>> _get(String path) async {
    final client = HttpClient()..connectionTimeout = const Duration(seconds: 5);
    try {
      final request = await client
          .getUrl(endpoint.resolve(path))
          .timeout(const Duration(seconds: 5));
      request.followRedirects = false;
      final response =
          await request.close().timeout(const Duration(seconds: 5));
      if (response.statusCode != 200) {
        throw PublicNameException(response.statusCode == 404
            ? 'Public name not found'
            : 'Public-name service unavailable');
      }
      final body = await utf8.decoder
          .bind(response)
          .join()
          .timeout(const Duration(seconds: 5));
      if (body.length > 4096)
        throw const PublicNameException('Invalid registry response');
      final value = jsonDecode(body);
      if (value is! Map<String, dynamic>) {
        throw const PublicNameException('Invalid registry response');
      }
      return value;
    } on PublicNameException {
      rethrow;
    } catch (_) {
      throw const PublicNameException('Public-name service unavailable');
    } finally {
      client.close(force: true);
    }
  }

  Future<PublicNameRecord> resolve(String alias) async {
    if (!_alias.hasMatch(alias))
      throw const PublicNameException('Invalid public name');
    final data = await _get('resolve/$alias');
    if (data['alias'] != alias ||
        data['server'] != namespace ||
        data['rustdesk_id'] is! String ||
        !_identifier.hasMatch(data['rustdesk_id'] as String) ||
        data['public_key'] is! String) {
      throw const PublicNameException('Untrusted registry response');
    }
    final key = data['public_key'] as String;
    try {
      if (base64.encode(base64.decode(key)) != key ||
          base64.decode(key).length != 32) {
        throw const FormatException();
      }
    } catch (_) {
      throw const PublicNameException('Untrusted registry response');
    }
    return PublicNameRecord(
        alias, data['rustdesk_id'] as String, data['server'] as String, key);
  }

  Future<bool> available(String alias) async {
    if (!_alias.hasMatch(alias))
      throw const PublicNameException('Invalid public name');
    final data = await _get('availability/$alias');
    if (data['alias'] != alias ||
        data['server'] != namespace ||
        data['available'] is! bool) {
      throw const PublicNameException('Untrusted registry response');
    }
    return data['available'] as bool;
  }
}

/// Native explicit `native:` override also handles a new alphanumeric ID.
/// Known peer IDs take precedence over a registry alias, preserving recents.
Future<String> resolveKomiConnectionId(
  String raw, {
  required bool isKomi,
  required bool Function(String) isKnownPeer,
  KomiNameRegistry? registry,
  Future<void> Function(String)? verifyActiveServer,
}) async {
  if (!isKomi) return raw;
  if (raw.startsWith('native:')) {
    final id = raw.substring(7);
    if (id.isEmpty) throw const PublicNameException('Empty native ID');
    return id;
  }
  if (_digits.hasMatch(raw) ||
      raw.contains('@') ||
      raw.contains(':') ||
      raw.contains('.') ||
      raw.contains('/') ||
      raw.contains(' ') ||
      isKnownPeer(raw)) return raw;
  // Do not silently send a purported public name to the native engine.
  final canonical = raw.toLowerCase();
  if (!_alias.hasMatch(canonical) || !raw.runes.every((c) => c < 128)) {
    throw const PublicNameException(
        'Invalid public name; use native:ID for a native peer');
  }
  if (registry == null) {
    throw const PublicNameException('Public-name service is not configured');
  }
  final management = KomiNameManagement(registry);
  final verify = verifyActiveServer ?? (_) => management.ensureActiveServer();
  await verify(registry.namespace);
  final result = await registry.resolve(canonical);
  await verify(registry.namespace);
  return result.rustdeskId;
}
