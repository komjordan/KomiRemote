"""Compile only actual pure policy seams with cached dependencies, never the app.

Uses a public RFC 6238 fixture; does not read or write any user configuration.
"""
import json
from pathlib import Path
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[1]
OUT = Path(r'C:\kbuild\logs\komi5-native-unit')
OUT.mkdir(exist_ok=True)
lock_bytes = (ROOT / 'Cargo.lock').read_bytes()
lock = tomllib.loads(lock_bytes.decode())
versions = {}
for name in ['anyhow', 'serde', 'serde_derive', 'serde_json', 'totp-rs']:
    major = '5.' if name == 'totp-rs' else '1.'
    found = {p['version'] for p in lock['package'] if p['name'] == name and p['version'].startswith(major)}
    assert len(found) == 1, (name, found)
    versions[name] = found.pop()
manifest = '[package]\nname="komi5-policy-unit"\nversion="0.0.0"\nedition="2021"\n[dependencies]\n'
for name, version in versions.items():
    if name == 'totp-rs':
        manifest += f'{name} = {{version="={version}", default-features=false, features=["gen_secret", "otpauth"]}}\n'
    else:
        manifest += f'{name} = "={version}"\n'
(OUT / 'Cargo.toml').write_text(manifest)
(OUT / 'Cargo.lock').write_bytes(lock_bytes)
(OUT / 'src').mkdir()
auth = (ROOT / 'src/auth_2fa.rs').read_text(encoding='utf-8')
issuer_tests = auth[auth.index('fn totp_issuer('):auth.index('#[derive(Debug, Clone, Default, Serialize, Deserialize)]\npub struct TOTPInfo')]
structure = auth[auth.index('#[derive(Debug, Clone, Default, Serialize, Deserialize)]\npub struct TOTPInfo'):auth.index('impl TOTPInfo {')]
new_totp = auth[auth.index('    fn new_totp('):auth.index('    fn gen_totp_info(')]
windows = (ROOT / 'src/platform/windows.rs').read_text(encoding='utf-8')
printer = windows[windows.index('fn installer_printer_command('):windows.index('pub fn install_me(')]
# Include original name code byte-for-byte and original pure functions/test bodies.
code = 'use serde_derive::{Deserialize, Serialize};\nuse totp_rs::{Algorithm, TOTP};\ntype ResultType<T> = anyhow::Result<T>;\nfn get_app_name()->String { "KomiRemote".to_owned() }\n'
code += f'#[path = {json.dumps(str(ROOT / "src/komi_device_name.rs"))}] mod device_name;\n'
code += issuer_tests + structure + 'impl TOTPInfo {\n' + new_totp + '}\n' + printer
(OUT / 'src/lib.rs').write_text(code, encoding='utf-8')
command = ['C:\\rust\\cargo\\bin\\cargo.exe', 'test', '--offline', '--release', '--manifest-path', str(OUT / 'Cargo.toml'), '--target-dir', str(ROOT / 'target'), '--', '--nocapture']
print('Only standalone source seams: hostname, installer printer policy, TOTP RFC fixture', flush=True)
result = subprocess.run(command)
Path(r'C:\kbuild\logs\komi5-native-unit-exit.txt').write_text(str(result.returncode))
raise SystemExit(result.returncode)
