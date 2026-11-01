//! Independent scanline set operations for solid skins and sparse infill.
//!
//! Every line is clipped against actual contours. A skin is the part of the current layer
//! missing from at least one requested neighbour; the remaining region is sparse infill.

use solidcraft_geom::Vec2;

use crate::GcodeError;

const MAX_SCANLINES: usize = 20_000;
const MAX_WORK: usize = 200_000_000;
const MAX_SEGMENTS: usize = 1_000_000;
const MIN_SEGMENT: f64 = 1e-8;

type Interval = (f64, f64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FillKind {
    Skin,
    Infill,
}

/// Shared by all layers, so many individually small layers cannot evade the work limit.
pub(crate) struct WorkBudget<'a> {
    stop: &'a mut dyn FnMut() -> bool,
    work: usize,
    segments: usize,
}

impl<'a> WorkBudget<'a> {
    pub(crate) fn new(stop: &'a mut dyn FnMut() -> bool) -> Self {
        Self { stop, work: MAX_WORK, segments: MAX_SEGMENTS }
    }

    pub(crate) fn checkpoint(&mut self) -> Result<(), GcodeError> {
        if (self.stop)() { Err(GcodeError::Cancelled) } else { Ok(()) }
    }

    pub(crate) fn spend(&mut self, amount: usize) -> Result<(), GcodeError> {
        self.checkpoint()?;
        self.work = self.work.checked_sub(amount).ok_or(GcodeError::TooComplex("scanline work"))?;
        Ok(())
    }

    pub(crate) fn segment(&mut self) -> Result<(), GcodeError> {
        self.segments = self.segments.checked_sub(1).ok_or(GcodeError::TooComplex("path segments"))?;
        Ok(())
    }
}

pub(crate) struct FillPlanner {
    current: Vec<Vec<Vec2>>,
    neighbours: Vec<Vec<Vec<Vec2>>>,
    full_skin: bool,
    sin: f64,
    cos: f64,
}

impl FillPlanner {
    pub(crate) fn new(
        current: &[Vec<Vec2>],
        neighbours: &[&[Vec<Vec2>]],
        full_skin: bool,
        angle: f64,
        budget: &mut WorkBudget<'_>,
    ) -> Result<Self, GcodeError> {
        let (sin, cos) = angle.sin_cos();
        let current = project(current, sin, cos, budget)?;
        let neighbours =
            if full_skin { Vec::new() } else { neighbours.iter().map(|loops| project(loops, sin, cos, budget)).collect::<Result<_, _>>()? };
        Ok(Self { current, neighbours, full_skin, sin, cos })
    }

    pub(crate) fn paths(&self, spacing: f64, kind: FillKind, budget: &mut WorkBudget<'_>) -> Result<Vec<(Vec2, Vec2)>, GcodeError> {
        if spacing == 0.0
            || self.current.is_empty()
            || (self.full_skin && kind == FillKind::Infill)
            || (!self.full_skin && self.neighbours.is_empty() && kind == FillKind::Skin)
        {
            return Ok(Vec::new());
        }
        if !spacing.is_finite() || spacing <= 1e-3 {
            return Err(GcodeError::BadSetting("fill spacing must exceed 0.001 mm"));
        }
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        for p in self.current.iter().flatten() {
            lo = lo.min(p.y);
            hi = hi.max(p.y);
        }
        if hi <= lo {
            return Ok(Vec::new());
        }
        let rows = ((hi - lo) / spacing).ceil();
        if rows > MAX_SCANLINES as f64 {
            return Err(GcodeError::TooComplex("scanlines in one layer"));
        }
        // The same global lattice is used for skin and 100% infill. Their interval masks
        // are complementary, so a fully solid layer is never extruded twice at their seam.
        let mut first = (lo / spacing).floor() * spacing + spacing * 0.5;
        if first < lo {
            first += spacing;
        }
        let mut paths = Vec::new();
        for row in 0..=rows as usize {
            budget.checkpoint()?;
            let y = first + row as f64 * spacing;
            if y >= hi {
                break;
            }
            let current = intervals_at(&self.current, y, budget)?;
            let covered = if self.full_skin {
                Vec::new()
            } else {
                let mut covered = current.clone();
                for neighbour in &self.neighbours {
                    if covered.is_empty() {
                        break;
                    }
                    let next = intervals_at(neighbour, y, budget)?;
                    budget.spend(covered.len().saturating_add(next.len()))?;
                    covered = intersect(&covered, &next);
                }
                covered
            };
            let spans = match kind {
                FillKind::Skin => subtract(&current, &covered),
                FillKind::Infill => covered,
            };
            for (start, end) in spans {
                if end - start > MIN_SEGMENT {
                    budget.segment()?;
                    let restore = |x: f64| Vec2::new(x * self.cos - y * self.sin, x * self.sin + y * self.cos);
                    paths.push((restore(start), restore(end)));
                }
            }
        }
        Ok(paths)
    }
}

