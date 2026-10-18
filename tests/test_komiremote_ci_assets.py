"""Local ZIP/payload fixtures only: never download or execute Windows code."""
import hashlib
import importlib.util
import os
from pathlib import Path
import tempfile
import sys
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / '.github/komiremote'))


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

    def test_sha512_pinned_tool_is_verified_before_extraction(self):
        with tempfile.TemporaryDirectory(dir=os.environ.get('TMPDIR')) as tmp:
            root = Path(tmp)
            archive = root / 'tool.zip'
            with zipfile.ZipFile(archive, 'w') as z:
                z.writestr('nasm-3.01/nasm.exe', b'fixture, never executed')
            with self.assertRaises(ValueError):
                FETCH.extract_verified(archive, root / 'bad', '0' * 128, algorithm='sha512')
            self.assertFalse((root / 'bad').exists())
            digest = hashlib.sha512(archive.read_bytes()).hexdigest()
            self.assertEqual(FETCH.extract_verified(archive, root / 'good', digest, algorithm='sha512'), digest)
            self.assertTrue((root / 'good/nasm-3.01/nasm.exe').is_file())

    def test_nasm_provision_uses_verified_versioned_path(self):
        nasm = load('provision_nasm')
        with tempfile.TemporaryDirectory(dir=os.environ.get('TMPDIR')) as tmp:
            root = Path(tmp)
            archive = root / 'nasm.zip'
            with zipfile.ZipFile(archive, 'w') as z:
                z.writestr('nasm-3.01/nasm.exe', b'fixture, never executed')
            digest = hashlib.sha512(archive.read_bytes()).hexdigest()
            with self.assertRaises(ValueError):
                nasm.install(archive, root / 'bad', '0' * 128)
            self.assertFalse((root / 'bad').exists())
            exe = nasm.install(archive, root / 'tool', digest)
            self.assertEqual(exe, (root / 'tool/nasm-3.01/nasm.exe').resolve())
            self.assertEqual(nasm.VERSION, '3.01')
            self.assertEqual(len(nasm.SHA512), 128)

    def test_inf_requires_runtime_name_catalog_and_declared_driver_binary(self):
        with tempfile.TemporaryDirectory(dir=os.environ.get('TMPDIR')) as tmp:
            root = Path(tmp)
            (root / 'fixture.inf').write_text('[Version]\n')
            with self.assertRaises(FileNotFoundError):
                ASSEMBLE.validate_driver(root, 'RustDeskPrinterDriver.inf')
            inf = root / 'usbmmidd.inf'
            inf.write_text('[Version]\nCatalogFile=usb.cat\n[SourceDisksFiles.amd64]\nusb.dll=1\n')
            for missing in ('usb.cat', 'usb.dll'):
                with self.assertRaises(FileNotFoundError):
                    ASSEMBLE.validate_driver(root, 'usbmmIdd.inf')
                (root / missing).write_bytes(b'fixture')
            ASSEMBLE.validate_driver(root, 'usbmmIdd.inf')
            inf.write_text('[Version]\nCatalogFile=../escape.cat\n[SourceDisksFiles]\nusb.dll=1\n')
            with self.assertRaises(ValueError):
                ASSEMBLE.validate_driver(root, 'usbmmIdd.inf')

    def test_cmake_manifest_checks_actual_plugin_and_crt_outputs(self):
        with tempfile.TemporaryDirectory(dir=os.environ.get('TMPDIR')) as tmp:
            root = Path(tmp)
            bundle = root / 'rustdesk'
            bundle.mkdir()
            manifest = root / 'install_manifest.txt'
            names = ('rustdesk.exe', 'flutter_windows.dll', 'librustdesk.dll',
                     'example_plugin.dll', 'vcruntime140.dll')
            manifest.write_text('\n'.join('C:/build/runner/Release/' + n for n in names))
            for name in names:
                with self.assertRaises(FileNotFoundError):
                    ASSEMBLE.validate_install_manifest(bundle, manifest)
                (bundle / name).write_bytes(b'fixture')
            ASSEMBLE.validate_install_manifest(bundle, manifest)

    def test_rejects_incomplete_runtime_payload(self):
        with tempfile.TemporaryDirectory(dir=os.environ.get('TMPDIR')) as tmp:
            root = Path(tmp)
            for name in ('rustdesk.exe', 'librustdesk.dll', 'flutter_windows.dll', 'dylib_virtual_display.dll'):
                p = root / 'rustdesk' / name
                p.parent.mkdir(parents=True, exist_ok=True)
                p.write_bytes(b'fixture')
            (root / 'rustdesk/data/flutter_assets').mkdir(parents=True)
            # The old assembler accepted these empty assets and arbitrary driver INF.
            usb = root / '.komiremote-downloads/usbmmidd/usbmmidd_v2'
            (usb / 'Win32').mkdir(parents=True)
            for name in ('deviceinstaller64.exe', 'deviceinstaller.exe', 'usbmmidd.bat'):
                (usb / name).write_bytes(b'fixture')
            for name in ('.komiremote-downloads/printer/rustdesk_printer_driver_v4-1.4/fixture.inf',
                         '.komiremote-downloads/adapter/printer_driver_adapter.dll',
                         '.komiremote-topmost/WindowInjection/x64/Release/WindowInjection.dll'):
                p = root / name
                p.parent.mkdir(parents=True, exist_ok=True)
                p.write_bytes(b'fixture')
            with self.assertRaises(FileNotFoundError):
                ASSEMBLE.assemble(root)

    def test_assemble_includes_required_payload_without_install_helpers(self):
        with tempfile.TemporaryDirectory(dir=os.environ.get('TMPDIR')) as tmp:
            root = Path(tmp)
            for name in ('rustdesk.exe', 'librustdesk.dll', 'flutter_windows.dll', 'dylib_virtual_display.dll'):
                path = root / 'rustdesk' / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b'fixture, not a real binary')
            for name in ('data/icudtl.dat', 'data/app.so', 'data/flutter_assets/AssetManifest.bin',
                         'data/flutter_assets/assets/icon.png'):
                p = root / 'rustdesk' / name
                p.parent.mkdir(parents=True, exist_ok=True)
                p.write_bytes(b'fixture')
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
            # Minimal INF fixtures with actual runtime basenames, not real drivers.
            for directory, inf_name in ((usb, 'usbmmidd.inf'),
                                       (root / '.komiremote-downloads/printer/rustdesk_printer_driver_v4-1.4',
                                        'RustDeskPrinterDriver.inf')):
                (directory / inf_name).write_text('[Version]\nCatalogFile=fixture.cat\n[SourceDisksFiles.amd64]\nfixture.dll=1\n')
                for name in ('fixture.cat', 'fixture.dll'):
                    (directory / name).write_bytes(b'fixture')
            manifest = root / 'flutter/build/windows/x64/install_manifest.txt'
            manifest.parent.mkdir(parents=True)
            (root / 'rustdesk/example_plugin.dll').write_bytes(b'fixture')
            manifest.write_text('\n'.join('C:/build/runner/Release/' + n for n in (
                'rustdesk.exe', 'flutter_windows.dll', 'librustdesk.dll', 'example_plugin.dll')))
            for relative in ('rustdesk/data/icudtl.dat', 'rustdesk/data/app.so',
                             'rustdesk/data/flutter_assets/AssetManifest.bin',
                             'rustdesk/example_plugin.dll',
                             '.komiremote-downloads/usbmmidd/usbmmidd_v2/usbmmidd.inf',
                             '.komiremote-downloads/usbmmidd/usbmmidd_v2/fixture.cat',
                             '.komiremote-downloads/usbmmidd/usbmmidd_v2/fixture.dll',
                             '.komiremote-downloads/printer/rustdesk_printer_driver_v4-1.4/RustDeskPrinterDriver.inf'):
                with self.subTest(missing=relative):
                    path = root / relative
                    original = path.read_bytes()
                    path.unlink()
                    with self.assertRaises(FileNotFoundError):
                        ASSEMBLE.assemble(root)
                    path.write_bytes(original)
                    self.assertFalse((root / 'rustdesk/KomiRemote.exe').exists())
            ASSEMBLE.assemble(root)
            self.assertTrue((root / 'rustdesk/KomiRemote.exe').is_file())
            self.assertFalse((root / 'rustdesk/rustdesk.exe').exists())
            self.assertTrue((root / 'rustdesk/librustdesk.dll').is_file())
            for name in ('usbmmidd_v2/usbmmidd.inf', 'drivers/RustDeskPrinterDriver/RustDeskPrinterDriver.inf',
                         'printer_driver_adapter.dll', 'WindowInjection.dll'):
                self.assertTrue((root / 'rustdesk' / name).is_file())
            for name in ('Win32', 'deviceinstaller64.exe', 'deviceinstaller.exe', 'usbmmidd.bat'):
                self.assertFalse((root / 'rustdesk/usbmmidd_v2' / name).exists())
            (root / 'rustdesk/librustdesk.dll').unlink()
            with self.assertRaises(FileNotFoundError):
                ASSEMBLE.assemble(root)


if __name__ == '__main__':
    unittest.main()
