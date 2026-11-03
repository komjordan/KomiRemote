"""Source seam guards; not a Flutter, Apple SDK or camera runtime test."""
from pathlib import Path
import re
import subprocess
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1]
REFERENCE = Path('/home/hermes/komi-v29/integrated')


def block(text, anchor):
    start = text.index(anchor)
    brace = text.index('{', start)
    depth, end = 1, brace + 1
    while depth:
        depth += (text[end] == '{') - (text[end] == '}')
        end += 1
    return text[start:end]


class MacVideoIntegration(unittest.TestCase):
    @unittest.skipUnless(REFERENCE.exists(), 'v29 reference checkout required for exact callback comparison')
    def test_flutter_callbacks_match_v29(self):
        actual = (ROOT / 'src/flutter.rs').read_text()
        reference = (REFERENCE / 'src/flutter.rs').read_text()
        for anchor in ['fn komi_camera_invite(', 'fn update_komi_video_call_state(',
                       'fn komi_video_frame(&self, path:', 'fn komi_video_frame(&self, id:']:
            self.assertEqual(block(actual, anchor), block(reference, anchor))
        # A common desktop tree keeps the Windows protocol callback. Native
        # floating pills remain compiled only on Windows, not on macOS.
        lib = (ROOT / 'src/lib.rs').read_text()
        self.assertRegex(lib, r'#\[cfg\(target_os = "windows"\)\]\s*mod komi_pills;')
        self.assertIn('fn komi_host_priority(&self, active: bool)', actual)

    def test_permissions_and_single_owner_rpc(self):
        ui = (ROOT / 'flutter/lib/desktop/widgets/komi_video_call_window.dart').read_text()
        native = (ROOT / 'flutter/macos/Runner/MainFlutterWindow.swift').read_text()
        for method in ['canRecordCamera', 'requestRecordCamera']:
            self.assertIn("'" + method + "'", ui)
            self.assertIn('case "' + method + '":', native)
        self.assertNotIn('check_camera_permission', ui)
        self.assertNotIn('request_camera_permission', ui)
        rpc = (ROOT / 'flutter/lib/utils/multi_window_manager.dart').read_text()
        handler = block(rpc, '  void setMethodHandler(')
        self.assertEqual(handler.count('DesktopMultiWindow.setMethodHandler'), 1)
        self.assertLess(handler.index('KomiVideoCallUi.handleRpc'), handler.index('await handler('))
        main = (ROOT / 'flutter/lib/main.dart').read_text()
        self.assertLess(main.index("argument['kind']"), main.index("int type = argument['type']"))
        self.assertIn('rustDeskWinManager.setMethodHandler', block(main, 'void runConnectionManagerScreen()'))

    def test_wire_and_ffi_surface(self):
        if REFERENCE.exists():
            for anchor in ['message KomiVideoCall ', 'message KomiCallCameraFrame ']:
                self.assertEqual(block((ROOT / 'libs/base/protos/message.proto').read_text(), anchor),
                                 block((REFERENCE / 'libs/base/protos/message.proto').read_text(), anchor))
        actual = (ROOT / 'src/flutter_ffi.rs').read_text()
        baseline = subprocess.check_output(['git', 'show', 'HEAD:src/flutter_ffi.rs'], cwd=ROOT, text=True)
        def signatures(s):
            return [re.sub(r'\s+', '', m) for m in re.findall(r'pub\s+(?:async\s+)?fn\s+[^\{]+\{', s)]
        self.assertEqual(signatures(actual), signatures(baseline))
        print('FFI signatures preserved:', len(signatures(actual)))

    def test_camera_adapter_and_locked_dependency_closure(self):
        quartz = (ROOT / 'libs/scrap/src/common/quartz.rs').read_text()
        self.assertIn('pub fn from_camera_rgba', quartz)
        self.assertIn('PixelStorage::Surface(frame)', quartz)
        self.assertIn('PixelStorage::Camera(_) => Pixfmt::RGBA', quartz)
        manifest = tomllib.loads((ROOT / 'libs/scrap/Cargo.toml').read_text())
        camera = manifest['target']['cfg(target_os = "macos")']['dependencies']['nokhwa']
        self.assertIn('input-avfoundation', camera['features'])
        packages = tomllib.loads((ROOT / 'Cargo.lock').read_text())['package']
        by_name = {}
        for package in packages:
            by_name.setdefault(package['name'], []).append(package)
        nokhwa = by_name['nokhwa'][0]
        self.assertTrue(nokhwa['source'].endswith('#c2f74662b6ce117f7f94301693fdfadc0b1ec91a'))
        self.assertIn('nokhwa-bindings-macos', nokhwa['dependencies'])
        seen = set()
        def check(package):
            key = (package['name'], package['version'], package.get('source', ''))
            if key in seen:
                return
            seen.add(key)
            for dep in package.get('dependencies', []):
                parts = dep.split(' ')
                matches = by_name.get(parts[0], [])
                if len(parts) > 1:
                    matches = [p for p in matches if p['version'] == parts[1]]
                self.assertEqual(len(matches), 1, dep)
                check(matches[0])
        check(by_name['nokhwa-bindings-macos'][0])
        print('Mac camera locked transitive packages:', len(seen))
        self.assertEqual((ROOT / 'Cargo.lock').read_bytes(),
                         subprocess.check_output(['git', 'show', 'HEAD:Cargo.lock'], cwd=ROOT))


if __name__ == '__main__':
    unittest.main()
