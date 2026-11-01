"""Source-level guards for the -5 home changes; run with Python unittest.

These supplement, not replace, Flutter tests and desktop accessibility QA.
"""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
HOME = ROOT / 'flutter/lib/desktop/pages/desktop_home_page.dart'
NAME = ROOT / 'flutter/lib/desktop/komi_device_name.dart'
CONNECTION = ROOT / 'flutter/lib/desktop/pages/connection_page.dart'


class AutomaticNameTests(unittest.TestCase):
    def test_home_never_reads_or_writes_legacy_name(self):
        home = HOME.read_text(encoding='utf-8')
        for obsolete in ('kOptionKomiDeviceName', '_saveDeviceName',
                         '_deviceNameController', '_deviceNameFocusNode',
                         '_deviceNameSaveTimer'):
            self.assertFalse(obsolete in home, obsolete)
        self.assertIn('komiAutomaticDeviceName(Platform.localHostname)', home)
        self.assertIn("kOptionKomiDeviceName = 'komi-device-name'", NAME.read_text())

    def test_home_name_is_noneditable_and_info_uses_accessible_button(self):
        home = HOME.read_text(encoding='utf-8')
        self.assertIn('Widget _buildDeviceNameDisplay(', home)
        block = home.split('Widget _buildDeviceNameDisplay(')[1].split(
            'Widget buildPopupMenu(')[0]
        self.assertNotIn('TextFormField', block)
        self.assertNotIn('switch2page', block)
        self.assertIn("translate('komi_home_device_name')", block)
        self.assertIn('IconButton(', block)
        self.assertIn('tooltip: customizationHint', block)
        self.assertIn('Pour modifier ce nom, allez dans Paramètres > Sécurité > Personnaliser mon nom de connexion.', block)
        self.assertIn('showDialog<void>', block)
        self.assertIn('TextOverflow.ellipsis', block)


class PresentationTests(unittest.TestCase):
    def test_install_card_uses_theme_surface_and_foreground(self):
        home = HOME.read_text(encoding='utf-8')
        card = home.split('Widget buildInstallCard(')[1].split('void initState()')[0]
        self.assertTrue('color: cardColors.surface' in card)
        self.assertTrue('cardColors.onSurface' in card)
        self.assertIn('seedColor: cardColors.secondary', card)
        self.assertRegex(card, r'backgroundColor:\s*cardActions\.primary')
        self.assertRegex(card, r'foregroundColor:\s*cardActions\.onPrimary')
        self.assertFalse('0xFF171F1C' in card)
        self.assertIn('onTap: onPressed', card)

    def test_connection_help_is_localized_and_does_not_replace_parser(self):
        connection = CONNECTION.read_text()
        self.assertTrue("translate('komi_connection_help')" in connection)
        self.assertTrue("translate('komi_connection_help_title')" in connection)
        self.assertIn('getConnectionPageTitle(context, false)', connection)
        self.assertIn('inputFormatters: [IDTextInputFormatter()]', connection)
        self.assertIn('var id = _idController.id;', connection)

    def test_new_keys_have_nonempty_english_and_french_resources(self):
        for language in ('en', 'fr'):
            resource = (ROOT / 'src/lang' / (language + '.rs')).read_text()
            for key in ('komi_home_device_name', 'komi_home_device_name_help',
                        'komi_connection_help_title', 'komi_connection_help'):
                self.assertEqual(resource.count('("' + key + '",'), 1, key)
                self.assertNotIn('("' + key + '", "")', resource)
            self.assertIn('("komi_device_name", "Alias")', resource)


if __name__ == '__main__':
    unittest.main()
