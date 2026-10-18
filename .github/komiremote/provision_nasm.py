"""Install CI-local NASM; checksum provenance is the pinned vcpkg tool recipe.

https://github.com/microsoft/vcpkg/blob/9e593bb18ea69cc5095e012465dcd675a822ed0d/scripts/cmake/vcpkg_find_acquire_program(NASM).cmake
This is a build-tool pin, NOT approval of any of the four native payload assets.
"""
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import urllib.request

from fetch_assets import extract_verified

VERSION = '3.01'
URL = f'https://www.nasm.us/pub/nasm/releasebuilds/{VERSION}/win64/nasm-{VERSION}-win64.zip'
SHA512 = '771c238ddb17c98d5736ccaba4ade1d1601d896f09e588489cb43a4f6381bc0ae14d1869f5316fe94f847f54867e65cf12665529b1e7ad88e5e7d3e162719a4f'


def install(archive, destination, expected=SHA512):
    extract_verified(archive, destination, expected, algorithm='sha512')
    exe = destination / f'nasm-{VERSION}/nasm.exe'
    if not exe.is_file() or not exe.stat().st_size:
        raise FileNotFoundError(f'Pinned NASM layout missing: {exe}')
    return exe.resolve()


def main():
    root = Path(os.environ['RUNNER_TEMP']) / 'komiremote-nasm'
    root.mkdir(exist_ok=False)
    archive = root / 'nasm.zip'
    with urllib.request.urlopen(URL, timeout=120) as source, archive.open('wb') as target:
        shutil.copyfileobj(source, target)
    exe = install(archive, root / 'payload')
    version = subprocess.check_output([str(exe), '-v'], text=True).strip()
    if not re.match(r'NASM version 3\.01(?:\s|$)', version):
        raise RuntimeError(f'Unexpected NASM version: {version}')
    with Path(os.environ['GITHUB_PATH']).open('a', encoding='utf-8') as paths:
        paths.write(str(exe.parent) + '\n')
    output = Path('komiremote-test-output')
    output.mkdir(exist_ok=True)
    (output / 'nasm.json').write_text(json.dumps({
        'url': URL, 'sha512': SHA512, 'version': version, 'executable': str(exe),
        'provenance': 'vcpkg 9e593bb18ea69cc5095e012465dcd675a822ed0d NASM recipe',
    }, indent=2))
    print(f'Verified NASM: {exe}: {version}')


if __name__ == '__main__':
    main()
