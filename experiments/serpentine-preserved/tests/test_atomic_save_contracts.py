"""Native container writes must be isolated, atomic and clean on failure."""
import json
import os
from pathlib import Path
import threading
from unittest import mock
import zipfile

import pytest
from serpentine3d.fileio import native


def document(label):
    return {'format': 'serpentine3d', 'version': 2,
            'objects': [{'id': label, 'name': label, 'brep': label * 100}]}


def test_save_does_not_touch_an_unrelated_tmp_file(tmp_path):
    target = tmp_path / 'model.serp'
    unrelated = tmp_path / 'model.serp.tmp'
    unrelated.write_bytes(b'user data, not a disposable staging file')
    native._write_container(document('new'), str(target), None)
    assert unrelated.read_bytes() == b'user data, not a disposable staging file'
    with zipfile.ZipFile(target) as archive:
        assert json.loads(archive.read('document.json')) == document('new')


def test_write_failure_preserves_existing_file_and_removes_only_its_staging(tmp_path):
    target = tmp_path / 'model.serp'
    target.write_bytes(b'previous complete model')
    unrelated = tmp_path / 'user.tmp'
    unrelated.write_bytes(b'keep')
    original = zipfile.ZipFile.writestr
    def broken(self, name, *args, **kwargs):
        if name == 'document.json':
            raise OSError('simulated full disk')
        return original(self, name, *args, **kwargs)
    with mock.patch.object(zipfile.ZipFile, 'writestr', broken):
        with pytest.raises(OSError, match='full disk'):
            native._write_container(document('new'), str(target), None)
    assert target.read_bytes() == b'previous complete model'
    assert unrelated.read_bytes() == b'keep'
    assert sorted(p.name for p in tmp_path.iterdir()) == ['model.serp', 'user.tmp']


def test_replace_failure_keeps_old_model_and_cleans_new_staging(tmp_path):
    target = tmp_path / 'model.serp'
    target.write_bytes(b'old model')
    with mock.patch.object(os, 'replace', side_effect=PermissionError('model in use')):
        with pytest.raises(PermissionError, match='model in use'):
            native._write_container(document('new'), str(target), None)
    assert target.read_bytes() == b'old model'
    assert [p.name for p in tmp_path.iterdir()] == ['model.serp']


def test_concurrent_writers_each_commit_one_complete_archive(tmp_path):
    target = tmp_path / 'model.serp'
    barrier = threading.Barrier(2)
    original = os.replace
    errors, sources = [], []
    seen = set()
    gate = threading.Lock()
    def synchronized_replace(src, dst):
        # Retry calls must not re-enter the two-writer barrier. On Windows
        # simultaneous os.replace can briefly fail with a sharing violation.
        with gate:
            first_attempt = src not in seen
            if first_attempt:
                seen.add(src)
                sources.append(src)
        if first_attempt:
            barrier.wait(timeout=10)
        original(src, dst)
    def save(label):
        try:
            native._write_container(document(label), str(target), None)
        except BaseException as exc:
            errors.append(exc)
    with mock.patch.object(os, 'replace', synchronized_replace):
        threads = [threading.Thread(target=save, args=(label,)) for label in ('A', 'B')]
        for t in threads:
            t.start()
        for t in threads:
            t.join(15)
        assert not any(t.is_alive() for t in threads)
    assert not errors
    assert len(set(map(str, sources))) == 2
    with zipfile.ZipFile(target) as archive:
        assert archive.testzip() is None
        assert json.loads(archive.read('document.json')) in (document('A'), document('B'))
    assert [p.name for p in tmp_path.iterdir()] == ['model.serp']


