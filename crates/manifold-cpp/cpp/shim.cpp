// Arclet-owned C ABI adapter for Apache-2.0 Manifold.
// Only owned scalar buffers cross the boundary; C++ exceptions never do.
#include <manifold/manifold.h>

#include <cmath>
#include <cstddef>
#include <cstdint>
#include <limits>
#include <memory>
#include <new>
#include <vector>

namespace {
constexpr size_t kMaxVertices = 1000000;
constexpr size_t kMaxTriangles = 2000000;
constexpr size_t kMaxResultVertices = 4000000;
constexpr size_t kMaxResultTriangles = 8000000;
constexpr double kMaxCoord = 1.0e9;

bool valid_input(const double* xyz, size_t nv, const uint32_t* tri,
                 size_t nt) noexcept {
  if (!xyz || !tri || nv < 4 || nt < 4 || nv > kMaxVertices ||
      nt > kMaxTriangles) return false;
  for (size_t i = 0; i < nv * 3; ++i) {
    if (!std::isfinite(xyz[i]) || std::abs(xyz[i]) > kMaxCoord) return false;
  }
  for (size_t i = 0; i < nt; ++i) {
    const auto a = tri[3 * i];
    const auto b = tri[3 * i + 1];
    const auto c = tri[3 * i + 2];
    if (a >= nv || b >= nv || c >= nv || a == b || a == c || b == c)
      return false;
  }
  return true;
}

manifold::Manifold as_manifold(const double* xyz, size_t nv,
                               const uint32_t* tri, size_t nt) {
  manifold::MeshGL64 mesh;
  mesh.numProp = 3;
  mesh.vertProperties.assign(xyz, xyz + nv * 3);
  // MeshGL64 uses 64-bit indices in Manifold 3.x; the Arclet public API is u32.
  mesh.triVerts.assign(tri, tri + nt * 3);
  return manifold::Manifold(mesh);
}
}  // namespace

struct ac_manifold_result {
  std::vector<double> xyz;
  std::vector<uint32_t> indices;
};

extern "C" {

int ac_manifold_boolean(
    const double* a_xyz, size_t na, const uint32_t* a_tri, size_t ta,
    const double* b_xyz, size_t nb, const uint32_t* b_tri, size_t tb,
    int operation, ac_manifold_result** out) noexcept {
  if (!out) return 1;
  *out = nullptr;
  if (!valid_input(a_xyz, na, a_tri, ta) ||
      !valid_input(b_xyz, nb, b_tri, tb) ||
      (operation != 0 && operation != 1 && operation != 2)) return 1;
  try {
    const manifold::Manifold a = as_manifold(a_xyz, na, a_tri, ta);
    const manifold::Manifold b = as_manifold(b_xyz, nb, b_tri, tb);
    if (a.Status() != manifold::Manifold::Error::NoError ||
        b.Status() != manifold::Manifold::Error::NoError) return 3;
    const manifold::Manifold result = operation == 0 ? a + b :
                                      operation == 1 ? a - b : a ^ b;
    if (result.Status() != manifold::Manifold::Error::NoError) return 3;
    const manifold::MeshGL64 mesh = result.GetMeshGL64();
    if (mesh.numProp < 3 || mesh.triVerts.size() % 3 != 0 ||
        mesh.vertProperties.size() % mesh.numProp != 0 ||
        mesh.vertProperties.size() / mesh.numProp > kMaxResultVertices ||
        mesh.triVerts.size() / 3 > kMaxResultTriangles) return 4;
    auto owned = std::make_unique<ac_manifold_result>();
    owned->xyz.reserve((mesh.vertProperties.size() / mesh.numProp) * 3);
    for (size_t i = 0; i < mesh.vertProperties.size(); i += mesh.numProp) {
      for (size_t j = 0; j < 3; ++j) {
        const double val = mesh.vertProperties[i + j];
        if (!std::isfinite(val)) return 3;
        owned->xyz.push_back(val);
      }
    }
    owned->indices.reserve(mesh.triVerts.size());
    const size_t vertex_count = owned->xyz.size() / 3;
    for (uint64_t i : mesh.triVerts) {
      if (i >= vertex_count || i > std::numeric_limits<uint32_t>::max())
        return 3;
      owned->indices.push_back(static_cast<uint32_t>(i));
    }
    *out = owned.release();
    return 0;
  } catch (const std::bad_alloc&) {
    return 2;
  } catch (...) {
    return 3;
  }
}

size_t ac_manifold_vertex_count(const ac_manifold_result* result) noexcept {
  return result ? result->xyz.size() / 3 : 0;
}
size_t ac_manifold_triangle_count(const ac_manifold_result* result) noexcept {
  return result ? result->indices.size() / 3 : 0;
}
const double* ac_manifold_positions(const ac_manifold_result* result) noexcept {
  return result && !result->xyz.empty() ? result->xyz.data() : nullptr;
}
const uint32_t* ac_manifold_indices(const ac_manifold_result* result) noexcept {
  return result && !result->indices.empty() ? result->indices.data() : nullptr;
}
void ac_manifold_free(ac_manifold_result* result) noexcept {
  delete result;
}

}  // extern "C"
