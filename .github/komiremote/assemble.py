"""Assemble upstream x64 portable payload; do not run clients/installers/drivers."""
from pathlib import Path
import shutil


def require(path):
    if not path.is_file() or not path.stat().st_size:
        raise FileNotFoundError(f'Missing or empty required payload: {path}')


def assemble(root=Path('.')):
    bundle = root / 'rustdesk'
    downloads = root / '.komiremote-downloads'
    for name in ('rustdesk.exe', 'librustdesk.dll', 'flutter_windows.dll', 'dylib_virtual_display.dll'):
        require(bundle / name)
    if not (bundle / 'data/flutter_assets').is_dir():
        raise FileNotFoundError('Flutter assets missing')
    usb = downloads / 'usbmmidd/usbmmidd_v2'
    # Match upstream removal of 32-bit payload and helper installation entrypoints.
    shutil.rmtree(usb / 'Win32')
    for name in ('deviceinstaller64.exe', 'deviceinstaller.exe', 'usbmmidd.bat'):
        (usb / name).unlink()
    shutil.copytree(usb, bundle / 'usbmmidd_v2')
    printer = downloads / 'printer/rustdesk_printer_driver_v4-1.4'
    if not printer.is_dir() or not any(printer.rglob('*.inf')):
        raise FileNotFoundError('Printer driver archive layout invalid')
    shutil.copytree(printer, bundle / 'drivers/RustDeskPrinterDriver')
    adapter = downloads / 'adapter/printer_driver_adapter.dll'
    topmost = root / '.komiremote-topmost/WindowInjection/x64/Release/WindowInjection.dll'
    for source in (adapter, topmost):
        require(source)
        shutil.copy2(source, bundle / source.name)


if __name__ == '__main__':
    assemble()
