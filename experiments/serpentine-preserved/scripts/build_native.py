#!/usr/bin/env python3
"""Build/install the optional small Rust accelerator into a source checkout.

No crates are downloaded: the crate has no dependencies. A working Rust
compiler and platform linker are required. No UI or global Python is changed.
"""
from __future__ import annotations
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

BUNDLE = Path(__file__).resolve().parent.parent


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('upstream', type=Path)
    parser.add_argument('--check', action='store_true', help='Run Rust fmt, lint and tests first')
    args = parser.parse_args()
    root = args.upstream.resolve(strict=True)
    if not (root / 'serpentine3d/core/native_spatial.py').is_file():
        raise ValueError('Apply the reviewed backend patches before building the accelerator')
    crate = BUNDLE / 'native-pick-index'
    target = crate / 'target'
    env = os.environ.copy()
    env['CARGO_TARGET_DIR'] = str(target)
    # The experiment does not inherit flags or linker wrappers from Arclet's
    # unrelated parent workspace. Use portable target defaults, not host CPU.
    env['RUSTFLAGS'] = ''
    env['CARGO_ENCODED_RUSTFLAGS'] = ''
    if args.check:
        for command in (['cargo', 'fmt', '--all', '--', '--check'],
                        ['cargo', 'clippy', '--offline', '--locked', '--all-targets', '--', '-D', 'warnings'],
                        ['cargo', 'test', '--offline', '--locked']):
            subprocess.run(command, cwd=crate, env=env, check=True)
    subprocess.run(['cargo', 'build', '--offline', '--locked', '--release'], cwd=crate, env=env, check=True)
    name = {'win32': 'serpentine_pick_index.dll', 'darwin': 'libserpentine_pick_index.dylib',
            'linux': 'libserpentine_pick_index.so'}.get(sys.platform)
    if name is None:
        raise RuntimeError('This operating system has no tested accelerator filename')
    source = target / 'release' / name
    destination = root / 'serpentine3d' / '_native'
    destination.mkdir(exist_ok=True)
    # Never expose a half-written library to another starting app.
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(dir=destination, prefix='.native-', delete=False) as stream:
            temporary = Path(stream.name)
            with source.open('rb') as data:
                shutil.copyfileobj(data, stream)
        os.replace(temporary, destination / name)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)
    record = {'library': name, 'bytes': source.stat().st_size,
              'sha256': hashlib.sha256(source.read_bytes()).hexdigest(), 'abi': 1,
              'compiler': subprocess.check_output(['rustc', '--version'], text=True).strip(),
              'source': 'experiments/serpentine-preserved/native-pick-index',
              'dependencies': [], 'cpu_flags': 'portable compiler defaults'}
    (destination / 'build.json').write_text(json.dumps(record, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(record, indent=2))


if __name__ == '__main__':
    main()
