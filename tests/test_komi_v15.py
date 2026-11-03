import json
import pathlib
import re
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
TAG = "v1.5.1-komi.71"


def read(rel):
    return (ROOT / rel).read_text(encoding="utf-8")


def about_info():
    page = read("flutter/lib/desktop/pages/desktop_setting_page.dart")
    start = page.index("List<Widget> _komiAboutInfo(")
    return page[start : page.index("Widget _komiAboutLogo()", start)]


def about_urls():
    page = read("flutter/lib/desktop/pages/desktop_setting_page.dart")
    return dict(re.findall(r"const String (kKomi\w+Url) =\s*'([^']+)';", page))


class KomiV15About(unittest.TestCase):
    def test_licence_named_once_and_linked(self):
        info = about_info()
        self.assertEqual(info.count("AGPL-3.0"), 1)
        self.assertNotIn("Voir la licence", info)
        self.assertIn("onTap: () => launchUrlString(kKomiLicenceUrl)", info)
        self.assertEqual(
            about_urls()["kKomiLicenceUrl"],
            "https://github.com/komjordan/KomiRemote/blob/main/LICENCE",
        )

    def test_order_licence_modified_source_then_build_date(self):
        info = about_info()
        order = [
            "line('Description : Bureau à distance')",
            "'Éditeur : Axel KAMGAING KOM'",
            "const TextSpan(text: 'Licence : ')",
            "', sans aucune garantie'",
            "line('Version modifiée de RustDesk, 2026.')",
            "link('Code source', kKomiSourceUrl, kKomiSourceUrl)",
            "line('Date de compilation : $buildDate')",
            "'Copyright © 2026 Purslane Tech Pte. Ltd.'",
            "link('Mentions légales', 'LEGAL-NOTICES-KOMIREMOTE.md', kKomiLegalNoticesUrl)",
        ]
        positions = [info.index(item) for item in order]
        self.assertEqual(positions, sorted(positions))
        self.assertEqual(info.count("Version modifiée de RustDesk"), 1)

    def test_source_and_legal_links_use_published_tag(self):
        urls = about_urls()
        self.assertEqual(urls["kKomiWindowsSourceUrl"], f"https://github.com/komjordan/KomiRemote/tree/{TAG}")
        self.assertEqual(urls["kKomiMacSourceUrl"], "https://github.com/komjordan/KomiRemote/tree/komiremote-macos")
        settings = read("flutter/lib/desktop/pages/desktop_setting_page.dart")
        self.assertIn("Platform.isMacOS ? kKomiMacSourceUrl : kKomiWindowsSourceUrl", settings)
        workflow = read(".github/workflows/komiremote-macos-test.yml")
        self.assertIn("assert t.count(old) == 1", workflow)
        self.assertIn("os.environ['GITHUB_SHA']", workflow)
        self.assertEqual(
            urls["kKomiLegalNoticesUrl"],
            f"https://github.com/komjordan/KomiRemote/blob/{TAG}/docs/komiremote/LEGAL-NOTICES-KOMIREMOTE.md",
        )
        self.assertTrue((ROOT / "docs/komiremote/LEGAL-NOTICES-KOMIREMOTE.md").is_file())
        self.assertTrue((ROOT / "LICENCE").is_file())


class KomiV15Toolbar(unittest.TestCase):
    def test_expanded_handle_has_no_frame_or_shadow(self):
        bar = read("flutter/lib/desktop/widgets/remote_toolbar.dart")
        collapse = bar[bar.index("Widget _buildDraggableCollapse(") : bar.index("Widget _buildToolbar(")]
        self.assertIn("type: compact ? MaterialType.canvas : MaterialType.transparency", collapse)
        self.assertIn("elevation: compact ? _ToolbarTheme.elevation : 0", collapse)
        handle = bar[bar.index("class _DraggableShowHideState") :]
        handle = handle[: handle.index("\nclass ")]
        expanded = handle[handle.index("final axis = widget.isHorizontal") :]
        self.assertNotIn("Border.all", expanded)
        self.assertNotIn("BoxDecoration", expanded)
        self.assertNotIn("Divider", expanded)
        # The collapsed bar keeps its framed look.
        compact = handle[handle.index("if (widget.compact)") : handle.index("final axis = widget.isHorizontal")]
        self.assertIn("Border.all", compact)


