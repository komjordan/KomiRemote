"""Local ZIP/payload fixtures only: never download or execute Windows code."""
import hashlib
import importlib.util
import os
from pathlib import Path
import tempfile
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]


def load(name):
    spec = importlib.util.spec_from_file_location(name, ROOT / '.github/komiremote' / (name + '.py'))
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


FETCH = load('fetch_assets')
ASSEMBLE = load('assemble')


class AssetSafetyTests(unittest.TestCase):
    def test_hash_and_path_validation_precede_extraction(self):
        with tempfile.TemporaryDirectory(dir=os.environ.get('TMPDIR')) as tmp:
            root = Path(tmp)
            archive = root / 'fixture.zip'
            for member in ('../escape', '/absolute', 'C:/absolute', 'folder/../escape',
                           'folder\\..\\escape', 'file:stream', 'folder./file', 'CON'):
                with self.subTest(member=member):
                    with zipfile.ZipFile(archive, 'w') as z:
                        z.writestr(member, b'test fixture, not a real binary')
                    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
                    destination = root / 'out'
                    with self.assertRaises(ValueError):
                        FETCH.extract_verified(archive, destination, digest)
                    self.assertFalse(destination.exists())
            with zipfile.ZipFile(archive, 'w') as z:
                z.writestr('safe/test.txt', b'fixture')
            for digest in ('0' * 64, '', 'x' * 64):
                with self.assertRaises(ValueError):
                    FETCH.extract_verified(archive, root / 'bad', digest)
                self.assertFalse((root / 'bad').exists())
            digest = hashlib.sha256(archive.read_bytes()).hexdigest()
            self.assertEqual(FETCH.extract_verified(archive, root / 'good', digest), digest)
            self.assertEqual((root / 'good/safe/test.txt').read_bytes(), b'fixture')

    def test_assemble_includes_required_payload_without_install_helpers(self):
        with tempfile.TemporaryDirectory(dir=os.environ.get('TMPDIR')) as tmp:
            root = Path(tmp)
            for name in ('rustdesk.exe', 'librustdesk.dll', 'flutter_windows.dll', 'dylib_virtual_display.dll'):
                path = root / 'rustdesk' / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b'fixture, not a real binary')
            (root / 'rustdesk/data/flutter_assets').mkdir(parents=True)
            usb = root / '.komiremote-downloads/usbmmidd/usbmmidd_v2'
            (usb / 'Win32').mkdir(parents=True)
            for name in ('deviceinstaller64.exe', 'deviceinstaller.exe', 'usbmmidd.bat', 'usbmmidd.inf'):
                (usb / name).write_bytes(b'fixture')
            for name in ('.komiremote-downloads/printer/rustdesk_printer_driver_v4-1.4/fixture.inf',
                         '.komiremote-downloads/adapter/printer_driver_adapter.dll',
                         '.komiremote-topmost/WindowInjection/x64/Release/WindowInjection.dll'):
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b'fixture')
            ASSEMBLE.assemble(root)
            for name in ('usbmmidd_v2/usbmmidd.inf', 'drivers/RustDeskPrinterDriver/fixture.inf',
                         'printer_driver_adapter.dll', 'WindowInjection.dll'):
                self.assertTrue((root / 'rustdesk' / name).is_file())
            for name in ('Win32', 'deviceinstaller64.exe', 'deviceinstaller.exe', 'usbmmidd.bat'):
                self.assertFalse((root / 'rustdesk/usbmmidd_v2' / name).exists())
            (root / 'rustdesk/librustdesk.dll').unlink()
            with self.assertRaises(FileNotFoundError):
                ASSEMBLE.assemble(root)


if __name__ == '__main__':
    unittest.main()
