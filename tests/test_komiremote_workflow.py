"""Static safety contract, not a Windows build. Requires PyYAML 6.0.2."""
from pathlib import Path
import re
import unittest
import yaml

ROOT = Path(__file__).resolve().parents[1]
WORKFLOW = ROOT / '.github/workflows/komiremote-windows-test.yml'


class ManualWorkflowTests(unittest.TestCase):
    def test_upstream_pins_and_isolation(self):
        w = yaml.load(WORKFLOW.read_text(), Loader=yaml.BaseLoader)
        upstream = yaml.load((ROOT / '.github/workflows/flutter-build.yml').read_text(), Loader=yaml.BaseLoader)
        self.assertEqual(w['env']['VCPKG_COMMIT_ID'], upstream['env']['VCPKG_COMMIT_ID'])
        self.assertEqual(w['env']['VCPKG_BINARY_SOURCES'], 'clear')
        self.assertEqual(w['jobs']['windows']['runs-on'], 'windows-2022')
        self.assertEqual(w['jobs']['bridge']['needs'], 'validate')
        self.assertEqual(w['jobs']['windows']['needs'], 'bridge')
        inputs = w['on']['workflow_dispatch']['inputs']
        self.assertEqual(set(inputs), {'engine_sha256', 'usbmmidd_sha256', 'printer_sha256', 'adapter_sha256'})
        self.assertTrue(all(v['required'] == 'true' for v in inputs.values()))
        bridge = w['jobs']['bridge']['steps']
        windows = w['jobs']['windows']['steps']
        get_action = lambda steps, name: next(s for s in steps if s.get('uses', '').startswith(name + '@'))
        self.assertEqual(get_action(bridge, 'subosito/flutter-action')['with']['flutter-version'], '3.22.3')
        self.assertEqual(get_action(windows, 'subosito/flutter-action')['with']['flutter-version'], upstream['env']['FLUTTER_VERSION'])
        self.assertEqual(get_action(windows, 'KyleMayes/install-llvm-action')['with']['version'], upstream['env']['LLVM_VERSION'])
        for steps in (bridge, windows):
            self.assertEqual(get_action(steps, 'dtolnay/rust-toolchain')['with']['toolchain'], '1.75.0')
        upstream_bridge = yaml.load((ROOT / '.github/workflows/bridge.yml').read_text(), Loader=yaml.BaseLoader)
        expected = upstream_bridge['jobs']['generate_bridge']['steps'][-1]['with']['path']
        uploaded = get_action(bridge, 'actions/upload-artifact')['with']['path']
        self.assertEqual({p.removeprefix('./') for p in expected.split()}, set(uploaded.split()))
        all_runs = '\n'.join(s.get('run', '') for j in w['jobs'].values() for s in j['steps'])
        self.assertNotIn('${{', all_runs, 'input values must enter scripts only through env')
        for required in ('--portable --flutter --skip-portable-pack --hwcodec --vram',
                         '--enforce-lockfile', 'git diff --exit-code -- Cargo.lock flutter/pubspec.lock',
                         'cargo-expand --version 1.0.95 --locked',
                         'flutter_rust_bridge_codegen --version 1.80.1',
                         'brotli==1.1.0', 'ecd8d6a139eee76845ea66423fb739af450fda90',
                         'generate.py -f ../../rustdesk/ -o . -e ../../rustdesk/rustdesk.exe'):
            self.assertIn(required, all_runs)

    def test_referenced_build_helpers_exist(self):
        for name in ('fetch_assets.py', 'assemble.py'):
            self.assertTrue((ROOT / '.github/komiremote' / name).is_file(), name)

    def test_manual_readonly_artifacts_only(self):
        self.assertTrue(WORKFLOW.exists(), 'dedicated manual workflow missing')
        workflow = yaml.load(WORKFLOW.read_text(), Loader=yaml.BaseLoader)
        self.assertEqual(set(workflow['on']), {'workflow_dispatch'})
        self.assertEqual(workflow['permissions'], {'contents': 'read'})
        text = WORKFLOW.read_text()
        for forbidden in ('secrets.', 'secrets: inherit', 'action-gh-release',
                          'git push', 'git commit', 'contents: write',
                          'pull_request_target', 'run-service', 'preprocess.py'):
            self.assertNotIn(forbidden, text)
        for job in workflow['jobs'].values():
            self.assertNotIn('uses', job, 'do not call upstream reusable workflows')
            self.assertNotIn('permissions', job)
            self.assertLessEqual(int(job['timeout-minutes']), 180)
            for step in job['steps']:
                self.assertNotIn('continue-on-error', step)
                if 'uses' not in step:
                    continue
                self.assertRegex(step['uses'], r'^[\w/-]+@[a-f0-9]{40}$')
                if step['uses'].startswith('actions/checkout@'):
                    self.assertEqual(step['with']['persist-credentials'], 'false')
                if step['uses'].startswith('actions/upload-artifact@'):
                    self.assertEqual(step['if'], 'success()')
                    self.assertEqual(step['with']['if-no-files-found'], 'error')
                    self.assertLessEqual(int(step['with']['retention-days']), 7)


if __name__ == '__main__':
    unittest.main()
