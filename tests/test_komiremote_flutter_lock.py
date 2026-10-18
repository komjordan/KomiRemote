"""Real resolver regression; opt in with KOMIREMOTE_FLUTTER=/path/to/flutter.

Uses an isolated copy, never resolves/rewrites the checkout's lockfile.
This is not a Windows compilation or bridge generation test.
"""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
FLUTTER = os.environ.get('KOMIREMOTE_FLUTTER', '')


@unittest.skipUnless(FLUTTER, 'set KOMIREMOTE_FLUTTER to Flutter 3.24.5 executable')
class FlutterLockTests(unittest.TestCase):
    def test_windows_sdk_resolves_committed_lock_without_changes(self):
        version = subprocess.run([FLUTTER, '--version', '--machine'],
                                 capture_output=True, text=True, check=True)
        self.assertEqual(json.loads(version.stdout)['frameworkVersion'], '3.24.5')
        with tempfile.TemporaryDirectory(dir=os.environ.get('TMPDIR')) as tmp:
            project = Path(tmp) / 'flutter'
            shutil.copytree(ROOT / 'flutter', project,
                            ignore=shutil.ignore_patterns('build', '.dart_tool', 'ephemeral'))
            before = {name: (project / name).read_bytes()
                      for name in ('pubspec.yaml', 'pubspec.lock')}
            result = subprocess.run([FLUTTER, 'pub', 'get', '--enforce-lockfile'],
                                    cwd=project, capture_output=True, text=True, timeout=180)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertIn('Got dependencies!', result.stdout)
            self.assertEqual(before, {name: (project / name).read_bytes() for name in before})


if __name__ == '__main__':
    unittest.main()
