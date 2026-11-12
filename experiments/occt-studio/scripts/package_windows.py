#!/usr/bin/env python3
"""Package only the actual PE dependency closure, licenses and corresponding OCCT source.

Run on the Windows CI builder after its native tests pass. Requires pefile.
No system DLL or standalone font file is copied into the distribution.
"""
from __future__ import annotations
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import urllib.request
import zipfile
import pefile


def copy_text_notices(source: Path, destination: Path) -> int:
    count = 0
    if not source.exists():
        return count
    for path in source.rglob('*'):
        if not path.is_file() or path.stat().st_size > 2_000_000:
            continue
        name = path.name.upper()
        if not any(key in name for key in ('LICENSE', 'LICENCE', 'COPYING', 'NOTICE', 'COPYRIGHT')):
            continue
        if path.suffix.lower() in ('.ttf', '.otf', '.woff', '.woff2', '.exe', '.dll'):
            continue
        try:
            path.read_text(encoding='utf-8')
        except (UnicodeError, OSError):
            continue
        target = destination / path.relative_to(source)
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(path, target)
        count += 1
    return count


def main(prefix: Path, exe: Path, output: Path) -> None:
    source = Path(__file__).resolve().parent.parent
    if output.exists():
        raise RuntimeError(f'Refusing to replace existing package directory: {output}')
    output.mkdir(parents=True)
    dlls = {}
    for folder in (prefix / 'Library' / 'bin', prefix):
        for path in folder.glob('*.dll'):
            dlls[path.name.casefold()] = path
    records = [json.loads(p.read_text()) for p in (prefix / 'conda-meta').glob('*.json')]
    owners = {}
    for record in records:
        for file in record.get('files', []):
            owners[file.replace('\\', '/').casefold()] = record
    queue = [exe]
    visited = set()
    packages = {}
    while queue:
        path = queue.pop()
        key = path.name.casefold()
        if key in visited:
            continue
        visited.add(key)
        shutil.copy2(path, output / path.name)
        if path != exe:
            relative = path.relative_to(prefix).as_posix().casefold()
            owner = owners.get(relative)
            if owner is None:
                raise RuntimeError(f'Cannot identify distribution license owner: {relative}')
            packages[owner['name']] = owner
        image = pefile.PE(str(path), fast_load=True)
        image.parse_data_directories(directories=[1, 13])
        for entry in list(getattr(image, 'DIRECTORY_ENTRY_IMPORT', [])) + list(getattr(image, 'DIRECTORY_ENTRY_DELAY_IMPORT', [])):
            name = entry.dll.decode('ascii').casefold()
            if name in dlls:
                queue.append(dlls[name])
            elif name.startswith(('api-ms-win-', 'ext-ms-win-')) or (Path(os.environ['SystemRoot']) / 'System32' / name).exists():
                pass  # Supplied by Windows, never copied from the CI machine.
            else:
                raise RuntimeError(f'Unresolved runtime dependency {name} imported by {path.name}')
        image.close()
    notices = output / 'third-party'
    notices.mkdir()
    for name, record in packages.items():
        dest = notices / f"{name}-{record['version']}"
        dest.mkdir()
        (dest / 'package-record.json').write_text(json.dumps(record, indent=2))
        cache = Path(record.get('link', {}).get('source', ''))
        if not (cache / 'info').is_dir():
            raise RuntimeError(f'Missing package provenance/licenses for {name}: {cache}')
        for folder in ('licenses', 'recipe'):
            if (cache / 'info' / folder).exists():
                shutil.copytree(cache / 'info' / folder, dest / folder)
        copied = copy_text_notices(cache / 'info', dest / 'notices')
        if copied == 0 and not (dest / 'licenses').exists():
            raise RuntimeError(f'No license text found for bundled package: {name}')
    # Include the complete unmodified upstream OCCT source and the exact package recipe/patches.
    occt = packages.get('occt')
    if not occt or occt['version'] != '7.8.1':
        raise RuntimeError('This release recipe is deliberately pinned to OCCT 7.8.1')
    source_url = 'https://github.com/Open-Cascade-SAS/OCCT/archive/refs/tags/V7_8_1.tar.gz'
    archive = notices / 'OCCT-V7_8_1-source.tar.gz'
    urllib.request.urlretrieve(source_url, archive)
    source_record = {'url': source_url, 'sha256': hashlib.sha256(archive.read_bytes()).hexdigest(),
                     'recipe': f"occt-{occt['version']}/recipe", 'modifications_by_this_application': 'none'}
    (notices / 'occt-corresponding-source.json').write_text(json.dumps(source_record, indent=2))
    # Preserve notices for the Rust dependency graph, including embedded font licenses,
    # but never copy standalone font binaries.
    metadata = json.loads(subprocess.check_output(['cargo', 'metadata', '--locked', '--format-version', '1', '--features', 'desktop', '--filter-platform', 'x86_64-pc-windows-msvc'], cwd=source))
    rust_index = []
    for package in metadata['packages']:
        if package.get('source') is None:
            continue
        root = Path(package['manifest_path']).parent
        dest = notices / 'rust' / f"{package['name']}-{package['version']}"
        count = copy_text_notices(root, dest)
        rust_index.append({'name': package['name'], 'version': package['version'], 'license': package.get('license'), 'repository': package.get('repository'), 'notice_files': count})
    (notices / 'rust-dependencies.json').write_text(json.dumps(rust_index, indent=2))
    source_out = output / 'source'
    source_out.mkdir()
    for name in ('src', 'native', 'tests', 'scripts', 'examples'):
        shutil.copytree(source / name, source_out / name)
    for name in ('Cargo.toml', 'Cargo.lock', 'build.rs', 'README.md', 'QA.md', 'LICENSE'):
        shutil.copy2(source / name, source_out / name)
    shutil.copy2(source / 'LICENSE', output / 'LICENSE')
    shutil.copy2(source / 'README.md', output / 'README.md')
    shutil.copy2(source / 'QA.md', output / 'QA.md')
    shutil.copytree(source / 'evidence' / 'showcase', output / 'samples')
    (output / 'Start-Arclet.cmd').write_text('@echo off\r\ncd /d "%~dp0"\r\narclet-occt-studio.exe --desktop --demo\r\nif errorlevel 1 pause\r\n')
    (output / 'RUNTIME-NOTICE.txt').write_text(
        'Development build, unsigned. Extract the entire directory, then run Start-Arclet.cmd.\n'
        'Uses Open CASCADE Technology (LGPL-2.1 with the OCCT exception), dynamically linked.\n'
        'The application source, dependency notices, exact package recipes and OCCT source are included.\n'
        'You may replace compatible DLLs and rebuild the application; no anti-relinking restriction applies.\n'
        'Requires a working OpenGL 3.3 driver. Native core tests do not certify every GPU.\n'
        'Keep backups of important work. Do not open untrusted CAD files on sensitive systems.\n')
    manifest = {'executable': exe.name, 'runtime_files': sorted(visited), 'packages': {k: {'version': v['version'], 'build': v['build'], 'license': v.get('license')} for k, v in packages.items()}}
    (output / 'runtime-manifest.json').write_text(json.dumps(manifest, indent=2))
    forbidden = [p for p in output.rglob('*') if p.suffix.lower() in ('.ttf', '.otf', '.woff', '.woff2')]
    if forbidden:
        raise RuntimeError(f'Standalone font files must not be packaged: {forbidden}')
    hashes = {p.relative_to(output).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest() for p in output.rglob('*') if p.is_file()}
    (output / 'SHA256SUMS.json').write_text(json.dumps(hashes, indent=2))
    with zipfile.ZipFile(output.with_suffix('.zip'), 'w', zipfile.ZIP_DEFLATED, compresslevel=6) as z:
        for p in output.rglob('*'):
            if p.is_file():
                z.write(p, (Path(output.name) / p.relative_to(output)).as_posix())
    print(json.dumps(manifest, indent=2))


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--prefix', type=Path, required=True)
    parser.add_argument('--exe', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    main(args.prefix.resolve(), args.exe.resolve(), args.output.resolve())
