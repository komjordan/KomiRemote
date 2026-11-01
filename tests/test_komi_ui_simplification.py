import pathlib
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
HOME = ROOT / "flutter/lib/desktop/pages/desktop_home_page.dart"
SETTINGS = ROOT / "flutter/lib/desktop/pages/desktop_setting_page.dart"
MAIN = ROOT / "flutter/lib/main.dart"
PEERS_VIEW = ROOT / "flutter/lib/common/widgets/peers_view.dart"
PEER_TABS = ROOT / "flutter/lib/models/peer_tab_model.dart"
MATRIX = ROOT / "flutter/lib/desktop/widgets/komi_matrix_rain.dart"


class KomiUiSimplification(unittest.TestCase):
    def test_matrix_runtime_is_completely_removed(self):
        self.assertFalse(MATRIX.exists())
        dart = "\n".join(
            path.read_text(encoding="utf-8")
            for path in (ROOT / "flutter/lib").rglob("*.dart")
        )
        self.assertNotIn("MatrixRain", dart)
        self.assertNotIn("komi-matrix-animation", dart)

    def test_contacts_address_book_tab_is_hidden_only_for_komiremote(self):
        source = PEER_TABS.read_text(encoding="utf-8")
        self.assertIn("appName != 'KomiRemote'", source)
        self.assertIn("PeerTabIndex", source)
        self.assertIn("'Recent sessions'", source)

    def test_home_keeps_responsive_scroll_and_install_card_at_bottom(self):
        source = HOME.read_text(encoding="utf-8")
        self.assertIn("LayoutBuilder", source)
        self.assertIn("SingleChildScrollView", source)
        self.assertIn("BoxConstraints(minHeight: constraints.maxHeight)", source)
        self.assertIn("MainAxisAlignment.spaceBetween", source)
        self.assertNotIn("FutureBuilder<Widget>", source)
        self.assertIn("isKomiRemote ? 280.0", source)
        self.assertIn("_kKomiIdentityAccentHeight", source)

    def test_komiremote_main_window_starts_at_920_by_740(self):
        source = MAIN.read_text(encoding="utf-8")
        self.assertIn("isKomiRemote ? const Size(920, 740) : null", source)

    def test_komiremote_install_card_uses_no_pink_gradient(self):
        # The card's former hard-coded dark fill (0xFF171F1C) became the
        # theme surface; the pink gradient must stay limited to non-Komi builds.
        source = HOME.read_text(encoding="utf-8")
        self.assertIn("const Color(0xFF4FBB85)", source)
        self.assertIn("Icons.install_desktop_outlined", source)
        card = source[source.index("Widget buildInstallCard") :]
        komi_branch = card[card.index("decoration: isKomiRemote") :]
        komi_branch = komi_branch[: komi_branch.index(": const BoxDecoration(")]
        self.assertIn("color: cardColors.surface", komi_branch)
        self.assertNotIn("gradient", komi_branch)
        self.assertNotIn("226, 66, 188", komi_branch)

    def test_id_gear_alias_and_mode_aware_copy_are_preserved(self):
        source = HOME.read_text(encoding="utf-8")
        fr = (ROOT / "src/lang/fr.rs").read_text(encoding="utf-8")
        self.assertIn("const Color(0xFF4FBB85)", source)
        self.assertIn("Icons.settings_outlined", source)
        self.assertIn("Platform.localHostname", source)
        self.assertIn('(\"komi_device_name\", \"Alias\")', fr)
        self.assertIn("model.approveMode == 'click'", source)
        self.assertIn("model.approveMode == 'password'", source)
        self.assertIn("Partagez votre ID pour une assistance.\\n", fr)
        self.assertIn("Mot de passe ou confirmation manuelle", fr)
        self.assertIn("textAlign: TextAlign.center", source)

    def test_recent_empty_state_uses_komiremote_green(self):
        source = PEERS_VIEW.read_text(encoding="utf-8")
        self.assertIn("LoadEvent.recent", source)
        self.assertIn("isKomiRemote", source)
        self.assertIn("MyTheme.accent", source)

    def test_about_is_compact_and_uses_executable_build_sha256(self):
        source = SETTINGS.read_text(encoding="utf-8")
        branding = (ROOT / "flutter/lib/desktop/komi_branding.dart").read_text(
            encoding="utf-8"
        )
        self.assertIn("buildExecutableSha256", branding)
        self.assertIn("Platform.resolvedExecutable", branding)
        self.assertIn("Empreinte numérique", source)
        self.assertIn("line('Version : 1.5.1')", source)
        self.assertIn("SelectableText(", source)
        self.assertIn("fontFamily: 'monospace'", source)
        self.assertIn("const Color(0xFF171F1C)", source)
        self.assertIn("const Color(0xFF4FBB85)", source)
        self.assertIn("textAlign: TextAlign.center", source)
        self.assertIn("Portfolio", source)
        self.assertIn("Icons.language_outlined", source)
        self.assertIn("Axel KAMGAING KOM", source)
        self.assertNotIn("link('Code source'", source)
        komi_branch = source[source.index("if (isKomiRemote) {") : source.index(
            "return SingleChildScrollView(", source.index("if (isKomiRemote) {") + 1
        )]
        self.assertNotIn("${translate('ID')}: $myId", komi_branch)
        self.assertIn("LEGAL-NOTICES-KOMIREMOTE.md", source)

    def test_legal_notice_is_accessible_in_corresponding_source(self):
        notice = ROOT / "docs/komiremote/LEGAL-NOTICES-KOMIREMOTE.md"
        self.assertTrue(notice.exists())
        text = notice.read_text(encoding="utf-8")
        self.assertIn("GNU Affero General Public License", text)
        self.assertIn("SANS AUCUNE GARANTIE", text)
        self.assertIn("Purslane Tech Pte. Ltd.", text)
        self.assertIn("https://github.com/komjordan/KomiRemote", text)


if __name__ == "__main__":
    unittest.main()