@pytest.mark.skipif(os.name != 'nt', reason='Windows transient file sharing semantics')
def test_transient_windows_replacement_failure_retries_without_losing_data(tmp_path):
    target = tmp_path / 'model.serp'
    native._write_container(document('old'), str(target), None)
    actual_replace = os.replace
    calls = []
    def temporarily_busy(src, dst):
        calls.append((src, dst))
        if len(calls) <= 2:
            raise PermissionError('Access is denied during sharing')
        return actual_replace(src, dst)
    with mock.patch.object(os, 'replace', temporarily_busy):
        native._write_container(document('new'), str(target), None)
    assert len(calls) == 3
    with zipfile.ZipFile(target) as archive:
        assert archive.testzip() is None
        assert json.loads(archive.read('document.json')) == document('new')
    assert [p.name for p in tmp_path.iterdir()] == ['model.serp']


@pytest.mark.skipif(os.name != 'nt', reason='Windows transient file sharing semantics')
def test_permanently_locked_windows_target_has_bounded_retry_and_cleanup(tmp_path):
    target = tmp_path / 'model.serp'
    target.write_bytes(b'original and complete')
    with mock.patch.object(os, 'replace', side_effect=PermissionError('Locked')) as replace:
        with pytest.raises(PermissionError, match='Locked'):
            native._write_container(document('new'), str(target), None)
    assert replace.call_count == 8
    assert target.read_bytes() == b'original and complete'
    assert [p.name for p in tmp_path.iterdir()] == ['model.serp'


def test_sync_failure_does_not_replace_a_good_model(tmp_path):
    target = tmp_path / 'model.serp'
    target.write_bytes(b'good model')
    with mock.patch.object(os, 'fsync', side_effect=OSError('sync failure')):
        with pytest.raises(OSError, match='sync failure'):
            native._write_container(document('new'), str(target), None)
    assert target.read_bytes() == b'good model'
    assert [p.name for p in tmp_path.iterdir()] == ['model.serp']


def test_existing_tmp_symlink_cannot_redirect_a_write(tmp_path):
    victim = tmp_path / 'unrelated.txt'
    victim.write_bytes(b'untouched')
    try:
        (tmp_path / 'model.serp.tmp').symlink_to(victim)
    except OSError:
        pytest.skip('This host does not permit creating symlinks')
    native._write_container(document('new'), str(tmp_path / 'model.serp'), None)
    assert victim.read_bytes() == b'untouched'


def test_thumbnail_and_uncompressed_blobs_survive_the_transaction(tmp_path):
    target = tmp_path / 'model.serp'
    pixels = b'pixel data' * 120
    thumbnail = b'preview'
    native._write_container(document('mesh'), str(target), thumbnail, {'points/one.bin': pixels})
    with zipfile.ZipFile(target) as archive:
        assert archive.read('thumbnail.png') == thumbnail
        assert archive.read('points/one.bin') == pixels
        assert archive.getinfo('points/one.bin').compress_type == zipfile.ZIP_STORED
        assert json.loads(archive.read('meta.json'))['objects'] == 1
        assert archive.testzip() is None


def test_invalid_document_does_not_leave_a_partial_model(tmp_path):
    target = tmp_path / 'model.serp'
    target.write_bytes(b'old')
    doc = document('cycle')
    doc['loop'] = doc
    with pytest.raises(ValueError, match='Circular'):
        native._write_container(doc, str(target), None)
    assert target.read_bytes() == b'old'
    assert [p.name for p in tmp_path.iterdir()] == ['model.serp']


@pytest.mark.skipif(os.name != 'posix', reason='POSIX file permission semantics')
def test_existing_regular_file_permissions_are_preserved(tmp_path):
    import stat
    target = tmp_path / 'model.serp'
    target.write_bytes(b'old')
    target.chmod(0o640)
    native._write_container(document('new'), str(target), None)
    assert stat.S_IMODE(target.stat().st_mode) == 0o640


def test_pathlike_target_is_accepted(tmp_path):
    target = tmp_path / 'pathlike.serp'
    native._write_container(document('path'), target, None)
    with zipfile.ZipFile(target) as archive:
        assert json.loads(archive.read('document.json')) == document('path')
