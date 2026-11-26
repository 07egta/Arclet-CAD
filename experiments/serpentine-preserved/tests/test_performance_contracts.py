"""Original-UI backend regressions; no Qt or network is required.

Set PYTHONPATH to the pinned or patched Serpentine3D checkout.
"""
import gc
import threading
import time
import weakref
from unittest import mock

import numpy as np
import pytest

from serpentine3d.core import geometry, scene as scene_mod, spatial
from serpentine3d.core.scene import Scene
from serpentine3d.core.tessellate import DisplayMesh


def indexed_mesh(n=24001, dtype=np.float32, seed=173):
    rng = np.random.default_rng(seed)
    vertices = rng.uniform(-100, 100, (n + 7, 3)).astype(dtype)
    triangles = rng.integers(0, len(vertices), (n, 3), dtype=np.uint32)
    return vertices, triangles


def assert_index_covers(index, vertices, triangles):
    assert index is not None
    assert np.array_equal(np.sort(index.order), np.arange(len(triangles)))
    assert index.starts[0] == 0
    assert index.starts[-1] == len(triangles)
    for c in range(index.count):
        members = index.order[index.starts[c]:index.starts[c + 1]]
        pts = vertices[triangles[members]]
        assert np.all(pts >= index.mins[c])
        assert np.all(pts <= index.maxs[c])


@pytest.mark.parametrize('dtype', [np.float32, np.float64])
@pytest.mark.parametrize('seed', [0, 17, 901])
def test_indexed_triangles_have_the_same_conservative_boxes(dtype, seed):
    vertices, triangles = indexed_mesh(dtype=dtype, seed=seed)
    got = spatial.build_triangle_index(vertices, triangles)
    reference = spatial.build_index(vertices[triangles])
    assert np.array_equal(got.order, reference.order)
    assert np.array_equal(got.starts, reference.starts)
    assert np.array_equal(got.mins, reference.mins)
    assert np.array_equal(got.maxs, reference.maxs)
    assert_index_covers(got, vertices, triangles)


@pytest.mark.parametrize('kind', ['flat', 'point', 'strided', 'empty'])
def test_indexed_triangles_accept_degenerate_and_strided_inputs(kind):
    vertices, triangles = indexed_mesh(n=40001)
    if kind == 'flat':
        vertices[:, 2] = 0
    elif kind == 'point':
        vertices[:] = 7
    elif kind == 'strided':
        wide = np.zeros((len(vertices), 6), vertices.dtype)
        wide[:, ::2] = vertices
        vertices = wide[:, ::2]
        wide_i = np.zeros((len(triangles), 6), triangles.dtype)
        wide_i[:, ::2] = triangles
        triangles = wide_i[:, ::2]
    else:
        triangles = triangles[:0]
    got = spatial.build_triangle_index(vertices, triangles)
    if kind == 'empty':
        assert got is None
    else:
        assert_index_covers(got, vertices, triangles)


def test_small_display_mesh_never_expands_all_triangle_corners():
    class NoGather(np.ndarray):
        def __getitem__(self, key):
            if isinstance(key, np.ndarray):
                raise AssertionError('Small meshes must not allocate corner arrays')
            return super().__getitem__(key)
    mesh = DisplayMesh()
    mesh.local = np.zeros((9, 3), np.float32).view(NoGather)
    mesh.triangles = np.array([[0, 1, 2]], np.uint32)
    assert mesh.triangle_index() is None
    assert mesh.triangle_index() is None


def test_large_display_mesh_never_expands_an_n_by_3_by_3_array():
    class NoCornerCube(np.ndarray):
        def __getitem__(self, key):
            if isinstance(key, np.ndarray) and key.ndim == 2:
                raise AssertionError('The full triangle-corner cube is forbidden')
            return super().__getitem__(key)
    vertices, triangles = indexed_mesh()
    mesh = DisplayMesh()
    mesh.local = vertices.view(NoCornerCube)
    mesh.triangles = triangles
    first = mesh.triangle_index()
    assert first is mesh.triangle_index()
    assert_index_covers(first, vertices, triangles)


@pytest.mark.parametrize('mask_kind', ['none', 'one', 'all', 'alternating', 'random'])
def test_chunk_gather_preserves_exact_order_and_owns_the_result(mask_kind):
    vertices, triangles = indexed_mesh()
    index = spatial.build_index(vertices[triangles])
    mask = np.zeros(index.count, dtype=bool)
    if mask_kind == 'one':
        mask[-1] = True
    elif mask_kind == 'all':
        mask[:] = True
    elif mask_kind == 'alternating':
        mask[::2] = True
    elif mask_kind == 'random':
        mask = np.random.default_rng(44).random(index.count) > .75
    runs = [index.order[index.starts[i]:index.starts[i+1]]
            for i in np.flatnonzero(mask)]
    expected = np.concatenate(runs) if runs else np.array([], dtype=np.int64)
    actual = index.gather(mask)
    assert np.array_equal(actual, expected)
    assert not np.shares_memory(actual, index.order)


