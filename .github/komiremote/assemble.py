"""Assemble an unsigned test bundle, NOT a validated standalone portable.

Presence/layout checks only; INF semantics, PE closure and Windows runtime still
require review. Never run clients, installers or drivers here.
"""
from pathlib import Path, PureWindowsPath
import re
import shutil


def require(path):
    if not path.is_file() or not path.stat().st_size:
        raise FileNotFoundError(f'Missing or empty required payload: {path}')


def windows_file(root, name):
    """Resolve a safe relative path case-insensitively, also on Linux fixtures."""
    path = PureWindowsPath(name.strip().strip('"'))
    if path.drive or path.root or not path.parts or '..' in path.parts or ':' in str(path):
        raise ValueError(f'Unsafe payload reference: {name}')
    target = root
    for part in path.parts:
        matches = [p for p in target.iterdir() if p.name.casefold() == part.casefold()] if target.is_dir() else []
        if len(matches) != 1:
            raise FileNotFoundError(f'Missing or ambiguous payload reference: {root}/{name}')
        target = matches[0]
    require(target)
    return target


def validate_driver(root, inf_name):
    """Conservative flat INF check, not a SetupAPI parser/signature validation.

    Require the exact runtime INF, its catalogs and SourceDisksFiles entries.
    Unknown/indirect layouts fail for review rather than certifying a partial ZIP.
    """
    raw = windows_file(root, inf_name).read_bytes()
    text = raw.decode('utf-16') if raw.startswith((b'\xff\xfe', b'\xfe\xff')) else raw.decode('utf-8-sig', errors='strict')
    section = ''
    catalogs, sources = [], []
    disks = {}
    for line in text.splitlines():
        line = line.split(';', 1)[0].strip()
        if line.startswith('[') and line.endswith(']'):
            section = line[1:-1].casefold()
        elif '=' in line:
            key, value = (x.strip() for x in line.split('=', 1))
            if section == 'version' and re.fullmatch(r'catalogfile(?:\..+)?', key.casefold()):
                catalogs.append(value)
            elif section in ('sourcedisksnames', 'sourcedisksnames.amd64'):
                fields = value.split(',')
                # INF disk-root paths use one leading backslash, relative to media.
                disk_root = fields[3].strip().strip('"') if len(fields) > 3 else ''
                if disk_root.startswith('\\\\'):
                    raise ValueError('UNC disk-root path is unsupported')
                disk_root = disk_root.removeprefix('\\')
                path = PureWindowsPath(disk_root)
                if path.drive or path.root or '..' in path.parts or ':' in disk_root or '%' in disk_root:
                    raise ValueError(f'Unsafe or indirect INF disk-root path: {disk_root}')
                entries = disks.setdefault(section, {})
                if key in entries:
                    raise ValueError(f'Duplicate INF disk identifier: {key}')
                entries[key] = disk_root
            elif section in ('sourcedisksfiles', 'sourcedisksfiles.amd64'):
                # Check the x64 payload only; Win32 is intentionally pruned later.
                fields = value.split(',')
                subdir = fields[1].strip().strip('"') if len(fields) > 1 else ''
                sources.append((fields[0].strip(), subdir, key.strip('"')))
    if not catalogs or not sources:
        raise ValueError(f'Unreviewed INF layout (catalog/source list missing): {inf_name}')
    for name in catalogs:
        windows_file(root, name)
    for disk, subdir, name in sources:
        locations = {**disks.get('sourcedisksnames', {}), **disks.get('sourcedisksnames.amd64', {})}
        if disks and disk not in locations:
            raise ValueError(f'Unknown INF source disk: {disk}')
        # Validate each part separately: an absolute subdir must not discard disk_root.
        for component in (subdir, name):
            path = PureWindowsPath(component)
            if path.drive or path.root or '..' in path.parts or ':' in component or '%' in component:
                raise ValueError(f'Unsafe or indirect INF source: {component}')
        windows_file(root, str(PureWindowsPath(locations.get(disk, '')) / subdir / name))


def validate_install_manifest(bundle, manifest):
    """Check actual CMake install outputs, including generated plugin/CRT copies.

    This does not enumerate DLLs loaded dynamically or absent from the install list.
    """
    require(manifest)
    names = []
    for line in manifest.read_text(encoding='utf-8').splitlines():
        line = line.replace('\\', '/')
        marker = '/runner/Release/'
        if marker not in line:
            raise ValueError(f'Unexpected CMake install destination: {line}')
        name = line.split(marker, 1)[1]
        windows_file(bundle, name)
        names.append(name.casefold())
    # Minimum core components installed by Flutter's CMake runner
    core_expected = {'rustdesk.exe', 'flutter_windows.dll'}
    missing_core = core_expected - set(names)
    if missing_core:
        raise ValueError(f'Incomplete CMake install manifest, missing core components: {sorted(missing_core)}')
    if not any(n.endswith('_plugin.dll') for n in names):
        raise ValueError('No generated plugin DLL in CMake install manifest')


def assemble(root=Path('.')):
    bundle = root / 'rustdesk'
    downloads = root / '.komiremote-downloads'
    for name in ('rustdesk.exe', 'librustdesk.dll', 'flutter_windows.dll', 'dylib_virtual_display.dll',
                 'data/icudtl.dat', 'data/app.so',
                 'data/flutter_assets/AssetManifest.bin', 'data/flutter_assets/assets/icon.png'):
        require(bundle / name)
    validate_install_manifest(bundle, root / 'flutter/build/windows/x64/install_manifest.txt')
    usb = downloads / 'usbmmidd/usbmmidd_v2'
    printer = downloads / 'printer/rustdesk_printer_driver_v4-1.4'
    validate_driver(usb, 'usbmmIdd.inf')
    validate_driver(printer, 'RustDeskPrinterDriver.inf')
    adapter = downloads / 'adapter/printer_driver_adapter.dll'
    topmost = root / '.komiremote-topmost/WindowInjection/x64/Release/WindowInjection.dll'
    for source in (adapter, topmost):
        require(source)
    if (bundle / 'KomiRemote.exe').exists():
        raise FileExistsError('Refusing to overwrite KomiRemote.exe')
    # Match upstream removal of 32-bit payload and helper installation entrypoints.
    shutil.copytree(usb, bundle / 'usbmmidd_v2')
    shutil.rmtree(bundle / 'usbmmidd_v2/Win32')
    for name in ('deviceinstaller64.exe', 'deviceinstaller.exe', 'usbmmidd.bat'):
        (bundle / 'usbmmidd_v2' / name).unlink()
    # Verify again after pruning: a declared dependency must not have been removed.
    validate_driver(bundle / 'usbmmidd_v2', 'usbmmIdd.inf')
    shutil.copytree(printer, bundle / 'drivers/RustDeskPrinterDriver')
    for source in (adapter, topmost):
        shutil.copy2(source, bundle / source.name)
    # Packaging-only rename: DLL basename/FFI exports and CMake inputs stay upstream.
    # A future reviewed wrapper would derive localappdata/komiremote from this stem.
    (bundle / 'rustdesk.exe').rename(bundle / 'KomiRemote.exe')


if __name__ == '__main__':
    assemble()
