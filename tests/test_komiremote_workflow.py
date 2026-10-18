"""Static safety contract, not a Windows build. Requires PyYAML 6.0.2."""
from pathlib import Path
import re
import os
import subprocess
import sys
import json
import tempfile
import zipfile
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
                         'ecd8d6a139eee76845ea66423fb739af450fda90'):
            self.assertIn(required, all_runs)

    def test_referenced_build_helpers_exist(self):
        for name in ('fetch_assets.py', 'provision_nasm.py', 'inspect_imports.ps1'):
            self.assertTrue((ROOT / '.github/komiremote' / name).is_file(), name)
        self.assertTrue((ROOT / 'docs/komiremote/NOT-STANDALONE.txt').is_file())
        inspection = (ROOT / '.github/komiremote/inspect_imports.ps1').read_text()
        for flag in ('/DEPENDENTS', '/IMPORTS', '/HEADERS'):
            self.assertIn(flag, inspection)
        self.assertIn('$LASTEXITCODE', inspection)

    def test_verified_nasm_preflight_precedes_native_build(self):
        w = yaml.load(WORKFLOW.read_text(), Loader=yaml.BaseLoader)
        runs = '\n'.join(s.get('run', '') for s in w['jobs']['windows']['steps'])
        self.assertIn('python .github/komiremote/provision_nasm.py', runs)
        self.assertLess(runs.index('provision_nasm.py'), runs.index('nasm -v'))
        self.assertLess(runs.index('nasm -v'), runs.index('vcpkg.exe" install'))
        self.assertIn('command -v nasm', runs)
        self.assertIn('os.path.samefile', runs, 'verify exported PATH resolves the verified NASM, not an image tool')
        preflight = next(s['run'] for s in w['jobs']['windows']['steps']
                         if 'os.path.samefile' in s.get('run', ''))
        match = re.search(r"python - <<'PY'\n(.*?)\nPY", preflight, re.S)
        assert match is not None
        with tempfile.TemporaryDirectory(dir=os.environ.get('TMPDIR')) as tmp:
            root = Path(tmp)
            (root / 'komiremote-test-output').mkdir()
            for label in ('verified', 'other'):
                (root / label).mkdir()
                # A path-resolution fixture, not a NASM executable. Never launched.
                stub = root / label / ('nasm.exe' if os.name == 'nt' else 'nasm')
                stub.write_bytes(b'fixture')
                stub.chmod(0o700)
            expected = root / 'verified' / stub.name
            (root / 'komiremote-test-output/nasm.json').write_text(json.dumps({'executable': str(expected)}))
            for label, returncode in (('verified', 0), ('other', 1)):
                result = subprocess.run([sys.executable, '-c', match.group(1)], cwd=root,
                                        env={**os.environ, 'PATH': str(root / label)},
                                        capture_output=True, text=True)
                self.assertEqual(result.returncode, returncode, result.stderr)

    def test_msbuild_preflight_uses_verified_executable_without_path_lookup(self):
        w = yaml.load(WORKFLOW.read_text(), Loader=yaml.BaseLoader)
        steps = w['jobs']['windows']['steps']
        preflight = next(s['run'] for s in steps if 'msbuild -version' in s.get('run', '')
                         or '"$msbuild_exe" -version' in s.get('run', ''))
        version_line = next(line.strip() for line in preflight.splitlines()
                            if 'msbuild -version' in line or '"$msbuild_exe" -version' in line)
        with tempfile.TemporaryDirectory(dir=os.environ.get('TMPDIR')) as tmp:
            # Executable fixture, not MSBuild or a Windows compile.
            executable = Path(tmp) / 'VS with spaces' / 'MSBuild.exe'
            executable.parent.mkdir()
            executable.write_text('#!/bin/bash\n[[ "$1" == "-version" ]]\n')
            executable.chmod(0o700)
            result = subprocess.run(['/bin/bash', '-euc', version_line],
                                    env={'PATH': '/nonexistent', 'msbuild_exe': str(executable)},
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
        setup = next(s for s in steps if s.get('uses', '').startswith('microsoft/setup-msbuild@'))
        self.assertEqual(next(s for s in steps if s.get('run') == preflight)['env']['MSBUILD_DIR'],
                         '${{ steps.' + setup['id'] + '.outputs.msbuildPath }}')
        self.assertIn('test -f "$msbuild_exe"', preflight)
        self.assertIn('cygpath -u "$MSBUILD_DIR"', preflight)
        self.assertIn('MSBUILD_EXE=%s', preflight)
        self.assertIn('"$GITHUB_ENV"', preflight)
        build = next(s['run'] for s in steps if 'WindowInjection.vcxproj' in s.get('run', ''))
        self.assertIn('"$MSBUILD_EXE" .komiremote-topmost/', build)

    def test_only_non_standalone_zip_until_windows_dependency_validation(self):
        text = WORKFLOW.read_text()
        self.assertNotIn('python generate.py', text)
        self.assertNotIn('rustdesk-portable-packer.exe', text)
        self.assertNotIn('unsigned.exe', text)
        self.assertIn('bundle-x64-unsigned-NOT-STANDALONE', text)
        self.assertIn('shutil.make_archive', text)
        self.assertIn('NOT-STANDALONE.txt', text)
        self.assertIn('inspect_imports.ps1', text)

    def test_zip_step_keeps_upstream_binary_names(self):
        w = yaml.load(WORKFLOW.read_text(), Loader=yaml.BaseLoader)
        run = next(s['run'] for s in w['jobs']['windows']['steps']
                   if 'shutil.make_archive' in s.get('run', ''))
        match = re.search(r"python - <<'PY'\n(.*?)\nPY", run, re.S)
        assert match is not None
        snippet = match.group(1)
        with tempfile.TemporaryDirectory(dir=os.environ.get('TMPDIR')) as tmp:
            root = Path(tmp)
            (root / 'rustdesk').mkdir()
            (root / 'komiremote-test-output').mkdir()
            for name in ('rustdesk.exe', 'librustdesk.dll', 'NOT-STANDALONE.txt'):
                (root / 'rustdesk' / name).write_bytes(b'fixture, never executed')
            result = subprocess.run([sys.executable, '-c', snippet], cwd=root, capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            archive, = (root / 'komiremote-test-output').glob('*.zip')
            with zipfile.ZipFile(archive) as z:
                self.assertTrue(all(n.startswith('rustdesk/') for n in z.namelist()))
                self.assertIn('rustdesk/rustdesk.exe', z.namelist())
                self.assertIn('rustdesk/librustdesk.dll', z.namelist())

    def test_native_build_path_without_custom_assembler_or_rename(self):
        text = WORKFLOW.read_text()
        self.assertNotIn('assemble.py', text)
        self.assertNotIn('KomiRemote.exe', text)
        self.assertNotIn('mv rustdesk ', text)
        w = yaml.load(text, Loader=yaml.BaseLoader)
        build = next(s['run'] for s in w['jobs']['windows']['steps']
                     if 'build.py' in s.get('run', ''))
        self.assertIn('mv flutter/build/windows/x64/runner/Release rustdesk', build)
        for payload in ('usbmmidd_v2', 'drivers/RustDeskPrinterDriver', 'printer_driver_adapter.dll',
                        'WindowInjection.dll', 'librustdesk.dll', 'dylib_virtual_display.dll'):
            self.assertIn(payload, build)

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
