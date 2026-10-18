"""Download only fixed upstream assets; operator-approved SHA256, no trust-on-first-use.

Run from repository root in a disposable Windows CI checkout, never on an installed
client. Downloads are data only: no executable in an archive is run here.
"""
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import urllib.request
import zipfile

ASSETS = {
    'engine': ('ENGINE_SHA256', 'https://github.com/rustdesk/engine/releases/download/main/windows-x64-release.zip'),
    'usbmmidd': ('USBMMIDD_SHA256', 'https://github.com/rustdesk-org/rdev/releases/download/usbmmidd_v2/usbmmidd_v2.zip'),
    'printer': ('PRINTER_SHA256', 'https://github.com/rustdesk/hbb_common/releases/download/driver/rustdesk_printer_driver_v4-1.4.zip'),
    'adapter': ('ADAPTER_SHA256', 'https://github.com/rustdesk/hbb_common/releases/download/driver/printer_driver_adapter.zip'),
}


def extract_verified(archive, destination, expected):
    """Reject digest mismatch and Windows-unsafe ZIP paths before any extraction."""
    if not re.fullmatch(r'[a-fA-F0-9]{64}', expected):
        raise ValueError('Expected an approved SHA256, exactly 64 hexadecimal characters')
    actual = hashlib.sha256(archive.read_bytes()).hexdigest()
    if actual != expected.lower():
        raise ValueError(f'SHA256 mismatch for {archive.name}: {actual}')
    with zipfile.ZipFile(archive) as z:
        seen = set()
        for info in z.infolist():
            name = info.filename.replace('\\', '/')
            parts = PurePosixPath(name).parts
            normalized = name.rstrip('/').casefold()
            if (not parts or name.startswith('/') or '..' in parts
                    or ':' in name or any(p.endswith((' ', '.')) for p in parts)
                    or any(re.fullmatch(r'(?i:con|prn|aux|nul|com[1-9]|lpt[1-9])', p.split('.')[0]) for p in parts)
                    or (info.external_attr >> 16) & 0o170000 == 0o120000
                    or normalized in seen):
                raise ValueError(f'Unsafe or duplicate ZIP member: {name}')
            seen.add(normalized)
        z.extractall(destination)
    return actual


def main():
    # Validate every input before starting network operations.
    digests = {name: os.environ[key] for name, (key, _) in ASSETS.items()}
    for digest in digests.values():
        if not re.fullmatch(r'[a-fA-F0-9]{64}', digest):
            raise ValueError('All approved SHA256 inputs are required')
    root = Path('.komiremote-downloads')
    root.mkdir(exist_ok=False)
    verified = {}
    for name, (_, url) in ASSETS.items():
        archive = root / (name + '.zip')
        with urllib.request.urlopen(url, timeout=120) as source, archive.open('wb') as target:
            shutil.copyfileobj(source, target)
        digest = extract_verified(archive, root / name, digests[name])
        verified[name] = {'url': url, 'sha256': digest}
    (root / 'verified-sha256.json').write_text(json.dumps(verified, indent=2))


if __name__ == '__main__':
    main()
