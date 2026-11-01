"""Narrow source guards; Rust unit tests exercise the pure policy seams.

No network, dependencies, configuration writes, or real credential fixtures.
"""
import hashlib
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]


def source(path):
    return (ROOT / path).read_text(encoding="utf-8")


def digest(text):
    return hashlib.sha256(text.encode()).hexdigest()


class NativePolicyTests(unittest.TestCase):
    def test_printer_ui_cannot_rehydrate_or_emit_for_komi(self):
        dart = source("flutter/lib/desktop/pages/install_page.dart")
        self.assertIn("if (appName != 'KomiRemote') {\n      printer.value = installOptions['PRINTER'] == '1';", dart)
        self.assertIn("if (appName != 'KomiRemote')\n                Option(printer", dart)
        self.assertIn("if (appName != 'KomiRemote' && printer.value) args += ' printer';", dart)

    def test_native_printer_noop_including_silent_and_update(self):
        win = source("src/platform/windows.rs")
        silent = win.split('pub fn get_silent_install_options', 1)[1].split('// This function', 1)[0]
        self.assertIn('if crate::get_app_name() == "KomiRemote" {\n        return "desktopicon startmenu";', silent)
        self.assertIn('let install_remote_printer = installer_printer_command(', win)
        install = win.split('pub fn install_me(', 1)[1].split('pub fn run_after_install', 1)[0]
        self.assertIn('get_uninstall(false, false)?', install)
        self.assertIn('get_uninstall(kill_self, true)?', win)  # explicit uninstall retained
        self.assertIn('if app_name == "KomiRemote" {\n        return String::new();', win)
        self.assertIn('if app_name == "KomiRemote" { None } else { Some(reg_value_printer) }', win)
        update = win.split('pub fn update_me(debug:', 1)[1].split('pub fn ', 1)[0]
        self.assertIn('let is_printer_installed = app_name != "KomiRemote"\n        && remote_printer::is_rd_printer_installed', update)
        self.assertIn('let (uninstall_printer_cmd, install_printer_cmd) = if is_printer_installed {', update)
        self.assertIn('pub fn get_printer_names', win)
        self.assertIn('pub fn send_raw_data_to_printer', win)  # engine retained

    def test_only_legal_destination_and_tooltip_changed(self):
        dart = source("flutter/lib/desktop/pages/install_page.dart")
        self.assertEqual(dart.count('https://komjordan.fr/privacy.html'), 2)
        start = dart.index('              Container(')
        end = dart.index('              Row(\n                children: [\n                  Expanded(', start)
        original = dart[start:end].replace('https://komjordan.fr/privacy.html', 'https://rustdesk.com/privacy.html')
        self.assertEqual(digest(original), 'd4342f8814ac1964ce0c51d4380cd8e53a2d7a5dbb15762281f3bb6a4d51f5fb')

    def test_totp_branding_without_storage_or_telegram_changes(self):
        auth = source('src/auth_2fa.rs')
        self.assertIn('fn totp_issuer(app_name: &str)', auth)
        self.assertIn('"KomiRemote Connection"', auth)
        self.assertIn('"RustDesk Connection"', auth)
        self.assertIn('Some(totp_issuer(&crate::get_app_name()).to_owned())', auth)
        stored = auth[auth.index('    pub fn into_string'):auth.index('\npub fn generate2fa')]
        telegram = auth[auth.index('#[derive(Debug, Clone, Default, Serialize, Deserialize)]\npub struct TelegramBot'):]
        self.assertEqual(digest(stored), '5e294e069a0f5c9c43b3adc9be64861c8c3cdbc4d99a808817d6308fd47ab59b')
        self.assertEqual(digest(telegram), '72244def7aae807de6ee0c05ca9e8f20384237c6d40df864ff7bb95e8f98963d')

    def test_advertisement_ignores_legacy_option(self):
        conn = source('src/server/connection.rs')
        hostname = conn.split('pi.hostname = if crate::get_app_name()', 1)[1].split('pi.platform', 1)[0]
        self.assertNotIn('Config::get_option', hostname)
        self.assertIn('advertised_device_name(&crate::whoami_hostname())', hostname)
        names = source('src/komi_device_name.rs')
        self.assertIn('pub struct VerifiedConnectionName(String);', names)
        self.assertIn('select_device_name(None, system_hostname)', names)
        self.assertIn('"KomiRemote".to_owned()', names)


if __name__ == '__main__':
    unittest.main()
