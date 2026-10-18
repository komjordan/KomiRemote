"""Static integration guards, NOT Windows/Flutter runtime tests. stdlib only."""
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[1]


class WindowsInitialization(unittest.TestCase):
    def test_defaults_precede_windows_core_and_ffi_startup(self):
        core = (ROOT / 'src/core_main.rs').read_text().split('pub fn core_main()', 1)[1]
        self.assertIn('crate::common::initialize_komi_defaults();', core)
        self.assertLess(core.index('crate::common::initialize_komi_defaults();'), core.index('global_init()'))
        ffi = (ROOT / 'src/flutter_ffi.rs').read_text().split('fn initialize(', 1)[1]
        self.assertLess(ffi.index('crate::common::initialize_komi_defaults();'), ffi.index('start_flutter_async_runner()'))
        self.assertLess(ffi.index('crate::common::initialize_komi_defaults();'), ffi.index('custom_client_config.is_empty()'))
        common = (ROOT / 'src/common.rs').read_text()
        loader = common.split('pub fn load_custom_client()', 1)[1]
        self.assertLess(loader.index('initialize_komi_defaults();'), loader.index('custom.txt'))
        helper = common.split('pub(crate) fn initialize_komi_defaults()', 1)[1].split('pub fn load_custom_client()', 1)[0]
        self.assertIn('#[cfg(windows)]', helper)
        self.assertIn('static INIT: std::sync::Once', helper)
        self.assertIn('&config::APP_NAME', helper)
        self.assertIn('&config::DEFAULT_SETTINGS', helper)
        self.assertIn('&config::DEFAULT_LOCAL_SETTINGS', helper)
        self.assertNotIn('OVERWRITE_', helper)

    def test_upstream_persisted_options_precede_fork_defaults(self):
        # Guards the actual pinned submodule implementation, not a mock resolver.
        config = (ROOT / 'libs/hbb_common/src/config.rs').read_text()
        resolver = config.split('fn get_or(', 1)[1].split('#[inline]', 1)[0]
        self.assertRegex(resolver, r'\.get\(k\)\s*\.or\(b.get\(k\)\)\s*\.or\(c.read\(\).unwrap\(\).get\(k\)\)')
        for reader, end in [('pub fn get_option(k: &str)', 'pub fn get_bool_option'), ('pub fn get_flutter_option(k: &str)', 'pub fn set_flutter_option')]:
            # LocalConfig uses this resolver for both language and Flutter theme.
            local = config.split('impl LocalConfig', 1)[1]
            body = local.split(reader, 1)[1].split(end, 1)[0]
            self.assertIn('&DEFAULT_LOCAL_SETTINGS', body)
        self.assertIn('k == keys::OPTION_LANGUAGE && v == "default"', config)
        self.assertIn('config.options.insert(k, "".to_owned())', config)


class BrandPresentation(unittest.TestCase):
    def test_visible_metadata_does_not_rename_internal_binaries(self):
        import tomllib
        for path in ['Cargo.toml', 'libs/portable/Cargo.toml']:
            metadata = tomllib.loads((ROOT / path).read_text())['package']['metadata']['winres']
            self.assertEqual(metadata['ProductName'], 'Komi Remote')
            self.assertEqual(metadata['FileDescription'], 'Komi Remote — RustDesk fork')
            self.assertEqual(metadata['OriginalFilename'], 'rustdesk.exe')
            self.assertIn('Purslane', metadata['LegalCopyright'])
        rc = (ROOT / 'flutter/windows/runner/Runner.rc').read_text()
        self.assertIn('VALUE "ProductName", "Komi Remote"', rc)
        self.assertIn('VALUE "CompanyName", "Komi Remote contributors"', rc)
        self.assertIn('VALUE "InternalName", "rustdesk"', rc)
        self.assertIn('VALUE "OriginalFilename", "rustdesk.exe"', rc)
        self.assertIn('Purslane Tech Pte. Ltd. All rights reserved.', rc)

    def test_about_identifies_fork_and_corresponding_source(self):
        about = (ROOT / 'flutter/lib/desktop/pages/desktop_setting_page.dart').read_text().split('class _AboutState', 1)[1]
        self.assertIn("isWindows && appName == 'KomiRemote'", about)
        self.assertIn('Komi Remote — fork indépendant de RustDesk', about)
        self.assertIn('https://github.com/komjordan/KomiRemote', about)
        self.assertIn('Purslane Tech Pte. Ltd.', about)
        self.assertIn('$license', about)


class OriginalBrandAssets(unittest.TestCase):
    def test_original_assets_match_pack_manifest(self):
        import hashlib
        import json
        manifest = json.loads((ROOT / 'docs/komiremote/assets-manifest.json').read_text())
        self.assertEqual(len(manifest), 15)
        for item in manifest:
            with self.subTest(target=item['target']):
                path = ROOT / item['target']
                self.assertTrue(path.is_file(), str(path))
                if path.is_file():
                    self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(), item['sha256'])


if __name__ == '__main__':
    unittest.main()