fn project(loops: &[Vec<Vec2>], sin: f64, cos: f64, budget: &mut WorkBudget<'_>) -> Result<Vec<Vec<Vec2>>, GcodeError> {
    let mut result = Vec::with_capacity(loops.len());
    for contour in loops {
        let mut projected = Vec::with_capacity(contour.len());
        for chunk in contour.chunks(256) {
            budget.spend(chunk.len())?;
            projected.extend(chunk.iter().map(|p| Vec2::new(p.x * cos + p.y * sin, -p.x * sin + p.y * cos)));
        }
        result.push(projected);
    }
    Ok(result)
}

fn intervals_at(loops: &[Vec<Vec2>], y: f64, budget: &mut WorkBudget<'_>) -> Result<Vec<Interval>, GcodeError> {
    let mut crossings = Vec::new();
    for contour in loops {
        for (index, (a, b)) in contour.iter().zip(contour.iter().cycle().skip(1)).take(contour.len()).enumerate() {
            if index % 256 == 0 {
                budget.spend((contour.len() - index).min(256))?;
            }
            if (a.y <= y) != (b.y <= y) {
                let x = a.x + (y - a.y) / (b.y - a.y) * (b.x - a.x);
                if !x.is_finite() {
                    return Err(GcodeError::BadSetting("non-finite contour intersection"));
                }
                crossings.push(x);
            }
        }
    }
    crossings.sort_by(f64::total_cmp);
    if crossings.len() % 2 != 0 {
        return Err(GcodeError::BadSetting("unpaired contour intersections"));
    }
    Ok(crossings
        .as_chunks::<2>()
        .0
        .iter()
        .filter_map(|pair| match pair {
            [a, b] if b - a > MIN_SEGMENT => Some((*a, *b)),
            _ => None,
        })
        .collect())
}

fn intersect(a: &[Interval], b: &[Interval]) -> Vec<Interval> {
    let (mut i, mut j) = (0, 0);
    let mut result = Vec::new();
    while let (Some(left), Some(right)) = (a.get(i), b.get(j)) {
        let start = left.0.max(right.0);
        let end = left.1.min(right.1);
        if end - start > MIN_SEGMENT {
            result.push((start, end));
        }
        if left.1 < right.1 {
            i += 1;
        } else {
            j += 1;
        }
    }
    result
}

fn subtract(current: &[Interval], covered: &[Interval]) -> Vec<Interval> {
    let mut result = Vec::new();
    let mut j = 0;
    for (start, end) in current.iter().copied() {
        let mut at = start;
        while let Some(interval) = covered.get(j) {
            if interval.1 <= at {
                j += 1;
            } else {
                break;
            }
        }
        let mut k = j;
        while let Some(interval) = covered.get(k) {
            if interval.0 >= end {
                break;
            }
            if interval.0 > at + MIN_SEGMENT {
                result.push((at, interval.0.min(end)));
            }
            at = at.max(interval.1);
            if at >= end {
                break;
            }
            k += 1;
        }
        if end > at + MIN_SEGMENT {
            result.push((at, end));
        }
        j = k;
    }
    result
}
