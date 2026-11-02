// C ABI shim over Clipper2 (BSL-1.0) for the solidcraft-polyclip-cpp crate.
//
// Coordinates arrive as int64 values already scaled by the Rust side. Every entry point catches
// all C++ exceptions and reports them through `status`, because no exception may cross the C ABI.
// status: 0 ok, 1 Clipper2 or other exception, 2 out of memory, 3 unknown boolean operation.
#include <clipper2/clipper.h>

#include <cstddef>
#include <cstdint>
#include <new>
#include <vector>

using namespace Clipper2Lib;

struct pc2_result {
  std::vector<int64_t> coords;  // x, y pairs of every ring, rings back to back
  std::vector<size_t> lens;     // points per ring
};

namespace {

Path64 to_path(const int64_t* xy, size_t n) {
  Path64 path;
  path.reserve(n);
  for (size_t i = 0; i < n; ++i) {
    path.emplace_back(xy[2 * i], xy[2 * i + 1]);
  }
  return path;
}

pc2_result* to_result(const Paths64& paths) {
  auto* r = new pc2_result();
  for (const auto& path : paths) {
    r->lens.push_back(path.size());
    for (const auto& p : path) {
      r->coords.push_back(p.x);
      r->coords.push_back(p.y);
    }
  }
  return r;
}

}  // namespace

extern "C" {

pc2_result* pc2_offset(const int64_t* xy, size_t n, double delta, double miter_limit, int* status) {
  try {
    ClipperOffset offset(miter_limit);
    offset.AddPath(to_path(xy, n), JoinType::Miter, EndType::Polygon);
    Paths64 solution;
    offset.Execute(delta, solution);
    *status = 0;
    return to_result(solution);
  } catch (const std::bad_alloc&) {
    *status = 2;
  } catch (...) {
    *status = 1;
  }
  return nullptr;
}

pc2_result* pc2_boolean(const int64_t* a, size_t na, const int64_t* b, size_t nb, int op, int* status) {
  ClipType type;
  switch (op) {
    case 0: type = ClipType::Union; break;
    case 1: type = ClipType::Intersection; break;
    case 2: type = ClipType::Difference; break;
    case 3: type = ClipType::Xor; break;
    default:
      *status = 3;
      return nullptr;
  }
  try {
    Clipper64 clipper;
    clipper.AddSubject(Paths64{to_path(a, na)});
    clipper.AddClip(Paths64{to_path(b, nb)});
    Paths64 solution;
    if (!clipper.Execute(type, FillRule::NonZero, solution)) {
      *status = 1;
      return nullptr;
    }
    *status = 0;
    return to_result(solution);
  } catch (const std::bad_alloc&) {
    *status = 2;
  } catch (...) {
    *status = 1;
  }
  return nullptr;
}

size_t pc2_ring_count(const pc2_result* r) { return r->lens.size(); }

size_t pc2_ring_len(const pc2_result* r, size_t i) { return i < r->lens.size() ? r->lens[i] : 0; }

// Null when the result has no points; callers only read it when a ring has points.
const int64_t* pc2_coords(const pc2_result* r) { return r->coords.empty() ? nullptr : r->coords.data(); }

void pc2_free(pc2_result* r) { delete r; }

}  // extern "C"
