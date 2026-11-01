import unittest
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
HOME = ROOT / "flutter/lib/desktop/pages/desktop_home_page.dart"
CONNECTION = ROOT / "flutter/lib/desktop/pages/connection_page.dart"
PEERS = ROOT / "flutter/lib/common/widgets/peer_tab_page.dart"
SETTINGS = ROOT / "flutter/lib/desktop/pages/desktop_setting_page.dart"
UI_INTERFACE = ROOT / "src/ui_interface.rs"
FR = ROOT / "src/lang/fr.rs"
EN = ROOT / "src/lang/en.rs"


class KomiHarmonization(unittest.TestCase):
    def test_identity_sections_share_layout_and_action_button(self):
        source = HOME.read_text(encoding="utf-8")
        self.assertGreaterEqual(source.count("_buildKomiIdentityBlock("), 4)
        self.assertGreaterEqual(source.count("_buildKomiIdentityAction("), 4)
        self.assertIn("const double _kKomiIdentityActionSize = 30.0", source)
        self.assertIn("color: MyTheme.accent", source)
        self.assertNotIn("suffixIcon: const Icon(Icons.edit_outlined", source)

    def test_connect_button_uses_komi_installer_palette(self):
        source = CONNECTION.read_text(encoding="utf-8")
        self.assertIn("backgroundColor: MyTheme.accent", source)
        self.assertIn("foregroundColor: const Color(0xFF111419)", source)
        self.assertRegex(
            source,
            r"disabledBackgroundColor:\s*MyTheme\.accent\.withOpacity\(0\.55\)",
        )
        self.assertIn("borderRadius: BorderRadius.circular(10)", source)

    def test_peer_navigation_actions_use_komi_accent(self):
        source = PEERS.read_text(encoding="utf-8")
        self.assertIn("Color _komiPeerActionColor", source)
        select_all = source.split("Widget selectAll(", 1)[1].split("Widget closeSelection(", 1)[0]
        self.assertIn("color: _komiPeerActionColor(context)", select_all)
        self.assertRegex(source, r"isKomiRemote\s*\?\s*MyTheme\.accent")
        self.assertIn("_komiPeerActionColor(context", source)
        self.assertIn("colorFilter: svgColor(_komiPeerActionColor(context", source)

    def test_about_is_simplified_and_has_horizontal_logo(self):
        source = SETTINGS.read_text(encoding="utf-8")
        self.assertNotIn("line('Projet : Fork indépendant de RustDesk OSS')", source)
        self.assertNotIn("link('Code source'", source)
        self.assertNotIn('"Projet d\'origine"', source)
        self.assertIn("title_suffix: [", source)
        self.assertIn("Widget _komiAboutLogo()", source)
        self.assertIn("loadLogo()", source)

    def test_printer_controls_are_hidden_only_for_komiremote(self):
        source = SETTINGS.read_text(encoding="utf-8")
        self.assertIn("if (isWindows &&\n        !isKomiRemote &&", source)
        self.assertIn("if (isWindows && !isKomiRemote)", source)
        self.assertIn("SettingsTabKey.printer", source)
        self.assertIn("kOptionEnableRemotePrinter", source)

    def test_ipc_loss_is_not_reported_as_server_unreachable(self):
        rust = UI_INTERFACE.read_text(encoding="utf-8")
        flutter = CONNECTION.read_text(encoding="utf-8")
        fr = FR.read_text(encoding="utf-8")
        en = EN.read_text(encoding="utf-8")
        self.assertIn("pub ipc_connected: bool", rust)
        self.assertIn("ipc_connected: true", rust)
        self.assertIn("ipc_connected: false", rust)
        self.assertIn("final ipcConnected = status['ipc_connected'] as bool? ?? false", flutter)
        self.assertIn("komi_status_service_unavailable", flutter)
        self.assertIn("État du service indisponible", fr)
        self.assertIn("Service status unavailable", en)
        self.assertIn("komi_status_rendezvous_unreachable", flutter)
        self.assertIn("Serveur ID/rendezvous inaccessible", fr)
        self.assertIn("ID/rendezvous server unreachable", en)


if __name__ == "__main__":
    unittest.main()
