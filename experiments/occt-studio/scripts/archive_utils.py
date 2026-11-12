"""Reproducible, bounded-memory distribution archives (standard library only)."""
from __future__ import annotations

import hashlib
from pathlib import Path
import shutil
import stat
import zipfile

FONT_SUFFIXES = frozenset({'.ttf', '.otf', '.woff', '.woff2', '.ttc'})
ZIP_EPOCH = (1980, 1, 1, 0, 0, 0)
CHUNK_BYTES = 1024 * 1024


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        for chunk in iter(lambda: stream.read(CHUNK_BYTES), b''):
            digest.update(chunk)
    return digest.hexdigest()


def archive_tree(root: Path, destination: Path) -> None:
    """Archive a trusted staging directory without using filesystem timestamps.

    Dependency packages sometimes have Unix-epoch mtimes, which are outside the
    ZIP timestamp range. A fixed ZIP epoch also makes identical contents produce
    identical archives. Symlinks and standalone fonts are rejected, not followed.
    Existing archives are never replaced. At most one chunk is copied at a time.
    """
    root = root.resolve(strict=True)
    destination = destination.absolute()
    if not root.is_dir():
        raise ValueError('Archive root must be a directory')
    if destination == root or root in destination.parents:
        raise ValueError('Archive destination must be outside the staging tree')
    files = []
    for path in sorted(root.rglob('*'), key=lambda p: p.relative_to(root).as_posix()):
        if path.is_symlink():
            raise ValueError(f'Symlinks are not permitted in packages: {path}')
        if path.is_dir():
            continue
        if not stat.S_ISREG(path.stat().st_mode):
            raise ValueError(f'Non-regular package file: {path}')
        if path.suffix.casefold() in FONT_SUFFIXES:
            raise ValueError(f'Standalone fonts must not be distributed: {path}')
        files.append(path)
    created = False
    try:
        with destination.open('xb') as raw:
            created = True
            with zipfile.ZipFile(raw, 'w', compression=zipfile.ZIP_DEFLATED, compresslevel=6) as archive:
                for path in files:
                    name = (Path(root.name) / path.relative_to(root)).as_posix()
                    info = zipfile.ZipInfo(name, date_time=ZIP_EPOCH)
                    info.compress_type = zipfile.ZIP_DEFLATED
                    info.create_system = 3
                    info.external_attr = (stat.S_IFREG | 0o644) << 16
                    with path.open('rb') as source, archive.open(info, 'w', force_zip64=True) as target:
                        shutil.copyfileobj(source, target, CHUNK_BYTES)
    except BaseException:
        if created:
            destination.unlink(missing_ok=True)
        raise