class KomiV15Recording(unittest.TestCase):
    def test_stop_shows_saved_dialog(self):
        model = read("flutter/lib/models/model.dart")
        rec = model[model.index("class RecordingModel") : model.index("class ElevationModel")]
        self.assertIn("final stopped = _start && !status;", rec)
        self.assertIn("bind.mainVideoSaveDirectory(root: false)", rec)
        self.assertIn("SelectableText(dir)", rec)
        self.assertIn("dialogButton('komi_open_folder'", rec)
        self.assertIn("dialogButton('komi_ok'", rec)
        self.assertIn("launchUrl(Uri.file(dir))", rec)
        self.assertIn("isDesktop", rec)

    def test_translations(self):
        fr = read("src/lang/fr.rs")
        en = read("src/lang/en.rs")
        for key, fr_text, en_text in [
            ("komi_recording_saved_to", "Enregistrement sauvegardé dans :", "Recording saved to:"),
            ("komi_open_folder", "Ouvrir le dossier", "Open folder"),
            ("komi_ok", "OK", "OK"),
            ("komi_update_available", "Nouvelle version de KomiRemote disponible ({})", "New KomiRemote version available ({})"),
            ("komi_update_download", "Télécharger", "Download"),
            ("komi_update_later", "Plus tard", "Later"),
        ]:
            self.assertIn(f'("{key}", "{fr_text}")', fr)
            self.assertIn(f'("{key}", "{en_text}")', en)


class KomiV15Update(unittest.TestCase):
    def test_update_check_is_https_only_and_throttled(self):
        logic = read("flutter/lib/desktop/komi_update_logic.dart")
        self.assertIn("kKomiUpdateHost = 'komjordan.fr'", logic)
        self.assertIn("kKomiUpdateManifestUrl = 'https://komjordan.fr/komi-version.json'", logic)
        self.assertIn("uri.scheme == 'https'", logic)
        self.assertIn("Duration(hours: 24)", logic)
        upd = read("flutter/lib/desktop/komi_update.dart")
        self.assertIn("..followRedirects = false", upd)
        self.assertIn(".timeout(kKomiUpdateTimeout)", upd)
        self.assertIn("LaunchMode.externalApplication", upd)
        self.assertNotIn("Process.run", upd)
        self.assertNotIn("Process.start", upd)
        self.assertNotIn("File(", upd)
        home = read("flutter/lib/desktop/pages/desktop_home_page.dart")
        self.assertIn("komiScheduleUpdateCheck();", home)

    def test_current_version_matches_pubspec(self):
        logic = read("flutter/lib/desktop/komi_update_logic.dart")
        version = re.search(r"kKomiCurrentVersion = '([\d.]+)'", logic).group(1)
        build = re.search(r"kKomiCurrentBuild = (\d+);", logic).group(1)
        self.assertRegex(read("flutter/pubspec.yaml"), rf"(?m)^version: {re.escape(version)}\+{build}$")
        self.assertEqual(version, "1.5.1")
        self.assertGreaterEqual(int(build), 71)


class KomiV15Logos(unittest.TestCase):
    def test_brand_assets_keep_expected_formats(self):
        manifest = json.loads(read("docs/komiremote/assets-manifest.json"))
        sources = {item["target"]: item["source"] for item in manifest}
        self.assertEqual(sources["flutter/assets/icon.svg"], "svg/komiremote-icon.svg")
        self.assertEqual(sources["flutter/assets/logo_light.png"], "png/komiremote-wordmark-light-bg-1600.png")
        self.assertEqual(sources["flutter/assets/logo_dark.png"], "png/komiremote-wordmark-dark-bg-1600.png")
        for rel in ("res/icon.ico", "res/tray-icon.ico", "flutter/assets/icon.ico",
                    "flutter/windows/runner/resources/app_icon.ico"):
            self.assertEqual((ROOT / rel).read_bytes()[:4], b"\x00\x00\x01\x00", rel)
        for rel in ("res/32x32.png", "res/64x64.png", "res/128x128.png", "res/128x128@2x.png",
                    "res/icon.png", "flutter/assets/icon.png", "flutter/assets/logo.png"):
            self.assertEqual((ROOT / rel).read_bytes()[:8], b"\x89PNG\r\n\x1a\n", rel)
        self.assertIn("<svg", read("res/scalable.svg"))


if __name__ == "__main__":
    unittest.main()
