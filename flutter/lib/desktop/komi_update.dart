// Startup update notification for KomiRemote (desktop only). Checks
// https://komjordan.fr/komi-version.json at most once per 24h, never blocks
// startup, stays silent on any error and never downloads or runs anything:
// "Télécharger" only opens the download page in the browser.
import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_hbb/common.dart';
import 'package:flutter_hbb/desktop/komi_branding.dart';
import 'package:flutter_hbb/desktop/komi_update_logic.dart';
import 'package:flutter_hbb/models/platform_model.dart';
import 'package:http/http.dart' as http;
import 'package:url_launcher/url_launcher.dart';

const String kKomiUpdateLastCheckOption = 'komi-update-last-check';

bool _komiUpdateCheckStarted = false;

void komiScheduleUpdateCheck() {
  if (!isKomiRemote || !isDesktop || _komiUpdateCheckStarted) return;
  _komiUpdateCheckStarted = true;
  Future.delayed(const Duration(seconds: 5), () {
    unawaited(_komiCheckForUpdate().catchError((_) {}));
  });
}

Future<void> _komiCheckForUpdate() async {
  try {
    final now = DateTime.now().millisecondsSinceEpoch;
    final last =
        int.tryParse(bind.mainGetLocalOption(key: kKomiUpdateLastCheckOption));
    if (!komiUpdateCheckDue(last, now)) return;
    await bind.mainSetLocalOption(
        key: kKomiUpdateLastCheckOption, value: now.toString());

    final manifestUri = Uri.parse(kKomiUpdateManifestUrl);
    if (!komiIsAllowedUpdateUri(manifestUri)) return;
    final client = http.Client();
    String body;
    try {
      final request = http.Request('GET', manifestUri)
        ..followRedirects = false
        ..headers['Accept'] = 'application/json';
      final response =
          await client.send(request).timeout(kKomiUpdateTimeout);
      if (response.statusCode != 200) return;
      final bytes = await response.stream.toBytes().timeout(kKomiUpdateTimeout);
      if (bytes.length > kKomiUpdateMaxBytes) return;
      body = utf8.decode(bytes, allowMalformed: true);
    } finally {
      client.close();
    }
    final info = komiNewerUpdate(body);
    if (info == null) return;
    _komiShowUpdateDialog(info);
  } catch (_) {
    // Silent by design: no network, timeout, bad JSON, etc.
  }
}

void _komiShowUpdateDialog(KomiUpdateInfo info) {
  gFFI.dialogManager.show(
    (setState, close, context) => CustomAlertDialog(
      title: Text(translate('komi_update_available')
          .replaceAll('{}', info.version)),
      content: info.notes.isEmpty
          ? const SizedBox.shrink()
          : SelectableText(info.notes),
      actions: [
        dialogButton('komi_update_later', isOutline: true, onPressed: close),
        dialogButton('komi_update_download', onPressed: () {
          close();
          if (komiIsAllowedUpdateUri(info.url)) {
            launchUrl(info.url, mode: LaunchMode.externalApplication);
          }
        }),
      ],
      onSubmit: close,
      onCancel: close,
    ),
    tag: 'komi-update-available',
  );
}