@pytest.mark.parametrize('attribute', ['vertices', 'edge_segments', 'points'])
@pytest.mark.parametrize('far', [False, True])
def test_world_coordinate_replacement_invalidates_cached_bounds(attribute, far):
    mesh = DisplayMesh()
    origin = np.array([1e8, -2e8, 3e8]) if far else np.zeros(3)
    first = np.array([[0., 0., 0.], [1., 1., 1.]]) + origin
    data = first.reshape(1, 2, 3) if attribute == 'edge_segments' else first
    setattr(mesh, attribute, data)
    before = mesh.bounds()
    shifted = data + np.array([20., 30., 40.])
    setattr(mesh, attribute, shifted)
    after = mesh.bounds()
    assert after is not before
    assert np.allclose(after[0], origin + [20, 30, 40], rtol=0, atol=1e-5)
    assert np.allclose(after[1], origin + [21, 31, 41], rtol=0, atol=1e-5)


def test_replacing_vertices_invalidates_triangle_pick_index():
    vertices, triangles = indexed_mesh()
    mesh = DisplayMesh()
    mesh.vertices = vertices
    mesh.triangles = triangles
    first = mesh.triangle_index()
    mesh.vertices = vertices + 500
    second = mesh.triangle_index()
    assert second is not first
    assert_index_covers(second, mesh.local, triangles)


def test_replacing_segments_invalidates_edge_pick_index():
    rng = np.random.default_rng(7)
    segments = rng.uniform(0, 5, (24001, 2, 3)).astype(np.float32)
    mesh = DisplayMesh()
    mesh.edge_segments = segments
    first = mesh.segment_index()
    mesh.edge_segments = segments + 100
    second = mesh.segment_index()
    assert second is not first
    assert np.all(second.mins >= 100)


def test_tessellation_registry_does_not_retain_native_shapes():
    references = []
    for i in range(12):
        shape = geometry.make_box((i * 3., 0, 0), 1, 1, 1)
        references.append(weakref.ref(shape))
        lock = scene_mod._tess_lock(shape)
        with lock:
            pass
    del shape, lock
    gc.collect()
    assert all(ref() is None for ref in references), 'Lock registry retains freed native shapes'


def test_outstanding_lock_cannot_be_evicted_before_it_is_acquired():
    shape = geometry.make_box((0, 0, 0), 1, 1, 1)
    first = scene_mod._tess_lock(shape)
    # A worker can be descheduled between looking up and acquiring the lock.
    for i in range(1700):
        scene_mod._tess_lock(object())
    assert scene_mod._tess_lock(shape) is first


def test_lock_waiters_use_one_lock_and_no_tessellation_race():
    scene = Scene()
    obj = scene.add(geometry.make_sphere((0, 0, 0), 5))
    original = scene_mod.tessellate
    calls, results, errors = [], [], []
    barrier = threading.Barrier(8)
    def tessellate(*args, **kwargs):
        calls.append(1)
        time.sleep(.02)
        return original(*args, **kwargs)
    def read():
        try:
            barrier.wait(timeout=5)
            results.append(obj.mesh)
        except BaseException as exc:
            errors.append(exc)
    with mock.patch.object(scene_mod, 'tessellate', tessellate):
        threads = [threading.Thread(target=read) for _ in range(8)]
        for thread in threads:
            thread.start()
        for thread in threads:
            thread.join(10)
        assert not any(t.is_alive() for t in threads)
    assert not errors
    assert len(calls) == 1
    assert len(results) == 8
    assert all(result is results[0] for result in results)


def test_shape_assignment_clears_mesh_and_old_shape_bounds():
    scene = Scene()
    obj = scene.add(geometry.make_box((0, 0, 0), 1, 1, 1))
    previous_mesh = obj.mesh
    obj.bbox()
    obj.shape = geometry.make_box((20, 0, 0), 2, 2, 2)
    assert not obj.mesh_ready
    assert obj._bounds is None
    assert obj.mesh is not previous_mesh
    assert np.allclose(obj.mesh.bounds()[0], (20, 0, 0), atol=1e-6)


def test_replace_shape_does_not_carry_a_previous_native_shape_in_bounds():
    scene = Scene()
    obj = scene.add(geometry.make_box((0, 0, 0), 1, 1, 1))
    obj.bbox()
    previous = weakref.ref(obj.shape)
    new = scene.replace_shape(obj.id, geometry.make_box((5, 0, 0), 2, 2, 2))
    del obj
    gc.collect()
    assert new._bounds is None
    assert previous() is None


def test_meshing_keeps_exact_geometry_and_undo_results():
    from serpentine3d.core.history import History
    scene = Scene()
    history = History(scene)
    obj = scene.add(geometry.make_box((0, 0, 0), 2, 3, 4))
    baseline_box = obj.bbox()
    history.checkpoint('translate')
    moved = scene.replace_shape(obj.id, geometry.translate(obj.shape, (15, 0, 0)))
    assert moved.mesh.has_faces
    history.undo()
    assert np.allclose(scene.get(obj.id).bbox(), baseline_box)
    history.redo()
    assert np.allclose(scene.get(obj.id).bbox()[0], (15, 0, 0), atol=1e-6)
