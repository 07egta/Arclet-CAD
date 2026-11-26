"""Conformance tests for the optional native ABI and conservative grid."""
import gc
import os
from unittest import mock

import numpy as np
import pytest
from serpentine3d.core import spatial
from serpentine3d.core import native_spatial as native


@pytest.fixture(autouse=True)
def require_accelerator():
    if not native.backend_status()['available']:
        if os.environ.get('SERP3D_REQUIRE_NATIVE_TESTS') == '1':
            pytest.fail('Native tests required but the accelerator is unavailable')
        pytest.skip('Optional native library not built for this host')


@pytest.mark.parametrize('grid', [1, 2, 7, 31, 64])
@pytest.mark.parametrize('kind', ['random', 'sheet', 'point', 'tiny', 'far', 'unused'])
def test_exact_native_chunk_arrays_match_numpy(grid, kind, monkeypatch):
    rng = np.random.default_rng(701 + grid)
    vertices = rng.uniform(-200, 200, (307, 3)).astype(np.float32)
    triangles = rng.integers(0, 300, (20003, 3), dtype=np.uint32)
    if kind == 'sheet':
        vertices[:, 1] = 0
    elif kind == 'point':
        vertices[:] = 5
    elif kind == 'tiny':
        vertices *= 1e-15
    elif kind == 'far':
        vertices += 1e7
    elif kind == 'unused':
        vertices[300:] = np.nan
    monkeypatch.setattr(spatial, '_grid_size', lambda n: grid)
    reference = spatial.build_index(vertices[triangles])
    result = native.native_triangle_index(vertices, triangles, grid)
    assert result is not None
    for got, expected in zip(result, (reference.order, reference.starts, reference.mins, reference.maxs)):
        np.testing.assert_array_equal(got, expected)
        assert got.dtype == expected.dtype
        assert got.flags.owndata


@pytest.mark.parametrize('kind', ['float64', 'int64', 'strided', 'misaligned', 'big_endian'])
def test_unsupported_storage_stays_on_the_numpy_path(kind):
    vertices = np.zeros((100, 3), dtype=np.float32)
    triangles = np.zeros((20001, 3), dtype=np.uint32)
    if kind == 'float64':
        vertices = vertices.astype(np.float64)
    elif kind == 'int64':
        triangles = triangles.astype(np.int64)
    elif kind == 'strided':
        vertices = np.zeros((100, 6), np.float32)[:, ::2]
    elif kind == 'misaligned':
        vertices = np.ndarray((100, 3), dtype=np.float32, buffer=bytearray(1201), offset=1)
        assert not vertices.flags.aligned
    else:
        vertices = vertices.astype('>f4')
    assert native.native_triangle_index(vertices, triangles, 8) is None


@pytest.mark.parametrize('value', [np.nan, np.inf, -np.inf, np.finfo(np.float32).max])
def test_nonfinite_or_overflowing_midpoint_uses_existing_fallback(value):
    vertices = np.full((3, 3), value, np.float32)
    triangles = np.array([[0, 1, 2]], np.uint32)
    assert native.native_triangle_index(vertices, triangles, 8) is None


def test_bad_vertex_index_is_a_python_exception_not_a_native_read():
    vertices = np.zeros((3, 3), np.float32)
    for invalid in (3, 2**32 - 1):
        with pytest.raises(IndexError, match='outside'):
            native.native_triangle_index(vertices, np.array([[0, 1, invalid]], np.uint32), 8)


def test_outputs_survive_foreign_owner_free_and_future_calls():
    vertices = np.array([[0, 0, 0], [2, 3, 4]], np.float32)
    triangles = np.array([[0, 1, 0]] * 20001, np.uint32)
    got = native.native_triangle_index(vertices, triangles, 8)
    expected = [a.copy() for a in got]
    for _ in range(20):
        native.native_triangle_index(vertices + 10, triangles, 8)
    gc.collect()
    for a, b in zip(got, expected):
        np.testing.assert_array_equal(a, b)


def test_python_override_never_loads_a_library(monkeypatch):
    monkeypatch.setenv('SERP3D_PICK_BACKEND', 'python')
    with mock.patch.object(native, '_load', side_effect=AssertionError('must not load')):
        assert native.native_triangle_index(np.zeros((3, 3), np.float32), np.zeros((1, 3), np.uint32), 1) is None


def test_required_library_cannot_silently_become_a_fallback(monkeypatch):
    monkeypatch.setenv('SERP3D_PICK_BACKEND', 'rust')
    with mock.patch.object(native, '_load', return_value=(None, 'test missing')):
        with pytest.raises(RuntimeError, match='required but unavailable'):
            native.native_triangle_index(np.zeros((3, 3), np.float32), np.zeros((1, 3), np.uint32), 1)


def test_foreign_owner_is_freed_if_numpy_copy_raises():
    lib, _ = native._load()
    freed = []
    class Proxy:
        serp_pick_build_f32 = lib.serp_pick_build_f32
        serp_pick_view = lib.serp_pick_view
        def serp_pick_free(self, handle):
            freed.append(handle.value)
            lib.serp_pick_free(handle)
    with mock.patch.object(native, '_load', return_value=(Proxy(), None)):
        with mock.patch.object(np.ctypeslib, 'as_array', side_effect=MemoryError('copy failed')):
            with pytest.raises(MemoryError, match='copy failed'):
                native.native_triangle_index(np.zeros((3, 3), np.float32), np.zeros((1, 3), np.uint32), 1)
    assert len(freed) == 1 and freed[0]


def test_native_and_python_integration_select_the_same_candidates(monkeypatch):
    rng = np.random.default_rng(75)
    vertices = rng.uniform(-5, 5, (601, 3)).astype(np.float32)
    triangles = rng.integers(0, len(vertices), (80003, 3), dtype=np.uint32)
    monkeypatch.setenv('SERP3D_PICK_BACKEND', 'python')
    reference = spatial.build_triangle_index(vertices, triangles)
    monkeypatch.setenv('SERP3D_PICK_BACKEND', 'rust')
    accelerated = spatial.build_triangle_index(vertices, triangles)
    for seed in range(10):
        mask = np.random.default_rng(seed).random(reference.count) > .3
        np.testing.assert_array_equal(accelerated.gather(mask), reference.gather(mask))


@pytest.mark.parametrize('seed', [1, 11, 25])
def test_signed_zero_extents_keep_the_reference_bits(seed):
    rng = np.random.default_rng(seed)
    vertices = rng.choice(np.array([-0.0, 0.0], np.float32), (100, 3))
    triangles = rng.integers(0, 100, (20001, 3), dtype=np.uint32)
    reference = spatial.build_index(vertices[triangles])
    result = native.native_triangle_index(vertices, triangles, spatial._grid_size(len(triangles)))
    for got, expected in zip(result, (reference.order, reference.starts, reference.mins, reference.maxs)):
        assert got.dtype == expected.dtype
        assert got.tobytes() == expected.tobytes()
